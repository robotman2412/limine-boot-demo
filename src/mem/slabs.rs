// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    alloc::AllocError,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};

use crate::{
    array_from_fn, impl_has_list_node,
    mem::{
        PAGE_SIZE,
        pmm::{RawMemory, pfndb::PageUsage},
    },
    misc::list::{InvasiveList, InvasiveListNode},
    sync::spinlock::Spinlock,
};

/// Buddy page order used to allocate slabs.
pub const SLAB_BUDDY_ORDER: u8 = 1;
/// Smallest bucket size.
pub const BUCKET_SIZE_MIN: usize = 8;
/// Log-base 2 multiplier per bucket size.
pub const BUCKET_SIZE_EXP: u32 = 1;
/// Number of bucket sizes.
pub const BUCKET_COUNT: u8 = 6;
/// Largest bucket size.
pub const BUCKET_SIZE_MAX: usize = BUCKET_SIZE_MIN << (BUCKET_SIZE_EXP * (BUCKET_COUNT - 1) as u32);

/// Metadata block for a kernel slab heap page.
struct Slab {
    /// Linked list node.
    node: InvasiveListNode,
    /// What bucket size is in this page.
    bucket: u8,
    /// How many free objects the slab contains.
    free: u16,
}
impl_has_list_node!(Slab, node);

impl Slab {
    /// Allocate and initialize a new slab.
    fn new(bucket: u8) -> Result<*mut Self, AllocError> {
        let mem = RawMemory::new(SLAB_BUDDY_ORDER, PageUsage::KernelSlabs)?;
        let slab = unsafe { &mut *(mem.hhdm_ptr() as *mut Self) };
        core::mem::forget(mem);

        slab.node = InvasiveListNode::new();
        slab.bucket = bucket;
        let bitmap = unsafe { &mut *slab.bitmap_mut() };
        let obj_overhead = slab.obj_overhead();
        slab.free = slab.true_obj_count();

        // TODO: Optimize initial setting.
        for i in obj_overhead..1 << slab.obj_count_exp() {
            bitmap[(i as u32 / usize::BITS) as usize] |= 1 << (i as u32 % usize::BITS);
        }

        Ok(slab)
    }

    /// Get the log-base 2 size of an object.
    #[inline(always)]
    const fn obj_size_exp(&self) -> u32 {
        BUCKET_SIZE_MIN.ilog2() + (BUCKET_SIZE_EXP * self.bucket as u32)
    }

    /// Get the number of "overhead objects".
    #[inline(always)]
    const fn obj_overhead(&self) -> u16 {
        let overhead = self.bitmap().len() * size_of::<usize>() + size_of::<Self>();
        overhead.div_ceil(1 << self.obj_size_exp()) as u16
    }

    /// Get the log-base 2 of the number of objects that will fit sans overhead.
    #[inline(always)]
    const fn obj_count_exp(&self) -> u32 {
        // Long live powers of two for multiplication and division!
        let obj_size_exp = BUCKET_SIZE_MIN.ilog2() + (BUCKET_SIZE_EXP * self.bucket as u32);
        let slab_size_exp = PAGE_SIZE.ilog2() + SLAB_BUDDY_ORDER as u32;
        slab_size_exp - obj_size_exp
    }

    /// Get the number of objects that will fit in this slab.
    #[inline(always)]
    const fn true_obj_count(&self) -> u16 {
        (1 << self.obj_count_exp()) - self.obj_overhead()
    }

    /// Get the bitmap of available objects.
    #[inline(always)]
    const fn bitmap(&self) -> *const [usize] {
        unsafe {
            let base = (self as *const Self).add(1) as *const usize;
            slice_from_raw_parts(
                base,
                1 << (self.obj_count_exp().saturating_sub(usize::BITS.ilog2())),
            )
        }
    }

    /// Get the bitmap of available objects.
    #[inline(always)]
    const fn bitmap_mut(&mut self) -> *mut [usize] {
        unsafe {
            let base = (self as *mut Self).add(1) as *mut usize;
            slice_from_raw_parts_mut(
                base,
                1 << (self.obj_count_exp().saturating_sub(usize::BITS.ilog2())),
            )
        }
    }

