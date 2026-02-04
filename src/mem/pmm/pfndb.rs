// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    ops::Range,
    ptr::null_mut,
    sync::atomic::{AtomicU32, Ordering, fence},
};

use crate::mem::pmm::PPN;

/// What a page of memory is used for.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageUsage {
    /// Page is not usable.
    Reserved,
    /// Page is available for use.
    Free,
    /// Allocated kernel memory pages.
    KernelAnon,
    /// Kernel slab heap page.
    KernelSlabs,
    /// Kernel buddy heap page.
    KernelHeap,
    /// Allocated user memory pages.
    UserAnon,
    /// Cache-mode user memory pages.
    UserCache,
}

/// Metadata about a single page of physical memory.
#[derive(Debug)]
pub struct PageMeta {
    // TODO: Should usage and buddy_order be atomic?
    /// What this page is currently in use for.
    pub usage: PageUsage,
    /// What order buddy block this page belongs to.
    pub buddy_order: u8,
    /// Page reference count.
    pub refcount: AtomicU32,
}

/// Pointer to the page frame number database.
/// Contains metadata about the current status of all pages of physical memory.
pub static mut PFNDB: *mut PageMeta = null_mut();
/// The range of valid PFNDB entries.
pub static mut DOMAIN: Range<PPN> = 0..0;

/// Initialize the PFNDB with default values.
pub(super) unsafe fn init(ppn: Range<PPN>) {
    for ppn in ppn {
        let meta = unsafe { page_meta(ppn) };
        meta.usage = PageUsage::Reserved;
        meta.buddy_order = 0;
        meta.refcount = AtomicU32::new(1);
    }
    fence(Ordering::Release);
}

/// Get the metadata for the page that includes the given physical address.
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn page_meta(ppn: PPN) -> &'static mut PageMeta {
    debug_assert!(unsafe { DOMAIN.start <= ppn && ppn < DOMAIN.end });
    &mut *PFNDB.wrapping_add(ppn)
}
