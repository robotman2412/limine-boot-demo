// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    alloc::AllocError,
    ops::{Div, Range},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    mem::{
        PAGE_SIZE,
        pmm::pfndb::{PageMeta, PageUsage, page_meta},
        vmm::HHDM_OFFSET,
    },
    misc::list::{InvasiveList, InvasiveListNode},
    sync::spinlock::Spinlock,
};

pub(super) mod pfndb;

pub type PPN = usize;

/// The largest order that buddy blocks will be coalesced to.
pub const MAX_ORDER: u8 = 32;

/// A shared ownership of naturally-aligned block of physical memory.
pub struct RawMemory {
    ppn: PPN,
}

impl RawMemory {
    /// Try to allocate a block of memory.
    pub fn new(order: u8, usage: PageUsage) -> Result<Self, AllocError> {
        Ok(Self {
            ppn: unsafe { allocate(order, usage) }?,
        })
    }

    /// Create from physical page number without increasing its refcount.
    pub unsafe fn from_raw(ppn: PPN) -> Self {
        Self { ppn }
    }

    /// Create from physical page number and increase its refcount.
    pub unsafe fn from_raw_ref(ppn: PPN) -> Self {
        unsafe {
            page_meta(ppn).refcount.fetch_add(1, Ordering::Relaxed);
        }
        Self { ppn }
    }

    /// Get a pointer to this memory in the HHDM.
    pub fn hhdm_ptr(&self) -> *mut () {
        (self.ppn * PAGE_SIZE + unsafe { HHDM_OFFSET }) as _
    }
}

impl Clone for RawMemory {
    fn clone(&self) -> Self {
        unsafe {
            page_meta(self.ppn).refcount.fetch_add(1, Ordering::Relaxed);
        }
        Self { ppn: self.ppn }
    }
}

impl Drop for RawMemory {
    fn drop(&mut self) {
        unsafe {
            let meta = page_meta(self.ppn);
            if meta.refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
                deallocate(self.ppn, meta.buddy_order);
            }
        }
    }
}

/// The buddy allocator state.
struct BuddyAlloc {
    freelists: [InvasiveList<InvasiveListNode>; MAX_ORDER as usize],
}

impl BuddyAlloc {
    const fn new() -> Self {
        Self {
            freelists: [const { InvasiveList::new() }; _],
        }
    }

    /// Mark one block of memory as free.
    unsafe fn free(&mut self, mut ppn: PPN, mut order: u8, may_coalesce: bool) {
        // Attempt to coalesce.
        while may_coalesce && order < MAX_ORDER {
            let buddy_ppn = ppn ^ (1 << order);
            unsafe {
                if buddy_ppn < pfndb::DOMAIN.start || buddy_ppn >= pfndb::DOMAIN.end {
                    break;
                }
            }
            let buddy = unsafe { page_meta(buddy_ppn) };
            if buddy.usage != PageUsage::Free || buddy.buddy_order != order {
                break;
            }
            unsafe {
                self.freelists[order as usize]
                    .remove((buddy_ppn * PAGE_SIZE + HHDM_OFFSET) as *mut _);
            }
            ppn = ppn.min(buddy_ppn);
            order += 1;
        }

        // Update block metadata.
        for ppn in ppn..ppn + (1 << order) {
            let meta = unsafe { page_meta(ppn) };
            meta.buddy_order = order;
            meta.usage = PageUsage::Free;
            meta.refcount.store(0, Ordering::Release);
        }

        // Insert into the correct freelist.
        unsafe {
            let node = (ppn * PAGE_SIZE + HHDM_OFFSET) as *mut InvasiveListNode;
            *node = InvasiveListNode::new();
            self.freelists[order as usize]
                .push_front(node)
                .expect("PMM freelist is corrupt");
        }
    }

    /// Try to allocate a block of memory.
    fn allocate(&mut self, order: u8, usage: PageUsage) -> Option<PPN> {
        assert!(usage != PageUsage::Free);
        if order > MAX_ORDER {
            return None;
        }
        let mut split_order = None;
        for order in order..MAX_ORDER {
            if self.freelists[order as usize].len() != 0 {
                split_order = Some(order);
                break;
            }
        }
        let split_order = split_order?;

        // Split down to desired order.
        let block = unsafe { self.freelists[split_order as usize].pop_front().unwrap() };
        let ppn = (block as usize - unsafe { HHDM_OFFSET }) / PAGE_SIZE;
        if split_order > order {
            unsafe {
                for order in order + 1..split_order {
                    self.free(ppn + (1 << order), order, false);
                }
            }
        }

        // Update block metadata.
        for ppn in ppn..ppn + (1 << order) {
            let meta = unsafe { page_meta(ppn) };
            meta.buddy_order = order;
            meta.usage = usage;
            meta.refcount.store(0, Ordering::Release);
        }

        Some(ppn)
    }
}

