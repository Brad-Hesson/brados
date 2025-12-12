// Note: In Rust 1.82.0-nightly and before, the `uefi_std` feature is
// required for accessing `std::os::uefi::env::*`. The other default
// functionality doesn't need a nightly toolchain (with Rust 1.80 and later),
// but with that limited functionality you - currently - also can't integrate
// the `uefi` crate.
#![feature(uefi_std)]

use std::time::Duration;
use uefi::proto::console::gop::{BltPixel, GraphicsOutput};
use uefi::proto::console::pointer::Pointer;
use uefi::proto::console::text::{Input, Output};
use uefi::runtime::ResetType;
use uefi::{Handle, Status};

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

fn main() {
    setup_uefi_crate();

    let gop_handle = uefi::boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = uefi::boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let input_keys_handle = uefi::boot::get_handle_for_protocol::<Input>().unwrap();
    let input_keys = uefi::boot::open_protocol_exclusive::<Input>(input_keys_handle).unwrap();

    let (width, height) = gop.current_mode_info().resolution();
    let mut buffer = vec![BltPixel::new(0, 0, 0); width * height];

    for y in 0..height {
        let r = ((y as f32) / ((height - 1) as f32)) * 255.0;
        for x in 0..width {
            let g = ((x as f32) / ((width - 1) as f32)) * 255.0;
            let pixel = &mut buffer[x + y * width];
            pixel.red = r as u8;
            pixel.green = g as u8;
            pixel.blue = 255;
        }
    }

    gop.blt(uefi::proto::console::gop::BltOp::BufferToVideo {
        buffer: &buffer,
        src: uefi::proto::console::gop::BltRegion::Full,
        dest: (0, 0),
        dims: (width, height),
    })
    .unwrap();

    let key_event = input_keys.wait_for_key_event().unwrap();
    uefi::boot::wait_for_event(&mut [key_event]).unwrap();
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
