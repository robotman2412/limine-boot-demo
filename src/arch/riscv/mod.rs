// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

pub mod cpulocal;
pub mod csr;
pub mod exception;
pub mod irq;
pub mod regs;
pub mod sbi;
pub mod spinup;
pub mod timer;

pub const NAME: &str = "riscv64";
pub const NAME_BLANK: &str = "=======";
