// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::arch::cpulocal::ArchCpuLocal;

/// CPU-local data.
#[repr(C)]
pub struct CpuLocal {
    /// Architecture-specific CPU-local data.
    /// Must be the first member of this struct.
    pub arch: ArchCpuLocal,
    /// Software-assigned ID (so that they're contiguous).
    pub id: u32,
}

/// A preallocated struct for the BSP's CPU-local data.
pub static mut BSP_CPULOCAL: CpuLocal = CpuLocal {
    arch: ArchCpuLocal::new(0),
    id: 0,
};
