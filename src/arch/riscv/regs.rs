// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::fmt::Display;

/// Special registers state for interrupt frames.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct SpRegfile {
    pub sstatus: usize,
    pub scause: isize,
    pub stval: usize,
    pub fake_fp: *const (),
}

impl Display for SpRegfile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "  SSTATUS  0x{:x}\n  SCAUSE   0x{:x}\n  STVAL    0x{:x}\n",
            self.sstatus, self.scause, self.stval
        ))
    }
}

/// The general-purpose registers for interrupt frames.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct GpRegfile {
    pub pc: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

impl Display for GpRegfile {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "  PC  0x{:016x}  RA  0x{:016x}  SP  0x{:016x}  GP  0x{:016x}\n  TP  0x{:016x}  T0  0x{:016x}  T1  0x{:016x}  T2  0x{:016x}\n  S0  0x{:016x}  S1  0x{:016x}  A0  0x{:016x}  A1  0x{:016x}\n  A2  0x{:016x}  A3  0x{:016x}  A4  0x{:016x}  A5  0x{:016x}\n  A6  0x{:016x}  A7  0x{:016x}  S2  0x{:016x}  S3  0x{:016x}\n  S4  0x{:016x}  S5  0x{:016x}  S6  0x{:016x}  S7  0x{:016x}\n  S8  0x{:016x}  S9  0x{:016x}  S10 0x{:016x}  S11 0x{:016x}\n  T3  0x{:016x}  T4  0x{:016x}  T5  0x{:016x}  T6  0x{:016x}\n",
            self.pc,
            self.ra,
            self.sp,
            self.gp,
            self.tp,
            self.t0,
            self.t1,
            self.t2,
            self.s0,
            self.s1,
            self.a0,
            self.a1,
            self.a2,
            self.a3,
            self.a4,
            self.a5,
            self.a6,
            self.a7,
            self.s2,
            self.s3,
            self.s4,
            self.s5,
            self.s6,
            self.s7,
            self.s8,
            self.s9,
            self.s10,
            self.s11,
            self.t3,
            self.t4,
            self.t5,
            self.t6,
        ))?;

        Ok(())
    }
}

/// Interrupt frame.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IrqFrame {
    pub(super) sregs: SpRegfile,
    pub(super) regs: GpRegfile,
}

impl IrqFrame {
    pub fn set_retval(&mut self, val: usize) {
        self.regs.a0 = val;
    }

    pub fn set_big_retval(&mut self, val: [usize; 2]) {
        self.regs.a0 = val[0];
        self.regs.a1 = val[1];
    }

    pub fn set_pc(&mut self, val: usize) {
        self.regs.pc = val;
    }

    pub fn set_stack(&mut self, val: usize) {
        self.regs.sp = val;
    }

    pub fn get_pc(&self) -> usize {
        self.regs.pc
    }

    pub fn get_stack(&self) -> usize {
        self.regs.sp
    }

    pub const fn fault_code(&self) -> isize {
        self.sregs.scause
    }

    pub const fn fault_name(&self) -> Option<&'static str> {
        match self.sregs.scause {
            0 => Some("Instruction address misaligned"),
            1 => Some("Instruction access fault"),
            2 => Some("Illegal instruction"),
            3 => Some("Breakpoint"),
            4 => Some("Load address misaligned"),
            5 => Some("Load access fault"),
            6 => Some("Store address misaligned"),
            7 => Some("Store access fault"),
            8 => Some("E-call from U-mode"),
            9 => Some("E-call from S-mode"),
            12 => Some("Instruction page fault"),
            13 => Some("Load page fault"),
            15 => Some("Store page fault"),
            18 => Some("Software check"),
            19 => Some("Hardware error"),
            _ => None,
        }
    }

    pub const fn is_mem_trap(&self) -> Option<usize> {
        match self.sregs.scause {
            0 | 1 | 4 | 5 | 6 | 7 | 12 | 13 | 15 => Some(self.sregs.stval),
            2 | 3 => Some(self.regs.pc),
            _ => None,
        }
    }

    pub const fn is_kernel_mode(&self) -> bool {
        self.sregs.sstatus & 0x100 != 0
    }

    pub(crate) fn fault_fake_frame_ptr(&self) -> *const () {
        self.sregs.fake_fp
    }
}

impl Display for IrqFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}{}", &self.regs, &self.sregs)
    }
}

/// The floating-point register state.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FloatRegfile {
    pub ft0: u64,
    pub ft1: u64,
    pub ft2: u64,
    pub ft3: u64,
    pub ft4: u64,
    pub ft5: u64,
    pub ft6: u64,
    pub ft7: u64,
    pub fs0: u64,
    pub fs1: u64,
    pub fa0: u64,
    pub fa1: u64,
    pub fa2: u64,
    pub fa3: u64,
    pub fa4: u64,
    pub fa5: u64,
    pub fa6: u64,
    pub fa7: u64,
    pub fs2: u64,
    pub fs3: u64,
    pub fs4: u64,
    pub fs5: u64,
    pub fs6: u64,
    pub fs7: u64,
    pub fs8: u64,
    pub fs9: u64,
    pub fs10: u64,
    pub fs11: u64,
    pub ft8: u64,
    pub ft9: u64,
    pub ft10: u64,
    pub ft11: u64,
}
