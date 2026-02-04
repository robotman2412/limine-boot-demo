// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::device::dtb::dtb::Dtb;

pub mod dtb;

pub static mut DTB: Option<Dtb> = None;
