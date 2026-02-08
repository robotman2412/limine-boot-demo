// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::ops::Range;

use alloc::collections::linked_list::{self, LinkedList};

use crate::{
    logk,
    mem::{
        PAGE_SIZE,
        pmm::PPN,
        vmm::pagetable::{PTE, canon_half_size, is_canon_kernel_page_range},
    },
    misc::errno::{EResult, Errno},
    sched::sync::mutex::Mutex,
};

use super::*;

// TODO: (transparent?) hugepages support.

/// Allocator for virtal address ranges.
#[derive(Clone)]
#[repr(C)]
pub(super) struct VmaAlloc {
    /// Any pages before this will never be marked free.
    range_start: VPN,
    /// Any pages at or after this will never be marked free.
    range_end: VPN,
    /// Number of free virtual pages.
    free_page_count: VPN,
    /// Linked list of free kernel virtual addresses.
    free_list: LinkedList<Range<VPN>>,
}

impl VmaAlloc {
    /// Padding implicitly placed before and after any range allocated by [`Self::alloc`].
    pub const PADDING: VPN = 65536 / PAGE_SIZE;

    /// Create an empty allocator.
    pub fn new(page_range: Range<VPN>) -> EResult<Self> {
        let mut tmp = Self {
            range_start: page_range.start,
            range_end: page_range.end,
            free_page_count: 0,
            free_list: LinkedList::new(),
        };
        tmp.free(page_range);
        Ok(tmp)
    }

    /// The amount of free pages of virtual memory left in total.
    pub fn _free_page_count(&self) -> VPN {
        self.free_page_count
    }

    /// Mark a new region as free.
    pub fn free(&mut self, pages: Range<VPN>) {
        let pages = pages.start.max(self.range_start)..pages.end.min(self.range_end);
        if pages.len() == 0 {
            return;
        }

        if self.free_list.is_empty() {
            self.free_page_count += pages.len();
            self.free_list.push_back(pages);
            return;
        }

        // This finds the place to insert and removes overlap from other free regions with this one.
        let mut cursor = self.steal_impl(pages.clone());
        cursor.insert_before(pages.clone());

        // Coalesce the newly inserted entry with its neighbors.
        // The second statement must use `current` because it may no longer equal `pages`.
        if let Some(next) = cursor.peek_next()
            && next.start == pages.end
        {
            next.start = pages.start;
            cursor.remove_current();
        }
        if let Some(prev) = cursor.peek_prev()
            && prev.end == pages.start
        {
            cursor.current().unwrap().start = prev.start;
            cursor.move_prev();
            cursor.remove_current();
        }

        self.free_page_count += pages.len();
    }

    /// Allocate a range of pages.
    pub fn alloc(&mut self, amount: VPN) -> EResult<VPN> {
        if amount == 0 {
            return Err(Errno::EINVAL);
        }

        let mut cursor = self.free_list.cursor_front_mut();
        while let Some(next) = cursor.current() {
            if next.len() == amount + 2 * Self::PADDING {
                let vpn = next.start + Self::PADDING;
                cursor.remove_current();
                return Ok(vpn);
            } else if next.len() > amount + 2 * Self::PADDING {
                let vpn = next.start + Self::PADDING;
                next.start += amount + 2 * Self::PADDING;
                return Ok(vpn);
            } else {
                cursor.move_next();
            }
        }

        logk!(
            LogLevel::Error,
            "Out of virtual memory space (this should be nigh impossible!)"
        );
        Err(Errno::ENOMEM)
    }

    /// Mark a specific range as in use.
    pub fn steal(&mut self, pages: Range<VPN>) {
        if pages.len() == 0 {
            return;
        }
        self.steal_impl(pages);
    }