    unsafe fn allocate(&mut self) -> *mut () {
        let obj_size_exp = BUCKET_SIZE_MIN.ilog2() + (BUCKET_SIZE_EXP * self.bucket as u32);
        let bitmap = unsafe { &mut *self.bitmap_mut() };

        for i in 0..bitmap.len() {
            if bitmap[i] == 0 {
                continue;
            }

            let lsb = bitmap[i].trailing_zeros();
            bitmap[i] &= !(1 << lsb);

            let obj_index = i * usize::BITS as usize + lsb as usize;
            let ptr = self as *mut Self as *mut ();

            self.free -= 1;
            return unsafe { ptr.byte_add(obj_index << obj_size_exp) };
        }

        panic!("Slab 0x{:x} is already full", self as *mut Self as usize);
    }

    unsafe fn free(&mut self, ptr: *mut ()) {
        let obj_size_exp = self.obj_size_exp();
        let obj_index = (ptr as usize - self as *mut Self as usize) >> obj_size_exp;
        let obj_count = 1 << self.obj_count_exp();
        let bitmap = unsafe { &mut *self.bitmap_mut() };

        let overhead = self.obj_overhead() as usize;
        assert!(
            overhead <= obj_index && obj_index < obj_count,
            "Slab 0x{:x} free 0x{:x} (size 0x{:x}) out of bounds",
            self as *mut Self as usize,
            ptr as usize,
            1usize << obj_size_exp
        );

        let mask = 1 << (obj_index as u32 % usize::BITS);
        let i = obj_index / usize::BITS as usize;
        assert!(
            bitmap[i] & mask == 0,
            "Double free of 0x{:x} in slab 0x{:x}",
            ptr as usize,
            self as *mut Self as usize
        );
        bitmap[i] &= !mask;
        self.free += 1;
    }
}

/// One slabs bucket.
struct Bucket {
    /// Completely full slabs.
    slabs_full: InvasiveList<Slab>,
    /// Partially used slabs.
    slabs_partial: InvasiveList<Slab>,
    /// Completely empty slabs.
    slabs_empty: InvasiveList<Slab>,
    /// What bucket size this is.
    bucket: u8,
}

impl Bucket {
    const fn new(bucket: u8) -> Self {
        Self {
            slabs_full: InvasiveList::new(),
            slabs_partial: InvasiveList::new(),
            slabs_empty: InvasiveList::new(),
            bucket,
        }
    }

    unsafe fn allocate(&mut self) -> Result<*mut (), AllocError> {
        if self.slabs_partial.len() == 0 && self.slabs_empty.len() == 0 {
            for _ in 0..4 {
                if let Ok(slab) = Slab::new(self.bucket) {
                    // Can only fail if the heap is corrupted.
                    unsafe { self.slabs_empty.push_back(slab) }.expect("Heap is corrupt");
                }
            }
        }

        unsafe {
            let slab = self
                .slabs_partial
                .pop_front()
                .or_else(|| self.slabs_empty.pop_front())
                .ok_or(AllocError)?;

            let ptr = (&mut *slab).allocate();
            if (*slab).free == 0 {
                self.slabs_full.push_back(slab).expect("Heap is corrupt");
            } else {
                self.slabs_partial
                    .push_front(slab)
                    .expect("Heap is corrupt");
            }

            Ok(ptr)
        }
    }

    unsafe fn free(&mut self, slab: &mut Slab, ptr: *mut ()) {
        unsafe {
            let was_full = slab.free == slab.true_obj_count();
            slab.free(ptr);
            if slab.free == slab.true_obj_count() {
                self.slabs_partial.remove(slab);
                self.slabs_empty
                    .push_front(slab)
                    .expect("Slab is already in list");
            } else if was_full {
                self.slabs_full.remove(slab);
                self.slabs_partial
                    .push_front(slab)
                    .expect("Slab is already in list");
            }
        }
    }
}

static BUCKETS: [Spinlock<Bucket>; BUCKET_COUNT as usize] =
    array_from_fn!(i {Spinlock::new(Bucket::new(i as u8))});

pub unsafe fn allocate(min_size: usize) -> Result<*mut (), AllocError> {
    for bucket in 0..BUCKET_COUNT {
        let obj_size = BUCKET_SIZE_MIN << (BUCKET_SIZE_EXP * bucket as u32);
        if min_size <= obj_size {
            return unsafe { BUCKETS[bucket as usize].lock().allocate() };
        }
    }

    unreachable!(
        "Slab allocator asked to allocate too large (0x{:x}) an object",
        min_size
    );
}

pub unsafe fn free(ptr: *mut ()) {
    unsafe {
        let exp = PAGE_SIZE.ilog2() + SLAB_BUDDY_ORDER as u32;
        let meta = &mut *((ptr as usize >> exp << exp) as *mut Slab);
        BUCKETS[meta.bucket as usize].lock().free(meta, ptr);
    }
}
