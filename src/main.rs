// Note: In Rust 1.82.0-nightly and before, the `uefi_std` feature is
// required for accessing `std::os::uefi::env::*`. The other default
// functionality doesn't need a nightly toolchain (with Rust 1.80 and later),
// but with that limited functionality you - currently - also can't integrate
// the `uefi` crate.
#![feature(uefi_std)]

use std::convert::Infallible;
use std::time::Duration;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Bgr888;
use embedded_graphics::prelude::*;
use uefi::boot::ScopedProtocol;
use uefi::proto::console::gop::{BltPixel, GraphicsOutput};
use uefi::{
    Handle, Status, println,
    proto::console::text::{Input, Output},
    runtime::ResetType,
};

/// Performs the necessary setup code for the `uefi` crate.
fn setup_uefi_crate() {
    let st = std::os::uefi::env::system_table();
    let ih = std::os::uefi::env::image_handle();

    // Mandatory setup code for `uefi` crate.
    unsafe {
        uefi::table::set_system_table(st.as_ptr().cast());

        let ih = Handle::from_ptr(ih.as_ptr().cast()).unwrap();
        uefi::boot::set_image_handle(ih);
    }
}

struct UefiGop(ScopedProtocol<GraphicsOutput>);

impl OriginDimensions for UefiGop {
    fn size(&self) -> embedded_graphics::prelude::Size {
        let res = self.0.current_mode_info().resolution();
        embedded_graphics::prelude::Size {
            width: res.0 as u32,
            height: res.1 as u32,
        }
    }
}

impl embedded_graphics::draw_target::DrawTarget for UefiGop {
    type Color = Bgr888;

    type Error = uefi::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for p in pixels {
            self.0.blt(uefi::proto::console::gop::BltOp::VideoFill {
                color: BltPixel::new(p.1.r(), p.1.g(), p.1.b()),
                dest: (p.0.x as usize, p.0.y as usize),
                dims: (1, 1),
            })?;
        }
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let mut buffer = Vec::new();
        for (i, c) in colors.into_iter().enumerate() {
            buffer[i] = BltPixel::new(c.r(), c.g(), c.b());
        }
        self.0
            .blt(uefi::proto::console::gop::BltOp::BufferToVideo {
                buffer: &buffer,
                src: uefi::proto::console::gop::BltRegion::Full,
                dest: (area.top_left.x as usize, area.top_left.y as usize),
                dims: (area.size.width as usize, area.size.height as usize),
            })?;
        Ok(())
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        c: Self::Color,
    ) -> Result<(), Self::Error> {
        self.0.blt(uefi::proto::console::gop::BltOp::VideoFill {
            color: BltPixel::new(c.r(), c.g(), c.b()),
            dest: (area.top_left.x as usize, area.top_left.y as usize),
            dims: (area.size.width as usize, area.size.height as usize),
        })?;
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill_solid(&self.bounding_box(), color)
    }
}

fn main() {
    setup_uefi_crate();

    let input_keys_handle = uefi::boot::get_handle_for_protocol::<Input>().unwrap();
    let input_keys = uefi::boot::open_protocol_exclusive::<Input>(input_keys_handle).unwrap();

    let text_output_handle = uefi::boot::get_handle_for_protocol::<Output>().unwrap();
    let mut text_output =
        uefi::boot::open_protocol_exclusive::<Output>(text_output_handle).unwrap();

    let gop_handle = uefi::boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();


    let mut target = UefiGop(gop);
    let style = MonoTextStyle::new(&FONT_6X10, Bgr888::WHITE);

    let mut string = String::new();
    let modes = target.0.modes().collect::<Vec<_>>();
    for (i, mode) in modes.iter().enumerate() {
        string += &format!(
            "{}: {}x{} ({:?})\n",
            i,
            mode.info().resolution().0,
            mode.info().resolution().1,
            mode.info().pixel_format()
        );
    }
    let text = embedded_graphics::text::Text::new(&string, Point::new(0, 10), style);
    target.clear(Bgr888::BLACK).unwrap();
    text.draw(&mut target).unwrap();

    let key_event = input_keys.wait_for_key_event().unwrap();
    uefi::boot::wait_for_event(&mut [key_event]).unwrap();
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
