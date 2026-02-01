// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::arch::asm;

use crate::arch::csr;

/// Check whether interrupts are enabled.
pub fn is_enabled() -> bool {
    let mut mask: usize;
    unsafe {
        asm!("csrr {tmp}, sstatus", tmp = out(reg) mask);
    }
    mask & csr::sstatus::SIE_MASK != 0
}

/// Disable interrupts if some condition holds.
pub unsafe fn disable_if(cond: bool) -> bool {
    let mut mask: usize = (cond as usize) << csr::sstatus::SIE_BIT;
    unsafe {
        asm!("csrrc {tmp}, sstatus, {tmp}", tmp = inout(reg) mask);
    }
    mask & csr::sstatus::SIE_MASK != 0
}

/// Enable interrupts if some condition holds.
pub unsafe fn enable_if(cond: bool) {
    let mask: usize = (cond as usize) << csr::sstatus::SIE_BIT;
    unsafe {
        asm!("csrs sstatus, {tmp}", tmp = in(reg) mask);
    }
}

/// Disable interrupts.
pub unsafe fn disable() -> bool {
    unsafe { disable_if(true) }
}

/// Enable interrupts.
pub unsafe fn enable() {
    unsafe { enable_if(true) }
}
