// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{arch::asm, ptr::null_mut};

use crate::sched::cpulocal::CpuLocal;

/// Architecture-specific CPU-local data.
#[repr(C)]
pub struct ArchCpuLocal {
    /// Stack pointer to use for interrupts that return from user mode.
    pub irq_stack: *mut (),
    /// Scratch space used by the trap and interrupt handlers.
    pub scratch: [usize; 3],
    /// This HART's ID (`mhartid` CSR).
    pub hartid: usize,
}

impl ArchCpuLocal {
    pub const fn new(hartid: usize) -> Self {
        ArchCpuLocal {
            irq_stack: null_mut(),
            scratch: [0; _],
            hartid,
        }
    }
}

impl CpuLocal {
    /// Get the CPU-local pointer.
    #[inline(always)]
    pub fn get() -> *mut CpuLocal {
        unsafe {
            let ptr: *mut Self;
            asm!("csrr {ptr}, sscratch", ptr=out(reg)ptr, options(pure,nomem));
            ptr
        }
    }

    /// Set the CPU-local pointer.
    #[inline(always)]
    pub unsafe fn set(ptr: *mut Self) {
        unsafe {
            asm!("csrw sscratch, {ptr}", ptr=in(reg)ptr, options(nomem));
        }
    }
}

impl ArchCpuLocal {
    /// Set the interrupt stack pointer.
    pub fn set_irq_stack(&mut self, sp: *mut ()) {
        self.irq_stack = sp;
    }
}
