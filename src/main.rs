// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

#![no_std]
#![no_main]
#![feature(try_trait_v2)]
#![feature(unsafe_cell_access)]
#![feature(linkage)]
#![feature(macro_metavar_expr_concat)]
#![feature(formatting_options)]
#![feature(allocator_api)]
#![feature(try_blocks)]
#![feature(const_trait_impl)]
#![feature(const_destruct)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(ptr_metadata)]
#![feature(generic_atomic)]
#![feature(linked_list_cursors)]

extern crate alloc;
extern crate core;

pub mod device;
pub mod init;
pub mod irq;
pub mod mem;
pub mod misc;
pub mod sched;
pub mod sync;

#[cfg(target_arch = "riscv64")]
#[path = "arch/riscv/mod.rs"]
pub mod arch;
