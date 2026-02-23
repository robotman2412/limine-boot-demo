// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use limine_boot::{BaseRevision, framebuffer::Framebuffer, request::FramebufferRequest};

extern crate core;

// Implements the C runtime that Rust depends on.
pub mod crt;

pub static BASE_REVISION: BaseRevision = BaseRevision::new();
pub static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

pub fn fill_the_framebuffer(fb: &Framebuffer) {
    unsafe { fb.as_slice_mut() }.fill(0xff);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported(), "Base revision not supported");

    if let Some(resp) = FRAMEBUFFER.response() {
        for &fb in resp.framebuffers() {
            fill_the_framebuffer(fb);
        }
    }

    loop {}
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}
