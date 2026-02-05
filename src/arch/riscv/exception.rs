// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::arch::global_asm;

use crate::misc::panic;

use super::regs::IrqFrame;

global_asm!(include_str!("exception.S"));

unsafe extern "C" {
    pub(super) fn riscv_vector_table();
}

#[unsafe(no_mangle)]
unsafe extern "C" fn riscv_exception_handler(frame: &mut IrqFrame) {
    panic::unhandled_exception(frame);
}
