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
use uefi::proto::console::pointer::Pointer;
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

struct UefiGop {
    gop: ScopedProtocol<GraphicsOutput>,
    buffer: Vec<BltPixel>,
    size: (usize, usize),
}
impl UefiGop {
    fn new(gop: ScopedProtocol<GraphicsOutput>) -> Self {
        let size = gop.current_mode_info().resolution();
        let buffer = vec![BltPixel::new(0, 0, 0); size.0 * size.1];
        UefiGop { gop, buffer, size }
    }
    fn flush(&mut self) {
        self.gop
            .blt(uefi::proto::console::gop::BltOp::BufferToVideo {
                buffer: &self.buffer,
                src: uefi::proto::console::gop::BltRegion::Full,
                dest: (0, 0),
                dims: self.size,
            })
            .unwrap();
    }
}

impl OriginDimensions for UefiGop {
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size {
            width: self.size.0 as u32,
            height: self.size.1 as u32,
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
            let ind = p.0.x as usize + p.0.y as usize * self.size.0;
            self.buffer[ind] = BltPixel::new(p.1.r(), p.1.g(), p.1.b());
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
        let start_ind = area.top_left.x as usize + area.top_left.y as usize * self.size.0;
        for (i, c) in colors.into_iter().enumerate() {
            self.buffer[start_ind + i] = BltPixel::new(c.r(), c.g(), c.b());
        }
        Ok(())
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        c: Self::Color,
    ) -> Result<(), Self::Error> {
        let px = BltPixel::new(c.r(), c.g(), c.b());
        let start_ind = area.top_left.x as usize + area.top_left.y as usize * self.size.0;
        let len = area.size.width as usize * area.size.height as usize;
        self.buffer[start_ind..][..len].fill(px);
        Ok(())
    }

    fn clear(&mut self, c: Self::Color) -> Result<(), Self::Error> {
        let px = BltPixel::new(c.r(), c.g(), c.b());
        self.buffer.fill(px);
        Ok(())
    }
}

fn main() {
    setup_uefi_crate();

    let input_keys_handle = uefi::boot::get_handle_for_protocol::<Input>().unwrap();
    let mut input_keys = uefi::boot::open_protocol_exclusive::<Input>(input_keys_handle).unwrap();

    let input_mouse_handle = uefi::boot::get_handle_for_protocol::<Pointer>().unwrap();
    let mut input_mouse =
        uefi::boot::open_protocol_exclusive::<Pointer>(input_mouse_handle).unwrap();

    let gop_handle = uefi::boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let gop = uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let mut target = UefiGop::new(gop);
    let style = MonoTextStyle::new(&FONT_6X10, Bgr888::WHITE);

    let mut string = String::new();
    for i in 0.. {
        string.clear();
        string += &format!("Version 1\n");
        let modes = target.gop.modes().collect::<Vec<_>>();
        for (i, mode) in modes.iter().enumerate() {
            string += &format!(
                "{}: {}x{} ({:?})\n",
                i,
                mode.info().resolution().0,
                mode.info().resolution().1,
                mode.info().pixel_format()
            );
        }
        let mode = input_mouse.mode();
        string += &format!("{mode:#?}\n");
        let state = input_mouse.read_state();
        string += &format!("{state:#?}\n");
        string += &format!("{i}\n");

        let text = embedded_graphics::text::Text::new(&string, Point::new(0, 10), style);
        target.clear(Bgr888::BLACK).unwrap();
        text.draw(&mut target).unwrap();
        target.flush();
        if let Some(_) = input_keys.read_key().unwrap() {
            break;
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    uefi::runtime::reset(ResetType::WARM, Status::SUCCESS, None);
}
