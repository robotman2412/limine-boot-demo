// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

macro_rules! bitfield {
    (
        $name: ident : $bit: literal
    ) => {
        pub const ${concat($name, _BIT)}: u32 = $bit;
        pub const ${concat($name, _MASK)}: usize = 1 << $bit;
    };
    (
        $name: ident : $high: literal .. $low: literal
    ) => {
        pub const ${concat($name, _BIT)}: u32 = $low;
        pub const ${concat($name, _MASK)}: usize = (1 << ($high + 1)) - (1 << $low);
    };
}

pub mod sstatus {
    pub const XS_OFF: usize = 0;
    pub const XS_CLEAN: usize = 1;
    pub const XS_INITIAL: usize = 2;
    pub const XS_DIRTY: usize = 3;

    bitfield!(SIE: 1);
    bitfield!(SPIE: 5);
    bitfield!(UBE: 6);
    bitfield!(SPP: 8);
    bitfield!(VS: 10..9);
    bitfield!(FS: 14..13);
    bitfield!(XS: 16..15);
    bitfield!(SUM: 18);
    bitfield!(MXR: 19);
    bitfield!(UXL: 33..32);
    bitfield!(SD: 63);
}
