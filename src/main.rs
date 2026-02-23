// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]
#![feature(formatting_options)]

use core::{
    fmt::{Display, Formatter, FormattingOptions, Write},
    panic::PanicInfo,
    ptr::null_mut,
};

use flantermbindings::flanterm::{flanterm_context, flanterm_fb_init};
use limine_boot::{
    BaseRevision,
    framebuffer::Framebuffer,
    request::{
        BootloaderInfoRequest, DtbRequest, ExecutableAddressRequest, FramebufferRequest,
        HhdmRequest, MemmapRequest, RsdpRequest,
    },
};

extern crate core;

// Implements the C runtime that Rust depends on.
pub mod crt;

pub static BASE_REVISION: BaseRevision = BaseRevision::new();
pub static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
pub static BOOTLOADER: BootloaderInfoRequest = BootloaderInfoRequest::new();
pub static MEMMAP: MemmapRequest = MemmapRequest::new();
pub static HHDM: HhdmRequest = HhdmRequest::new();
pub static EXEC_ADDR: ExecutableAddressRequest = ExecutableAddressRequest::new();
pub static DTB: DtbRequest = DtbRequest::new();
pub static RSDP: RsdpRequest = RsdpRequest::new();

pub static mut FLANTERM_CTX: *mut flanterm_context = null_mut();

pub fn write(msg: &dyn Display) {
    unsafe {
        if !FLANTERM_CTX.is_null() {
            let mut fmt = Formatter::new(&mut *FLANTERM_CTX, FormattingOptions::new());
            let _ = msg.fmt(&mut fmt);
        }
    }
}

macro_rules! write {
    ($($args: expr),+ $(,)?) => {
        write(&format_args!($($args),+));
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    assert!(BASE_REVISION.is_supported(), "Base revision not supported");

    if let Some(resp) = FRAMEBUFFER.response()
        && let Some(fb) = resp.framebuffers().first()
    {
        unsafe {
            FLANTERM_CTX = flanterm_fb_init(
                None,
                None,
                fb.address() as *mut u32,
                fb.width as _,
                fb.height as _,
                fb.pitch as _,
                fb.red_mask_size as _,
                fb.red_mask_shift as _,
                fb.green_mask_size as _,
                fb.green_mask_shift as _,
                fb.blue_mask_size as _,
                fb.blue_mask_shift as _,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                0,
                0,
                0,
                0,
                0,
                0,
            );
        }
    }

    if let Some(resp) = BOOTLOADER.response() {
        write!("Bootloader name: {}\n", resp.name());
        write!("Bootloader version: {}\n", resp.version());
    }
    if let Some(resp) = MEMMAP.response() {
        write!("Memory map:\n");
        for &ent in resp.entries() {
            use limine_boot::memmap::*;
            write!(
                "{:x}-{:x} {}\n",
                ent.base,
                ent.base + ent.length - 1,
                match ent.type_ {
                    MEMMAP_USABLE => "Usable",
                    MEMMAP_RESERVED => "Reserved",
                    MEMMAP_ACPI_RECLAIMABLE => "ACPI reclaimable",
                    MEMMAP_ACPI_NVS => "ACPI NVS",
                    MEMMAP_BAD_MEMORY => "Bad memory",
                    MEMMAP_BOOTLOADER_RECLAIMABLE => "Bootloader reclaimable",
                    MEMMAP_EXECUTABLE_AND_MODULES => "Executable and modules",
                    MEMMAP_FRAMEBUFFER => "Framebuffer",
                    MEMMAP_MAPPED_RESERVED => "Mapped reserved",
                    _ => "?",
                }
            );
        }
    }
    if let Some(resp) = HHDM.response() {
        write!("HHDM offset: 0x{:x}\n", resp.offset);
    }
    if let Some(resp) = EXEC_ADDR.response() {
        write!(
            "Executable address: vaddr 0x{:x}, paddr 0x{:x}\n",
            resp.virtual_base,
            resp.physical_base
        );
    }
    if let Some(resp) = DTB.response() {
        write!("DTB address: 0x{:x}\n", resp.dtb_ptr as usize);
    }
    if let Some(resp) = RSDP.response() {
        write!("RSDP address: 0x{:x}\n", resp.address as usize);
    }

    loop {}
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}
