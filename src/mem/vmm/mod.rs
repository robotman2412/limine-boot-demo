// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use pagetable::PageTable;

use crate::{
    arch::mmu,
    mem::{PAGE_SIZE, pfndb, vmm::kernel_mm::KernelMemmap},
};

pub mod kernel_mm;
pub mod memmap;
pub mod pagetable;

pub mod prot {
    pub use crate::arch::mmu::prot::*;

    /// Map memory as read-write.
    pub const RW: u32 = R | W;
    /// Map memory as read-execute.
    pub const RX: u32 = R | X;
    /// Map memory as read-write-execute.
    pub const RWX: u32 = R | W | X;
}

// Resolved virtual addresses of the kernel.
unsafe extern "C" {
    static __start_text: [u8; 0];
    static __stop_text: [u8; 0];
    static __start_rodata: [u8; 0];
    static __stop_rodata: [u8; 0];
    static __start_data: [u8; 0];
    static __stop_data: [u8; 0];
}

pub type VPN = usize;

/// Kind of memory access.
pub enum AccessType {
    /// Read-only access.
    Read,
    /// Read-write access.
    Write,
    /// Read-execute access.
    Exec,
}

/// Offset to add to a RAM physical address to get the virtual address within the HHDM.
pub static mut HHDM_OFFSET: usize = 0;
/// The kernel's page table.
static mut KERNEL_MM: Option<KernelMemmap> = None;

/// Get the kernel page table.
pub unsafe fn kernel_mm() -> &'static KernelMemmap {
    unsafe { &*&raw const KERNEL_MM }.as_ref().unwrap()
}

/// Initialize the virtual memory subsystem.
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn init(kernel_vaddr: usize, kernel_paddr: usize) {
    mmu::early_init();
    let kernel_mm = KernelMemmap::new().expect("Failed to create kernel page tables");

    kernel_mm
        .map_pages(
            Some(pfndb::DOMAIN.start + HHDM_OFFSET / PAGE_SIZE),
            pfndb::DOMAIN.start,
            pfndb::DOMAIN.end - pfndb::DOMAIN.start,
            prot::RW,
        )
        .expect("Failed to create higher-half direct map");

    let seg_start = &raw const __start_text as *const _ as usize;
    let seg_end = &raw const __stop_text as *const _ as usize;
    kernel_mm
        .map(
            Some(seg_start),
            seg_start - kernel_vaddr + kernel_paddr,
            seg_end - seg_start,
            prot::RX,
        )
        .expect("Failed to map kernel RX");

    let seg_start = &raw const __start_rodata as *const _ as usize;
    let seg_end = &raw const __stop_rodata as *const _ as usize;
    kernel_mm
        .map(
            Some(seg_start),
            seg_start - kernel_vaddr + kernel_paddr,
            seg_end - seg_start,
            prot::R,
        )
        .expect("Failed to map kernel R");

    let seg_start = &raw const __start_data as *const _ as usize;
    let seg_end = &raw const __stop_data as *const _ as usize;
    kernel_mm
        .map(
            Some(seg_start),
            seg_start - kernel_vaddr + kernel_paddr,
            seg_end - seg_start,
            prot::RW,
        )
        .expect("Failed to map kernel RW");

    mmu::init(kernel_mm.pmap.root_ppn().into());

    KERNEL_MM = Some(kernel_mm);
}
