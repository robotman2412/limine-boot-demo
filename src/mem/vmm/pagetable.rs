// Copyright © 2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    num::NonZero,
    ops::Range,
    sync::atomic::{Atomic, Ordering},
};

use super::*;
use crate::{
    arch::mmu::{BITS_PER_LEVEL, INVALID_PTE, PackedPTE},
    irq::IrqGuard,
    mem::{
        PAGE_SIZE,
        pmm::{self, PPN},
    },
    misc::errno::EResult,
};

#[derive(Debug, Clone, Copy)]
/// Generic representation of a page table entry.
pub struct PTE {
    /// Physical page number that this PTE points to.
    pub ppn: PPN,
    /// Page protection flags, see [`super::flags`].
    pub flags: u32,
    /// At what level of the page table this PTE is stored.
    pub level: u8,
    /// Whether this PTE is valid.
    pub valid: bool,
    /// Whether this is a leaf PTE.
    pub leaf: bool,
}

impl PartialEq for PTE {
    fn eq(&self, other: &Self) -> bool {
        self.ppn == other.ppn
            && (self.flags & prot::RWX) == (other.flags & prot::RWX)
            && self.level == other.level
            && self.valid == other.valid
            && (self.leaf == other.leaf || !self.valid && !other.valid)
    }
}

impl PTE {
    /// The PTE that represents unmapped memory.
    pub const NULL: PTE = PTE {
        ppn: 0,
        flags: 0,
        level: 0,
        valid: false,
        leaf: false,
    };

    /// Whether this PTE represents unmapped memory (as some invalid PTEs may encode demand-mapped things).
    pub fn is_null(&self) -> bool {
        self.ppn == 0 && self.flags == 0 && !self.valid
    }
}

/// Abstracts direct manipulation and usage of page tables.
#[repr(C)]
pub struct PageTable {
    /// Root page number.
    root_ppn: NonZero<PPN>,
}

// TODO: This has multiple possible race conditions if concurrently modified.
impl PageTable {
    /// Create a new page table.
    pub fn new() -> EResult<Self> {
        Ok(Self {
            root_ppn: alloc_pgtable_page()?,
        })
    }

    /// Get the root page number.
    pub fn root_ppn(&self) -> NonZero<PPN> {
        self.root_ppn
    }

    /// Create a new mapping.
    pub unsafe fn map(&self, vpn: VPN, new_pte: PTE) -> EResult<PTE> {
        let level = new_pte.level;
        unsafe { self.map_impl(vpn, Some(new_pte), level) }
    }

