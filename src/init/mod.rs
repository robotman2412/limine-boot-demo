// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use crate::{arch, logk};

#[path = "boot/limine.rs"]
pub mod boot;

/// The kernel entrypoint proper.
pub unsafe extern "C" fn main() -> ! {
    logk!(LogLevel::Info, "========={}=========", arch::NAME_BLANK);
    logk!(LogLevel::Info, "Positron {} starting", arch::NAME);
    logk!(LogLevel::Info, "========={}=========", arch::NAME_BLANK);

    // PMM, kernel heap, parse FDT into device tree.
    unsafe {
        boot::early_init();
    }

    todo!()
}
