// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::ops::Range;

use alloc::{boxed::Box, collections::linked_list::LinkedList, sync::Arc};

use crate::{mem::pmm::RawMemory, misc::errno::EResult, sched::sync::mutex::Mutex};

use super::{pagetable::PageTable, *};

pub mod map {
    /// Private mapping; copy-on-write.
    pub const PRIVATE: u32 = 1 << 0;
    /// Shared mapping; directly accesses the page cache OR not coverted to copy-on-write upon fork.
    pub const SHARED: u32 = 1 << 1;
}

/// An object that can be mapped into memory.
pub trait Pager {
    /// Try to acquire a load of the backing object.
    /// The resulting page may be written to by userland if the matching [`Mapping`] is writable.
    fn load(&self, offset: VPN) -> EResult<RawMemory>;
}

pub type Anon = Arc<RawMemory>;
pub type AnonMap = [Option<Anon>];

/// A single mapping.
struct Mapping {
    /// Virtual page range that this is mapped into.
    range: Range<VPN>,
    /// Protection flags requested by userspace.
    prot: u32,
    /// Mapping flags requested by userspace.
    map: u32,
    /// Handle to the backing object.
    pager: Arc<dyn Pager>,
    /// Mapping of pages that shadow the backing object.
    amap: Option<Box<AnonMap>>,
}

impl Mapping {
    /// Handle a page fault; implementation of CoW, demand-paging, etc.
    /// Returns whether the access should be retried.
    pub fn page_fault(&self, pmap: &PageTable, vma: VPN, request: AccessType) -> bool {
        // TODO.
        false
    }
}

/// A process' memory map.
pub struct Memmap {
    /// Page tables that implement the memory mappings.
    pmap: PageTable,
    /// Authoritative set of memory mappings.
    map: Mutex<LinkedList<Mapping>>,
}

impl Memmap {
    /// Create a new, blank, memory map.
    pub fn new() -> EResult<Self> {
        let mut pmap = PageTable::new()?;
        unsafe {
            pmap.copy_higher_half(&kernel_mm());
        }
        Ok(Self {
            pmap,
            map: Mutex::new(LinkedList::new()),
        })
    }

    /// Handle a page fault; implementation of CoW, demand-paging, etc.
    /// Returns whether the access should be retried.
    pub fn page_fault(&self, vma: VPN, request: AccessType) -> bool {
        let map = self.map.lock_shared();
        for mapping in &*map {
            if mapping.range.contains(&vma) {
                return mapping.page_fault(&self.pmap, vma, request);
            }
        }
        false
    }
}