    /// Common implementation of [`Self::steal`] and [`Self::free`].
    fn steal_impl<'a>(&'a mut self, pages: Range<VPN>) -> linked_list::CursorMut<'a, Range<usize>> {
        let mut cursor = self.free_list.cursor_front_mut();
        while let Some(elem) = cursor.current() {
            if elem.start >= pages.start && elem.end <= pages.end {
                // Cursor entirely contained within range.
                self.free_page_count -= elem.len();
                cursor.remove_current();
            } else if pages.contains(&elem.end) {
                // End of cursor contained within range.
                self.free_page_count -= elem.end - pages.start;
                elem.end = pages.start;
                cursor.move_next();
            } else if pages.contains(&elem.start) {
                // Start of cursor contained within range.
                self.free_page_count -= pages.end - elem.start;
                elem.start = pages.end;
                break;
            } else if elem.start >= pages.end {
                // First element after range.
                break;
            } else {
                cursor.move_next();
            }
        }
        cursor
    }
}

/// The kernel memory map.
pub struct KernelMemmap {
    pub(super) pmap: PageTable,
    pub(super) vma: Mutex<VmaAlloc>,
}

impl KernelMemmap {
    pub(super) fn new() -> EResult<Self> {
        let hh_start = canon_half_size().wrapping_neg() / PAGE_SIZE;
        let page_range = hh_start + canon_half_size() / 16..hh_start + canon_half_size();
        Ok(Self {
            pmap: PageTable::new()?,
            vma: Mutex::new(VmaAlloc::new(page_range)?),
        })
    }

    pub unsafe fn map(
        &self,
        vaddr: Option<usize>,
        paddr: usize,
        size: usize,
        prot: u32,
    ) -> EResult<usize> {
        let vpn_start = vaddr.map(|x| x / PAGE_SIZE);
        let ppn_start = paddr / PAGE_SIZE;
        let page_count = (size + (paddr % PAGE_SIZE) + PAGE_SIZE - 1) / PAGE_SIZE;
        if let Some(vaddr) = vaddr
            && vaddr % PAGE_SIZE != paddr % PAGE_SIZE
        {
            logk!(
                LogLevel::Error,
                "Misaligned mapping rejected; vaddr: 0x{:x} paddr: 0x{:x} size: 0x{:x}",
                vaddr,
                paddr,
                size
            );
        }

        unsafe { self.map_pages(vpn_start, ppn_start, page_count, prot) }
    }

    pub unsafe fn unmap(&self, vaddr: usize, size: usize) {
        let vpn_start = vaddr / PAGE_SIZE;
        let page_count = (size + (vaddr % PAGE_SIZE) + PAGE_SIZE - 1) / PAGE_SIZE;

        unsafe { self.unmap_pages(vpn_start, page_count) }
    }

    pub unsafe fn map_pages(
        &self,
        vpn: Option<VPN>,
        ppn: PPN,
        size: VPN,
        prot: u32,
    ) -> EResult<VPN> {
        debug_assert!(prot & prot::R != 0);
        debug_assert!(prot & !(prot::RWX | prot::IO | prot::NC) == 0);
        let vpn = match vpn {
            Some(x) => {
                debug_assert!(is_canon_kernel_page_range(x..x.wrapping_add(size)));
                self.vma.lock().steal(x..x + size);
                x
            }
            None => self.vma.lock().alloc(size)?,
        };

        let flags = prot | prot::G | prot::A | prot::D;
        for i in 0..size {
            unsafe {
                self.pmap.map(
                    vpn + i,
                    PTE {
                        ppn: ppn + i,
                        flags,
                        level: 0,
                        valid: true,
                        leaf: true,
                    },
                )?;
                // TODO: This will leak various things if a mapping fails halfway.
            }
        }

        Ok(vpn)
    }

    pub unsafe fn unmap_pages(&self, vpn: VPN, size: VPN) {
        debug_assert!(is_canon_kernel_page_range(vpn..vpn.wrapping_add(size)));

        for vpn in vpn..vpn + size {
            unsafe {
                self.pmap.unmap(vpn, 0).unwrap();
            }
        }
    }
}
