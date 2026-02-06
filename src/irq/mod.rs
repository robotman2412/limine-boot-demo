// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::arch;

/// Guard that disable interrupts momentarily.
pub struct IrqGuard {
    was_enabled: bool,
}

impl IrqGuard {
    pub fn new() -> Self {
        IrqGuard {
            was_enabled: unsafe { arch::irq::disable() },
        }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        unsafe { arch::irq::enable_if(self.was_enabled) };
    }
}