static BUDDY_ALLOC: Spinlock<BuddyAlloc> = Spinlock::new(BuddyAlloc::new());

/// How many pages of RAM are accounted by the buddy allocator.
static TOTAL_PAGES: AtomicUsize = AtomicUsize::new(0);
/// How many pages of free space is available.
static FREE_PAGES: AtomicUsize = AtomicUsize::new(0);
/// How many pages are in use by the kernel.
static KERNEL_PAGES: AtomicUsize = AtomicUsize::new(0);
/// How many pages are in use by userspace.
static USER_PAGES: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn init(early_paddr: Range<usize>, all_ram: Range<usize>) {
    // Reserve space for the PFNDB.
    let ram_ppn = all_ram.start.div_ceil(PAGE_SIZE)..all_ram.end.div(PAGE_SIZE);
    let pfndb_size = size_of::<PageMeta>() * ram_ppn.len();
    let pfndb_paddr = early_paddr.start;
    let early_paddr = early_paddr.start + pfndb_size..early_paddr.end;

    TOTAL_PAGES.store(ram_ppn.len(), Ordering::Relaxed);

    unsafe {
        // Set up datastructures.
        pfndb::DOMAIN = ram_ppn.clone();
        pfndb::PFNDB = ((pfndb_paddr + HHDM_OFFSET) as *mut PageMeta).wrapping_sub(ram_ppn.start);
        pfndb::init(ram_ppn);

        // Mark the remainder of the early region as usable.
        mark_usable(early_paddr);
    }
}

/// Mark a contiguous range of memory as usable.
pub unsafe fn mark_usable(memory: Range<usize>) {
    let memory_pages = memory.start.div_ceil(PAGE_SIZE)..memory.end.div(PAGE_SIZE);
    let mut alloc = BUDDY_ALLOC.lock();
    for page in memory_pages {
        unsafe {
            alloc.free(page, 0, true);
        }
    }
}

/// Allocate a block of physical memory.
pub unsafe fn allocate(order: u8, usage: PageUsage) -> Result<PPN, AllocError> {
    let ppn = BUDDY_ALLOC
        .lock()
        .allocate(order, usage)
        .ok_or(AllocError)?;

    // Account memory usage statistics.
    let count = 1usize << order as u32;
    FREE_PAGES.fetch_sub(count, Ordering::Relaxed);
    match usage {
        PageUsage::Reserved | PageUsage::Free => unreachable!(),
        PageUsage::KernelAnon | PageUsage::KernelSlabs | PageUsage::KernelHeap => {
            KERNEL_PAGES.fetch_add(count, Ordering::Relaxed);
        }
        PageUsage::UserAnon | PageUsage::UserCache => {
            USER_PAGES.fetch_add(count, Ordering::Relaxed);
        }
    }

    Ok(ppn)
}

/// Free a block of physical memory.
pub unsafe fn deallocate(ppn: PPN, order: u8) {
    let meta;
    unsafe {
        meta = page_meta(ppn);
        BUDDY_ALLOC.lock().free(ppn, order, true);
    }

    // Account memory usage statistics.
    let count = 1usize << order as u32;
    match meta.usage {
        PageUsage::Reserved | PageUsage::Free => unreachable!(),
        PageUsage::KernelAnon | PageUsage::KernelSlabs | PageUsage::KernelHeap => {
            KERNEL_PAGES.fetch_sub(count, Ordering::Relaxed);
        }
        PageUsage::UserAnon | PageUsage::UserCache => {
            USER_PAGES.fetch_sub(count, Ordering::Relaxed);
        }
    }
    FREE_PAGES.fetch_add(count, Ordering::Relaxed);
}

pub fn total_memory() -> usize {
    TOTAL_PAGES.load(Ordering::Relaxed) * PAGE_SIZE
}

pub fn free_memory() -> usize {
    FREE_PAGES.load(Ordering::Relaxed) * PAGE_SIZE
}

pub fn kernel_memory() -> usize {
    KERNEL_PAGES.load(Ordering::Relaxed) * PAGE_SIZE
}

pub fn user_memory() -> usize {
    USER_PAGES.load(Ordering::Relaxed) * PAGE_SIZE
}
