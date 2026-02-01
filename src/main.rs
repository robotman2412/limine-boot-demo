// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]
#![feature(try_trait_v2)]
#![feature(unsafe_cell_access)]
#![feature(linkage)]
#![feature(macro_metavar_expr_concat)]
#![feature(formatting_options)]

pub mod init;
pub mod mem;
pub mod misc;
pub mod sched;
pub mod sync;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
pub mod arch;
