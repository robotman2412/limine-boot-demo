// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

pub mod crt;
pub mod list;
pub mod log;
pub mod panic;

#[macro_export]
macro_rules! array_from_fn {
    ($($i: ident)? $block: block) => {{
        use core::mem::MaybeUninit;
        let mut arr = [const{MaybeUninit::uninit()}; _];
        let mut i = 0;
        while i < arr.len() {
            $(let $i = i;)?
            arr[i] = MaybeUninit::new($block);
            i += 1;
        }
        unsafe { MaybeUninit::array_assume_init(arr) }
    }};
}
