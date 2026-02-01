// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::{arch, logk};

/// The kernel entrypoint proper.
pub extern "C" fn main() -> ! {
    logk!(LogLevel::Info, "=========================");
    logk!(LogLevel::Info, "Positron {} starting", arch::NAME);
    logk!(LogLevel::Info, "=========================");

    todo!()
}
