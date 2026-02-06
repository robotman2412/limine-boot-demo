// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    alloc::{GlobalAlloc, Layout},
    num::NonZero,
    ptr::null_mut,
};

use crate::mem::{
    pmm::pfndb::{self, PageUsage, page_meta},
    vmm::HHDM_OFFSET,
};

pub mod pmm;
pub mod slabs;
pub mod vmm;

pub const PAGE_SIZE: usize = 4096;

pub struct Heap;

#[global_allocator]
pub static mut HEAP: Heap = Heap;

unsafe impl GlobalAlloc for Heap {
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let min_size = layout.pad_to_align().size();

        let mem = if min_size <= slabs::BUCKET_SIZE_MAX {
            slabs::allocate(min_size)
        } else {
            try {
                let mut order = min_size.div_ceil(PAGE_SIZE).ilog2();
                if PAGE_SIZE << order < min_size {
                    order += 1;
                }
                let mem = pmm::RawMemory::new(order as u8, PageUsage::KernelHeap)?;
                let ptr = mem.hhdm_ptr();
                core::mem::forget(mem);
                ptr
            }
        };

        mem.unwrap_or(null_mut()) as *mut u8
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        let ppn = (ptr as usize - HHDM_OFFSET) / PAGE_SIZE;
        assert!(
            pfndb::DOMAIN.start <= ppn && ppn < pfndb::DOMAIN.end,
            "Free of unaccounted memory 0x{:x}",
            ptr as usize
        );
        let meta = page_meta(ppn);
        if meta.usage == PageUsage::KernelSlabs {
            slabs::free(ptr as *mut ());
        } else if meta.usage == PageUsage::KernelHeap {
            assert!(
                (ptr as usize - HHDM_OFFSET == ppn * PAGE_SIZE)
                    && (ppn == ppn >> meta.buddy_order << meta.buddy_order),
                "Misaligned buddy free 0x{:x} (block size 0x{:x})",
                ptr as usize,
                PAGE_SIZE << meta.buddy_order
            );
            drop(pmm::RawMemory::from_raw(NonZero::new_unchecked(
                ppn >> meta.buddy_order << meta.buddy_order,
            )));
        } else {
            panic!("Free of non-heap memory 0x{:x}", ptr as usize);
        }
    }
}
