// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::{init::main, misc::log::LOG_WRITE};
use core::arch::naked_asm;

pub mod csr;
pub mod irq;
pub mod sbi;
pub mod timer;

pub type PhysCpuID = usize;

pub const NAME: &str = "riscv64";
pub const NAME_BLANK: &str = "=======";

/// The kernel entrypoint.
#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        ".option push",
        ".option norelax",
        "la gp, __global_pointer$",
        ".option pop",
        "j {}",
        sym early_init
    );
}

fn sbi_legacy_log_write(msg: &str) {
    for &c in msg.as_bytes() {
        let _ = sbi::legacy::console_putchar(c);
    }
}

fn sbi_dbcn_log_write(msg: &str) {
    for &c in msg.as_bytes() {
        let _ = sbi::dbcn::write_byte(c);
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn early_init() -> ! {
    if sbi::dbcn::probe() {
        LOG_WRITE = sbi_dbcn_log_write;
    } else {
        LOG_WRITE = sbi_legacy_log_write;
    }
    main();
}
