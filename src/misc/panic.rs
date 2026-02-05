// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    arch::{irq, regs::IrqFrame},
    logk, writek,
};

static PANICKING: AtomicU32 = AtomicU32::new(0);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    start_panic();
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
    finish_panic();
}

pub fn unhandled_exception(frame: &IrqFrame) -> ! {
    start_panic();
    writek!("**** UNHANDLED EXCEPTION 0x{:x} ****\n", frame.fault_code());
    if let Some(name) = frame.fault_name() {
        writek!("{}\n", name);
    }
    if let Some(vaddr) = frame.is_mem_trap() {
        writek!("Virtual address: 0x{:x}\n", vaddr);
    }
    writek!("{}", frame);

    finish_panic();
}

fn start_panic() {
    if PANICKING.fetch_or(0, Ordering::Relaxed) != 0 {
        unsafe { irq::disable() };
        loop {}
    }
}

fn finish_panic() -> ! {
    writek!("**** KERNEL PANIC ****\n");
    // TODO: System power-off?
    unsafe { irq::disable() };
    loop {}
}

pub fn check_for_panic() {
    if PANICKING.load(Ordering::Relaxed) != 0 {
        unsafe { irq::disable() };
        loop {}
    }
}