    /// Remove an existing mapping.
    pub unsafe fn unmap(&self, vpn: VPN, level: u8) -> EResult<PTE> {
        unsafe { self.map_impl(vpn, None, level) }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn map_impl(&self, vpn: VPN, new_pte: Option<PTE>, level: u8) -> EResult<PTE> {
        let mut pgtable_ppn = self.root_ppn;
        let null_pte = new_pte.as_ref().map(|x| x.is_null()).unwrap_or(false);
        let global_flag = is_canon_kernel_page(vpn) as u32 * prot::G;
        let mut old_pte = None;

        // Descend the page table to the target level.
        for level in (level + 1..PAGING_LEVELS as u8).rev() {
            let index = get_vpn_index(vpn, level);
            let raw_pte = read_pte(pgtable_ppn.into(), index);
            let pte = PTE::unpack(raw_pte, level);

            loop {
                pgtable_ppn = if !pte.valid {
                    // Create a new level of page table.
                    if null_pte {
                        // Unless the new PTE is null.
                        return Ok(PTE::NULL);
                    }
                    let ppn = alloc_pgtable_page()?;
                    let res = cmpxchg_pte(
                        pgtable_ppn.into(),
                        index,
                        raw_pte,
                        PTE {
                            ppn: ppn.into(),
                            flags: global_flag,
                            valid: true,
                            leaf: false,
                            level,
                        }
                        .pack(),
                    );
                    if res.is_err() {
                        // Another thread concurrently modified this PTE, must retry.
                        pmm::deallocate(ppn, 0);
                        continue;
                    }
                    ppn
                } else if pte.leaf {
                    // A superpage is split into smaller pages.
                    let ppn = split_pgtable_leaf(pte, level - 1)?;
                    old_pte = Some(PTE {
                        ppn: pte.ppn + get_vpn_index(vpn, level - 1),
                        flags: pte.flags,
                        valid: true,
                        leaf: true,
                        level: level - 1,
                    });
                    let res = cmpxchg_pte(
                        pgtable_ppn.into(),
                        index,
                        raw_pte,
                        PTE {
                            ppn: ppn.into(),
                            flags: global_flag,
                            valid: true,
                            leaf: false,
                            level,
                        }
                        .pack(),
                    );
                    if res.is_err() {
                        // Another thread concurrently modified this PTE, must retry.
                        pmm::deallocate(ppn, 0);
                        continue;
                    }
                    ppn
                } else {
                    NonZero::new(pte.ppn).expect("Null page on valid PTE")
                };
                break;
            }
        }

        // Write new PTE.
        if let Some(new_pte) = new_pte {
            let index = get_vpn_index(vpn, new_pte.level);
            let order = new_pte.level;
            let tmp = PTE::unpack(xchg_pte(pgtable_ppn.into(), index, new_pte.pack()), order);
            if old_pte.is_none() {
                old_pte = Some(tmp);
            }
        }

        Ok(old_pte.unwrap_or(PTE::NULL))
    }

    /// Change the protection flags of a specific PTE.
    /// Fails if there is not a protectable PTE present at the correct location.
    pub unsafe fn protect(&self, vpn: VPN, level: u8, prot: u32) -> Result<PTE, PTE> {
        debug_assert!(prot & prot::R != 0);
        debug_assert!(prot & !prot::RWX == 0);
        let mut pgtable_ppn = self.root_ppn;
        let mut pte;

        let _noirq = IrqGuard::new();

        // Descend the page until a leaf is found.
        for _ in (level..unsafe { PAGING_LEVELS as u8 }).rev() {
            let index = get_vpn_index(vpn, level as u8);
            pte = PTE::unpack(unsafe { read_pte(pgtable_ppn.into(), index) }, level as u8);

            if !pte.valid || pte.leaf {
                return Err(pte);
            } else {
                pgtable_ppn = NonZero::new(pte.ppn).expect("Null page on valid PTE");
            }
        }

        let index = get_vpn_index(vpn, level as u8);

        let old = unsafe { read_pte(pgtable_ppn.into(), index) };
        let mut pte = PTE::unpack(old, level as u8);
        if !pte.valid || !pte.leaf {
            return Err(pte);
        }
        pte.flags &= !prot::RWX;
        pte.flags |= prot;
        let new = PTE::pack(pte);

        unsafe { xchg_pte(pgtable_ppn.into(), index, new) };
        Ok(pte)
    }

    /// Walk down the page table and read the target vaddr's PTE.
    #[inline(always)]
    pub fn walk(&self, vpn: VPN) -> PTE {
        self.walk_shallow(vpn, 0)
    }

    /// Walk down the page table and read the target vaddr's PTE.
    pub fn walk_shallow(&self, vpn: VPN, min_level: u32) -> PTE {
        debug_assert!(min_level < unsafe { PAGING_LEVELS });
        let mut pgtable_ppn = self.root_ppn;
        let mut pte;

        let _noirq = IrqGuard::new();

        // Descend the page until a leaf is found.
        for level in (0..unsafe { PAGING_LEVELS }).rev() {
            let index = get_vpn_index(vpn, level as u8);
            pte = PTE::unpack(unsafe { read_pte(pgtable_ppn.into(), index) }, level as u8);

            if level == min_level || !pte.valid && level > 0 {
                return pte;
            } else if pte.valid && !pte.leaf {
                pgtable_ppn = NonZero::new(pte.ppn).expect("Null page on valid PTE");
            } else {
                return pte;
            }
        }

        unreachable!("Valid non-leaf PTE at level 0");
    }

    /// Fill the higher half with empty pages.
    /// Used to construct the kernel page table.
    pub unsafe fn populate_higher_half(&mut self) -> EResult<()> {
        for i in PTE_PER_PAGE / 2..PTE_PER_PAGE {
            unsafe {
                let page = alloc_pgtable_page()?;
                let pte = PTE {
                    ppn: page.into(),
                    flags: 0,
                    level: PAGING_LEVELS as u8 - 1,
                    valid: true,
                    leaf: false,
                };
                xchg_pte(self.root_ppn.into(), i, pte.pack());
            }
        }
        Ok(())
    }

    /// Copy the higher-half mappings from what is assumed to be the kernel page table.
    pub unsafe fn copy_higher_half(&mut self, kernel_pt: &PageTable) {
        for i in PTE_PER_PAGE / 2..PTE_PER_PAGE {
            unsafe {
                xchg_pte(
                    self.root_ppn.into(),
                    i,
                    read_pte(kernel_pt.root_ppn.into(), i),
                );
            }
        }
    }

    /// Recursive implementation of the [`Drop`] trait.
    unsafe fn drop_impl(pgtable_ppn: NonZero<PPN>, level: u8, max: usize) {
        unsafe {
            for i in 0..max {
                let pte = PTE::unpack(read_pte(pgtable_ppn.into(), i), level);
                assert!(level > 0 || !pte.valid || pte.leaf);
                if pte.valid {
                    let pte_ppn = NonZero::new(pte.ppn).expect("Null page on valid PTE");
                    if !pte.leaf {
                        Self::drop_impl(pte_ppn, level - 1, PTE_PER_PAGE);
                    }
                }
            }
            pmm::deallocate(pgtable_ppn, 0);
        }
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        unsafe {
            Self::drop_impl(self.root_ppn, PAGING_LEVELS as u8, PTE_PER_PAGE / 2);
        }
    }
}

/// How many bits of address space ID are available.
pub static mut ASID_BITS: u32 = 0;
/// Number of paging levels.
pub static mut PAGING_LEVELS: u32 = 0;
/// Number of PTEs per page.
pub const PTE_PER_PAGE: usize = PAGE_SIZE / size_of::<PackedPTE>();

/// Get the index in the given page table level for the given virtual address.
#[inline(always)]
fn get_vpn_index(vpn: VPN, level: u8) -> usize {
    (vpn >> (level as u32 * BITS_PER_LEVEL)) % (1usize << BITS_PER_LEVEL)
}

/// Read a PTE without any fencing or flushing.
#[inline(always)]
unsafe fn read_pte(pgtable_ppn: PPN, index: usize) -> PackedPTE {
    let pte_vaddr =
        unsafe { HHDM_OFFSET } + pgtable_ppn * PAGE_SIZE + index * size_of::<PackedPTE>();
    unsafe { (*(pte_vaddr as *mut Atomic<PackedPTE>)).load(Ordering::Acquire) }
}

/// Write a PTE without any fencing or flushing.
#[inline(always)]
unsafe fn xchg_pte(pgtable_ppn: PPN, index: usize, pte: PackedPTE) -> PackedPTE {
    let pte_vaddr =
        unsafe { HHDM_OFFSET } + pgtable_ppn * PAGE_SIZE + index * size_of::<PackedPTE>();
    unsafe { (*(pte_vaddr as *mut Atomic<PackedPTE>)).swap(pte, Ordering::AcqRel) }
}

/// Compare-exchange a PTE without any fencing or flushing.
#[inline(always)]
unsafe fn cmpxchg_pte(
    pgtable_ppn: PPN,
    index: usize,
    old: PackedPTE,
    new: PackedPTE,
) -> Result<PackedPTE, PackedPTE> {
    let pte_vaddr =
        unsafe { HHDM_OFFSET } + pgtable_ppn * PAGE_SIZE + index * size_of::<PackedPTE>();
    unsafe {
        (*(pte_vaddr as *mut Atomic<PackedPTE>)).compare_exchange(
            old,
            new,
            Ordering::AcqRel,
            Ordering::Relaxed,
        )
    }
}

/// Try to allocate a new page table page.
fn alloc_pgtable_page() -> EResult<NonZero<PPN>> {
    let ppn = unsafe { pmm::allocate(0, pmm::pfndb::PageUsage::KernelAnon) }?;
    for i in 0..1usize << BITS_PER_LEVEL {
        unsafe { xchg_pte(ppn.into(), i, INVALID_PTE) };
    }
    Ok(ppn)
}

/// Determine the highest order of page that can be used for the start of a certain mapping.
#[inline(always)]
pub fn calc_superpage(vpn: VPN, ppn: PPN, size: VPN) -> u8 {
    ((vpn | ppn).trailing_zeros().min(size.ilog2()) / BITS_PER_LEVEL) as u8
}

/// Try to split a page table leaf node.
fn split_pgtable_leaf(orig: PTE, new_level: u8) -> EResult<NonZero<PPN>> {
    debug_assert!(orig.leaf && orig.valid);
    let ppn = unsafe { pmm::allocate(0, pmm::pfndb::PageUsage::KernelAnon) }?;

    for i in 0..1usize << BITS_PER_LEVEL {
        unsafe {
            xchg_pte(
                ppn.into(),
                i,
                PTE {
                    ppn: orig.ppn + (i << (new_level as u32 * BITS_PER_LEVEL)),
                    level: new_level,
                    ..orig
                }
                .pack(),
            )
        };
    }

    Ok(ppn)
}

/// Determine whether an address is canonical.
pub fn is_canon_addr(addr: usize) -> bool {
    let addr = addr as isize;
    let exp = usize::BITS - PAGE_SIZE.ilog2() - BITS_PER_LEVEL * unsafe { PAGING_LEVELS };
    let canon_addr = (addr << exp) >> exp;
    canon_addr == addr
}

/// Determine whether an address is a canonical kernel address.
pub fn is_canon_kernel_addr(addr: usize) -> bool {
    is_canon_addr(addr) && (addr as isize) < 0
}

/// Determine whether an address is a canonical user address.
pub fn is_canon_user_addr(addr: usize) -> bool {
    is_canon_addr(addr) && (addr as isize) >= 0
}

/// Determine whether an address is canonical.
pub fn is_canon_range(range: Range<usize>) -> bool {
    is_canon_addr(range.start) && (range.len() == 0 || is_canon_addr(range.end - 1))
}

/// Determine whether an address is a canonical kernel address.
pub fn is_canon_kernel_range(range: Range<usize>) -> bool {
    is_canon_kernel_addr(range.start) && (range.len() == 0 || is_canon_kernel_addr(range.end - 1))
}

/// Determine whether an address is a canonical user address.
pub fn is_canon_user_range(range: Range<usize>) -> bool {
    is_canon_user_addr(range.start) && (range.len() == 0 || is_canon_user_addr(range.end - 1))
}

/// Determine whether an address is canonical.
pub fn is_canon_page(addr: VPN) -> bool {
    // The upper (usually 12) bits of a VPN are ignored because a VPN is actually `usize::BITS - PAGE_SIZE.ilog2()` bits.
    let addr = (addr as isize) << PAGE_SIZE.ilog2() >> PAGE_SIZE.ilog2();
    let exp = usize::BITS - BITS_PER_LEVEL * unsafe { PAGING_LEVELS };
    let canon_page = (addr << exp) >> exp;
    canon_page == addr
}

/// Determine whether an address is a canonical kernel address.
pub fn is_canon_kernel_page(addr: VPN) -> bool {
    is_canon_page(addr) && (addr as isize) << PAGE_SIZE.ilog2() < 0
}

/// Determine whether an address is a canonical user address.
pub fn is_canon_user_page(addr: VPN) -> bool {
    is_canon_page(addr) && (addr as isize) >= 0
}

/// Determine whether an address is canonical.
pub fn is_canon_page_range(range: Range<VPN>) -> bool {
    is_canon_page(range.start) && (range.len() == 0 || is_canon_page(range.end - 1))
}

/// Determine whether an address is a canonical kernel address.
pub fn is_canon_kernel_page_range(range: Range<VPN>) -> bool {
    is_canon_kernel_page(range.start) && (range.len() == 0 || is_canon_kernel_page(range.end - 1))
}

/// Determine whether an address is a canonical user address.
pub fn is_canon_user_page_range(range: Range<VPN>) -> bool {
    is_canon_user_page(range.start) && (range.len() == 0 || is_canon_user_page(range.end - 1))
}

/// Get the size of a "half" of the canonical ranges.
pub fn canon_half_pages() -> usize {
    1 << (BITS_PER_LEVEL * unsafe { PAGING_LEVELS } - 1)
}

/// Get the size of a "half" of the canonical ranges.
pub fn canon_half_size() -> usize {
    PAGE_SIZE << (BITS_PER_LEVEL * unsafe { PAGING_LEVELS } - 1)
}

/// Get the start of the higher half.
pub fn higher_half_vaddr() -> usize {
    canon_half_size().wrapping_neg()
}

/// Get the start of the higher half.
pub fn higher_half_vpn() -> usize {
    higher_half_vaddr() / PAGE_SIZE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_addr(addr: usize) -> bool {
    is_canon_addr(addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_kernel_addr(addr: usize) -> bool {
    is_canon_kernel_addr(addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_user_addr(addr: usize) -> bool {
    is_canon_user_addr(addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_range(start: usize, len: usize) -> bool {
    is_canon_range(start..start + len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_kernel_range(start: usize, len: usize) -> bool {
    is_canon_kernel_range(start..start + len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_is_canon_user_range(start: usize, len: usize) -> bool {
    is_canon_user_range(start..start + len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_canon_half_size() -> usize {
    canon_half_size()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_higher_half_vaddr() -> usize {
    higher_half_vaddr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mmu_paging_levels() -> i32 {
    unsafe { PAGING_LEVELS as i32 }
}
