use core::arch::{asm, naked_asm};

use crate::{
    arch::exception,
    init::main,
    misc::log::LOG_WRITE,
    sched::cpulocal::{BSP_CPULOCAL, CpuLocal},
    writek,
};

use super::sbi;

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

fn spinup() {
    unsafe {
        asm!("csrw stvec, {}", in(reg) exception::riscv_vector_table as *const ());
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn early_init() -> ! {
    CpuLocal::set(&raw mut BSP_CPULOCAL);

    if sbi::dbcn::probe() {
        LOG_WRITE = sbi_dbcn_log_write;
    } else {
        LOG_WRITE = sbi_legacy_log_write;
    }
    spinup();

    writek!("\x1b[0m\n\n");
    main();
}
