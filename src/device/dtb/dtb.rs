// Copyright © 2025-2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{
    ffi::CStr,
    fmt::{Debug, Display, Write},
    ops::Range,
    ptr::slice_from_raw_parts,
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String};

use crate::logk;

const FDT_BEGIN_NODE: u32 = 0x1;
const FDT_END_NODE: u32 = 0x2;
const FDT_PROP: u32 = 0x3;
const FDT_NOP: u32 = 0x4;
const FDT_END: u32 = 0x9;

const FDT_MAGIC: u32 = 0xd00dfeed;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

/// Helper type for iterating over the FDT.
struct FdtIter {
    cursor: *const u32,
}

impl FdtIter {
    /// Get the next token in the FDT, if any.
    pub fn next(&mut self) -> u32 {
        loop {
            let tmp = u32::from_be(unsafe { *self.cursor });
            self.cursor = unsafe { self.cursor.add(1) };
            if tmp != FDT_NOP {
                return tmp;
            }
        }
    }

    /// Read a 32-bit big-endian integer from the FDT.
    pub fn read_u32(&mut self) -> u32 {
        let tmp = u32::from_be(unsafe { *self.cursor });
        self.cursor = unsafe { self.cursor.add(1) };
        tmp
    }

    /// Skip N bytes and re-align to the next 4-byte boundary.
    pub fn skip(&mut self, bytes: usize) {
        self.cursor = unsafe { self.cursor.add((bytes + 3) / 4) };
    }
}

/// Loaded device tree structure.
pub struct Dtb {
    /// DTB root node.
    root: Box<DtbNode>,
    /// Map from phandle to node.
    by_phandle: BTreeMap<u32, *const DtbNode>,
}

impl Dtb {
    /// Parse a DTB from the given pointer.
    pub unsafe fn parse(ptr: *const u8) -> Self {
        unsafe {
            let header = &*(ptr as *const FdtHeader);
            assert!(u32::from_be(header.magic) == FDT_MAGIC, "Invalid DTB magic");
            assert!(
                u32::from_be(header.version) >= 17 && u32::from_be(header.last_comp_version) <= 17,
                "Unsupported FDT (versions {}-{})",
                u32::from_be(header.last_comp_version),
                u32::from_be(header.version)
            );

            let mut iter = FdtIter {
                cursor: ptr.add(u32::from_be(header.off_dt_struct) as usize) as *const u32,
            };
            let strblk = ptr.add(u32::from_be(header.off_dt_strings) as usize) as *const u8;
            let mut by_phandle = BTreeMap::new();

            assert!(
                iter.next() == FDT_BEGIN_NODE,
                "DTB does not start with a node"
            );
            let (_, root) = DtbNode::parse(&mut iter, &mut by_phandle, strblk);

            Dtb { root, by_phandle }
        }
    }

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
    pub nodes: BTreeMap<String, Box<DtbNode>>,
    /// Child props.
    pub props: BTreeMap<String, Box<DtbProp>>,
}

impl DtbNode {
    /// Parse a node from the FDT.
    unsafe fn parse(
        iter: &mut FdtIter,
        by_phandle: &mut BTreeMap<u32, *const DtbNode>,
        strblk: *const u8,
    ) -> (String, Box<Self>) {
        let name_cstr = unsafe { CStr::from_ptr(iter.cursor as *const u8) };
        let name: String = name_cstr.to_str().unwrap_or_default().into();
        iter.skip(name_cstr.to_bytes_with_nul().len());

        let name_ptr = name.as_str() as *const str;
        let mut this = Box::new(Self {
            name: name_ptr,
            parent: core::ptr::null(),
            phandle: None,
            nodes: BTreeMap::new(),
            props: BTreeMap::new(),
        });

        loop {
            let token = iter.next();

            match token {
                FDT_BEGIN_NODE => {
                    let (child_name, mut child_node) =
                        unsafe { DtbNode::parse(iter, by_phandle, strblk) };
                    child_node.parent = this.as_ref() as *const DtbNode;
                    this.nodes.insert(child_name, child_node);
                }
                FDT_END_NODE => break,
                FDT_PROP => {
                    let (prop_name, mut prop) = unsafe { DtbProp::parse(iter, strblk) };
                    prop.parent = this.as_ref() as *const DtbNode;
                    if prop_name == "phandle" || prop_name == "linux,phandle" {
                        if let Some(value) = prop.read_uint_cells(0..1) {
                            this.phandle = Some(value as u32);
                            by_phandle.insert(value as u32, this.as_ref() as *const DtbNode);
                        }
                    }
                    this.props.insert(prop_name, prop);
                }
                FDT_END => break,
                _ => panic!("Unexpected FDT token: 0x{:x}", token),
            }
        }

        (name, this)
    }

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

impl Debug for DtbNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.name().is_empty() {
            f.write_char('/')?;
        } else {
            f.write_str(self.name())?;
        }
        f.write_str(" = {\n")?;

