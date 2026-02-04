// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::fmt::{Display, Formatter, FormattingOptions, Write};

pub static mut LOG_WRITE: fn(&str) = dummy_log_write;

pub fn dummy_log_write(_: &str) {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

pub fn writek(msg: &dyn Display) {
    struct Writer;
    impl Write for Writer {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            unsafe { LOG_WRITE(s) }
            Ok(())
        }
    }
    let mut writer = Writer;
    let mut formatter = Formatter::new(&mut writer, FormattingOptions::new());
    let _ = msg.fmt(&mut formatter);
}

pub fn logk(level: LogLevel, msg: &dyn Display) {
    #[rustfmt::skip]
    let prefix = match level {
        LogLevel::Debug   => "\x1b[34mDEBUG ",
        LogLevel::Info    => "\x1b[32mINFO  ",
        LogLevel::Warning => "\x1b[33mWARN  ",
        LogLevel::Error   => "\x1b[31mERROR ",
        LogLevel::Fatal   => "\x1b[31mFATAL ",
    };
    unsafe { LOG_WRITE(prefix) };
    writek(msg);
    unsafe { LOG_WRITE("\x1b[0m\n") };
}

#[macro_export]
macro_rules! writek {
    ($fmt: expr $(, $($arg: expr),+ $(,)?)?) => {
        {
            use crate::misc::log::*;
            writek(&format_args!($fmt $($(, $arg)+)*))
        }
    };
}

#[macro_export]
macro_rules! logk {
    ($level: expr, $fmt: expr $(, $($arg: expr),+ $(,)?)?) => {
        {
            use crate::misc::log::*;
            logk($level, &format_args!($fmt $($(, $arg)+)*))
        }
    };
}
