// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::panic::PanicInfo;

use crate::logk;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        logk!(
            LogLevel::Fatal,
            "{}:{}:{}: {}",
            loc.file(),
            loc.line(),
            loc.column(),
            info.message(),
        );
    } else {
        logk!(LogLevel::Fatal, "{}", info.message());
    }
    loop {}
}
