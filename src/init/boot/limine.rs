// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{ptr::addr_eq, usize};

use limine::{
    memory_map::{Entry, EntryType},
    request::{DeviceTreeBlobRequest, ExecutableAddressRequest, HhdmRequest, MemoryMapRequest},
};

use crate::{
    device::dtb::{self, dtb::Dtb},
    logk,
    mem::{pmm, vmm},
};

#[unsafe(link_section = ".requests")]
static HHDM_REQ: HhdmRequest = HhdmRequest::new();
#[unsafe(link_section = ".requests")]
static MEMMAP_REQ: MemoryMapRequest = MemoryMapRequest::new();
#[unsafe(link_section = ".requests")]
static DTB_REQ: DeviceTreeBlobRequest = DeviceTreeBlobRequest::new();
#[unsafe(link_section = ".requests")]
static ADDR_REQ: ExecutableAddressRequest = ExecutableAddressRequest::new();

pub unsafe fn early_init() {
    let memmap_resp = MEMMAP_REQ
        .get_response()
        .expect("Missing Limine memory map response");
    let hhdm_resp = HHDM_REQ
        .get_response()
        .expect("Missing Limine HHDM response");
    let addr_resp = ADDR_REQ
        .get_response()
        .expect("Missing Limine kernel physical address respone");

    // Initialize physical memory management.
    let mut lowest_paddr = usize::MAX;
    let mut highest_paddr = 0usize;
    let mut largest_region: Option<&Entry> = None;
    for region in memmap_resp.entries() {
        if region.entry_type == EntryType::USABLE
            || region.entry_type == EntryType::BOOTLOADER_RECLAIMABLE
        {
            lowest_paddr = lowest_paddr.min(region.base as usize);
            highest_paddr = highest_paddr.max((region.base + region.length) as usize);
        }

        if region.entry_type == EntryType::USABLE
            && largest_region
                .map(|x| x.length < region.length)
                .unwrap_or(true)
        {
            largest_region = Some(region);
        }

        logk!(
            LogLevel::Info,
            "{:016x}-{:016x} {}",
            region.base,
            region.base + region.length - 1,
            match region.entry_type {
                EntryType::USABLE => "Usable",
                EntryType::RESERVED => "Reserved",
                EntryType::ACPI_RECLAIMABLE => "ACPI reclaimable",
                EntryType::ACPI_NVS => "ACPI non-volatile storage",
                EntryType::BAD_MEMORY => "Bad memory",
                EntryType::BOOTLOADER_RECLAIMABLE => "Bootloader reclaimable",
                EntryType::EXECUTABLE_AND_MODULES => "Kernel/modules",
                EntryType::FRAMEBUFFER => "Framebuffer",
                _ => "?",
            }
        );
    }
    let largest_region = largest_region.expect("No usable memory");

    unsafe {
        vmm::HHDM_OFFSET = hhdm_resp.offset() as usize;

        pmm::init(
            largest_region.base as usize..(largest_region.base + largest_region.length) as usize,
            lowest_paddr..highest_paddr,
        );

        let mut reclaimable = 0;
        for &region in memmap_resp.entries() {
            if addr_eq(largest_region, region) {
                continue;
            }
            if region.entry_type == EntryType::USABLE {
                pmm::mark_usable(region.base as usize..(region.base + region.length) as usize);
            } else if region.entry_type == EntryType::BOOTLOADER_RECLAIMABLE {
                reclaimable += region.length as usize;
            }
        }
        logk!(
            LogLevel::Info,
            "PMM initialized, {} MiB total of which {} MiB reclaimable",
            pmm::total_memory() / 1024 / 1024,
            reclaimable / 1024 / 1024
        );

        vmm::init(
            addr_resp.virtual_base() as usize,
            addr_resp.physical_base() as usize,
        );
        logk!(LogLevel::Info, "VMM initialized");
    }

    // Read the DTB.
    if let Some(fdt) = DTB_REQ.get_response() {
        unsafe {
            let dtb = Dtb::parse(fdt.dtb_ptr() as _);
            dtb::DTB = Some(dtb);
        }
    }
}
