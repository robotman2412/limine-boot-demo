// Copyright © 2025-2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    fmt::{Display, Write},
    ops::Range,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String};

use crate::logk;

/// Loaded device tree structure.
pub struct Dtb {
    /// DTB root node.
    root: Box<DtbNode>,
    /// Map from phandle to node.
    by_phandle: BTreeMap<u32, *const DtbNode>,
}

impl Dtb {
    pub fn root(&self) -> &DtbNode {
        &self.root
    }

    pub fn by_phandle(&self, phandle: u32) -> Option<&DtbNode> {
        self.by_phandle.get(&phandle).map(|x| unsafe { &**x })
    }
}

/// Device tree node.
pub struct DtbNode {
    /// This node's name.
    name: *const str,
    /// Parent node, if any.
    parent: *const DtbNode,
    /// Cached phandle, if any.
    pub phandle: Option<u32>,
    /// Child nodes and props.
    pub nodes: BTreeMap<String, DtbNode>,
    /// Child props.
    pub props: BTreeMap<String, DtbProp>,
}

impl DtbNode {
    /// Get the node's name.
    pub fn name(&self) -> &str {
        if self.name.is_null() {
            return "";
        }
        unsafe { &*self.name }
    }

    /// Get the parent node.
    pub fn parent(&self) -> Option<&DtbNode> {
        if self.parent.is_null() {
            return None;
        }
        Some(unsafe { &*self.parent })
    }
}

impl Display for DtbNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Count how deep in the DTB this node is.
        let mut depth = 0;
        let mut cur = self;
        while let Some(node) = cur.parent() {
            depth += 1;
            cur = node;
        }

        // Iteratively walk down so the path is printed in proper order.
        for x in (1..depth).rev() {
            let mut cur = self;
            for _ in 0..x {
                cur = cur.parent().unwrap();
            }
            f.write_str(cur.name())?;
            f.write_char('/')?;
        }
        f.write_str(self.name())?;

        Ok(())
    }
}

/// Device tree property.
pub struct DtbProp {
    /// This prop's name.
    name: *const str,
    /// Parent node, if any.
    parent: *const DtbNode,
    /// Binary value.
    pub blob: Box<[u8]>,
}

impl DtbProp {
    /// Get the prop's name.
    pub fn name(&self) -> &str {
        unsafe { &*self.name }
    }

    /// Get the parent node.
    pub fn parent(&self) -> &DtbNode {
        unsafe { &*self.parent }
    }

    /// Read a cell in this prop.
    pub fn read_cell(&self, cell: usize) -> Option<u32> {
        self.read_uint_cells(cell..cell + 1).map(|x| x as u32)
    }

    /// Read this prop as some integer.
    pub fn read_uint_cells(&self, cells: Range<usize>) -> Option<u128> {
        debug_assert!(cells.len() <= 4);
        if self.blob.len() / 4 < cells.end {
            logk!(
                LogLevel::Warning,
                "DTB prop {} expected to have at least {} cells but has {}",
                self,
                cells.end,
                self.blob.len() / 4
            );
        }
        let mut value = 0u128;
        for cell in cells {
            value <<= 32;
            value |= (self.blob[cell * 4 + 3] as u128) << 0;
            value |= (self.blob[cell * 4 + 2] as u128) << 8;
            value |= (self.blob[cell * 4 + 1] as u128) << 16;
            value |= (self.blob[cell * 4 + 0] as u128) << 24;
        }
        Some(value)
    }

    /// Read this prop as some integer.
    pub fn read_uint(&self) -> Option<u128> {
        self.read_uint_cells(0..self.blob.len().div_ceil(4))
    }
}

impl Display for DtbProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.parent().fmt(f)?;
        f.write_str(" prop ")?;
        f.write_str(self.name())?;
        Ok(())
    }
}
