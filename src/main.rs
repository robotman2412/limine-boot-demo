// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]
#![feature(formatting_options)]

use core::{
    fmt::{Display, Formatter, FormattingOptions},
    panic::PanicInfo,
    ptr::null_mut,
};

use chrono::DateTime;
use flantermbindings::flanterm::{flanterm_context, flanterm_fb_init};
use limine_boot::{BaseRevision, request::*};

extern crate core;

// Implements the C runtime that Rust depends on.
pub mod crt;

pub static BASE_REVISION: BaseRevision = BaseRevision::new();
pub static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
pub static MEMMAP: MemmapRequest = MemmapRequest::new();
pub static BOOTLOADER: BootloaderInfoRequest = BootloaderInfoRequest::new();
pub static FIRMWARE: FirmwareTypeRequest = FirmwareTypeRequest::new();
pub static DATE: DateAtBootRequest = DateAtBootRequest::new();
pub static BOOT_TIME: BootloaderPerformanceRequest = BootloaderPerformanceRequest::new();
pub static HHDM: HhdmRequest = HhdmRequest::new();
pub static EXEC_ADDR: ExecutableAddressRequest = ExecutableAddressRequest::new();
pub static EXEC_FILE: ExecutableFileRequest = ExecutableFileRequest::new();
pub static EXEC_CMDLINE: ExecutableCmdlineRequest = ExecutableCmdlineRequest::new();
pub static DTB: DtbRequest = DtbRequest::new();
pub static RSDP: RsdpRequest = RsdpRequest::new();
pub static MP: MpRequest = MpRequest::new(0);
#[cfg(target_arch = "riscv64")]
pub static BSP_HARTID: BspHartidRequest = BspHartidRequest::new();
pub static MODULES: ModulesRequest = ModulesRequest::new();
#[cfg(target_arch = "x86_64")]
pub static KEEP_IOMMU: KeepIommuRequest = KeepIommuRequest::new();
pub static STACK: StackSizeRequest = StackSizeRequest::new(65536);
pub static PAGING: PagingModeRequest = PagingModeRequest::PREFER_MAXIMUM;
pub static ENTRY: EntryPointRequest = EntryPointRequest::new(_start);
pub static SMBIOS: SmbiosRequest = SmbiosRequest::new();
pub static EFI: EfiRequest = EfiRequest::new();
pub static EFI_MEMMAP: EfiMemmapRequest = EfiMemmapRequest::new();

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

    if let Some(resp) = MEMMAP.response() {
        let spaces = "\x1b[80C";
        write!("{}Memory map:\n", spaces);
        for &ent in resp.entries() {
            use limine_boot::memmap::*;
            write!(
                "{}{:x}-{:x} {}\n",
                spaces,
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
        write!("\x1b[1;1H");
    }

    fn honored<T: 'static, U>(name: &str, request: &Request<T, U>) {
        write!(
            "{} request honored: {}\n",
            name,
            if request.response().is_some() {
                "yes"
            } else {
                "no"
            }
        );
    }
    honored("Entrypoint", &ENTRY);
    honored("Stack size", &STACK);
    #[cfg(target_arch = "x86_64")]
    honored("Keep I/O MMU", &KEEP_IOMMU);

    if let Some(resp) = BOOTLOADER.response() {
        write!("Bootloader name: {}\n", resp.name());
        write!("Bootloader version: {}\n", resp.version());
    }
    if let Some(resp) = FIRMWARE.response() {
        use limine_boot::firmware::*;
        write!(
            "Firmware type: {}\n",
            match resp.firmware_type {
                FIRMWARE_TYPE_X86BIOS => "BIOS",
                FIRMWARE_TYPE_EFI32 => "EFI (32-bit)",
                FIRMWARE_TYPE_EFI64 => "EFI (64-bit)",
                FIRMWARE_TYPE_SBI => "SBI",
                _ => "?",
            }
        );
    }
    if let Some(resp) = EFI.response() {
        write!("EFI system table: 0x{:x}\n", resp.address as usize);
    }
    if let Some(resp) = EFI_MEMMAP.response() {
        write!("EFI memory map: 0x{:x}\n", resp.memmap().as_ptr() as usize);
    }
    if let Some(resp) = SMBIOS.response() {
        if !resp.entry_32.is_null() {
            write!("SMBIOS entry (32-bit): 0x{:x}\n", resp.entry_32 as usize);
        }
        if !resp.entry_64.is_null() {
            write!("SMBIOS entry (64-bit): 0x{:x}\n", resp.entry_64 as usize);
        }
    }
    if let Some(resp) = DATE.response() {
        let unix_seconds = resp.timestamp;
        let date = DateTime::from_timestamp(unix_seconds, 0)
            .unwrap()
            .naive_utc();
        write!("Date at boot: {}\n", date);
    }
    if let Some(resp) = BOOT_TIME.response() {
        write!(
            "Boot time: reset {}.{:06}, init {}.{:06}, exec {}.{:06}\n",
            resp.reset_usec / 1000000,
            resp.reset_usec % 1000000,
            resp.init_usec / 1000000,
            resp.init_usec % 1000000,
            resp.exec_usec / 1000000,
            resp.exec_usec % 1000000
        );
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
    if let Some(resp) = EXEC_FILE.response() {
        let file = resp.executable_file();
        write!(
            "Executable file: {:x}-{:x} {} {}\n",
            file.data().as_ptr() as usize,
            file.data().as_ptr() as usize + file.data().len() - 1,
            file.path(),
            file.cmdline(),
        );
    }
    if let Some(resp) = EXEC_CMDLINE.response() {
        write!("Command-line: {}\n", resp.cmdline());
    }
    if let Some(resp) = DTB.response() {
        write!("DTB address: 0x{:x}\n", resp.dtb_ptr as usize);
    }
    if let Some(resp) = RSDP.response() {
        write!("RSDP address: 0x{:x}\n", resp.address as usize);
    }
    if let Some(resp) = MP.response() {
        write!(
            "Multiprocessing supported, core count: {}\n",
            resp.cpus().len()
        );
    }
    #[cfg(target_arch = "riscv64")]
    if let Some(resp) = BSP_HARTID.response() {
        write!("BSP hartid: 0x{:x}\n", resp.bsp_hartid);
    }
    if let Some(resp) = MODULES.response() {
        let modules = resp.modules();
        write!(
            "{} module{} loaded{}\n",
            modules.len(),
            if modules.len() == 1 { "" } else { "s" },
            if modules.len() == 0 { "" } else { ":" }
        );
        for module in modules {
            write!(
                "{:x}-{:x} {} {}\n",
                module.data().as_ptr() as usize,
                module.data().as_ptr() as usize + module.data().len() - 1,
                module.path(),
                module.cmdline(),
            );
        }
    }

    loop {}
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}
