// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use pagetable::PageTable;

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
static mut KERNEL_MM: Option<PageTable> = None;

/// Get the kernel page table.
pub unsafe fn kernel_mm() -> &'static PageTable {
    unsafe { &*&raw const KERNEL_MM }.as_ref().unwrap()
}

/// Initialize the virtual memory subsystem.
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn init() {}