        struct IndentWriter<'a, 'b> {
            inner: &'a mut core::fmt::Formatter<'b>,
            on_newline: bool,
        }

        impl<'a, 'b> Write for IndentWriter<'a, 'b> {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for line in s.split_inclusive('\n') {
                    if self.on_newline {
                        self.inner.write_str("    ")?;
                    }
                    self.inner.write_str(line)?;
                    self.on_newline = line.ends_with('\n');
                }
                Ok(())
            }
        }

        for prop in self.props.values() {
            let mut indent_writer = IndentWriter {
                inner: f,
                on_newline: true,
            };
            write!(indent_writer, "{:?}", prop)?;
        }
        for node in self.nodes.values() {
            let mut indent_writer = IndentWriter {
                inner: f,
                on_newline: true,
            };
            write!(indent_writer, "{:?}", node)?;
        }

        f.write_str("}\n")?;
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
    /// Parse a prop from the FDT.
    unsafe fn parse(iter: &mut FdtIter, strblk: *const u8) -> (String, Box<Self>) {
        let len = iter.read_u32() as usize;
        let nameoff = iter.read_u32() as usize;
        let name_cstr = unsafe { CStr::from_ptr(strblk.add(nameoff)) };
        let name: String = name_cstr.to_str().unwrap_or_default().into();
        let blob: Box<[u8]> =
            unsafe { &*slice_from_raw_parts(iter.cursor as *const u8, len) }.into();
        iter.skip(len);
        let name_ptr = name.as_str() as *const str;
        (
            name,
            Box::new(Self {
                name: name_ptr,
                parent: core::ptr::null(),
                blob,
            }),
        )
    }

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

    /// Whether this likely contains a string.
    pub fn is_likely_string(&self) -> bool {
        if self.blob.is_empty() {
            return false;
        }
        // Check for printable ASCII and null termination.
        for &b in &self.blob[..self.blob.len() - 1] {
            if !(b.is_ascii_graphic() || b == b' ' || b == 0) {
                return false;
            }
        }
        self.blob[self.blob.len() - 1] == 0
    }
}

impl Display for DtbProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.parent())?;
        f.write_str(" prop ")?;
        f.write_str(self.name())?;
        Ok(())
    }
}

impl Debug for DtbProp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())?;
        f.write_str(" = ")?;
        if self.is_likely_string() {
            f.write_char('\"')?;
            for c in &self.blob {
                if c.is_ascii_graphic() || *c == b' ' {
                    f.write_char(*c as char)?;
                } else {
                    write!(f, "\\x{:02x}", c)?;
                }
            }
            f.write_char('\"')?;
        } else if self.blob.len() % 4 != 0 {
            f.write_char('<')?;
            for (i, byte) in self.blob.iter().enumerate() {
                if i > 0 {
                    f.write_char(' ')?;
                }
                write!(f, "0x{:02x}", byte)?;
            }
            f.write_char('>')?;
        } else {
            f.write_char('<')?;
            for i in 0..(self.blob.len() / 4) {
                if i > 0 {
                    f.write_char(' ')?;
                }
                let value = (self.blob[i * 4 + 0] as u32) << 24
                    | (self.blob[i * 4 + 1] as u32) << 16
                    | (self.blob[i * 4 + 2] as u32) << 8
                    | (self.blob[i * 4 + 3] as u32) << 0;
                write!(f, "0x{:08x}", value)?;
            }
            f.write_char('>')?;
        }
        f.write_str(";\n")?;

        Ok(())
    }
}
