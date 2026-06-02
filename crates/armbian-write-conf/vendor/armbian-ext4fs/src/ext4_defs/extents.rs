use crate::prelude::*;
use crate::return_errno_with_message;

use super::*;

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentHeader {
    /// Magic number, 0xF30A.
    pub magic: u16,

    /// Number of valid entries following the header.
    pub entries_count: u16,

    /// Maximum number of entries that could follow the header.
    pub max_entries_count: u16,

    /// Depth of this extent node in the extent tree. Depth 0 indicates that this node points to data blocks.
    pub depth: u16,

    /// Generation of the tree (used by Lustre, but not standard in ext4).
    pub generation: u32,
}

/// Structure representing an index node within an extent tree.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Ext4ExtentIndex {
    /// Block number from which this index node starts.
    pub first_block: u32,

    /// Lower 32-bits of the block number to which this index points.
    pub leaf_lo: u32,

    /// Upper 16-bits of the block number to which this index points.
    pub leaf_hi: u16,

    /// Padding for alignment.
    pub padding: u16,
}

/// Structure representing an Ext4 extent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Ext4Extent {
    /// First file block number that this extent covers.
    pub first_block: u32,

    /// Number of blocks covered by this extent.
    pub block_count: u16,

    /// Upper 16-bits of the block number to which this extent points.
    pub start_hi: u16,

    /// Lower 32-bits of the block number to which this extent points.
    pub start_lo: u32,
}

/// Extent tail structure
/// This is at the end of the extent block and contains a checksum
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentTail {
    /// crc32c(uuid+inum+generation+extent_block)
    pub et_checksum: u32,
}

/// Extent tree node. Includes the header, the data.
#[derive(Clone, Debug)]
pub struct ExtentNode {
    pub header: Ext4ExtentHeader,
    pub data: NodeData,
    pub is_root: bool,
}

/// Data of extent tree.
#[derive(Clone, Debug)]
pub enum NodeData {
    Root([u32; 15]),
    Internal(Vec<u8>), // size = BLOCK_SIZE
}

/// Search path in the extent tree.
#[derive(Clone, Debug)]
pub struct SearchPath {
    pub depth: u16,                // current depth
    pub maxdepth: u16,             // max depth
    pub path: Vec<ExtentPathNode>, // search result of each level
}

/// Extent tree node search result
#[derive(Clone, Debug)]
pub struct ExtentPathNode {
    pub header: Ext4ExtentHeader,       // save header for convenience
    pub index: Option<Ext4ExtentIndex>, // for convenience(you can get index through pos of extent node)
    pub extent: Option<Ext4Extent>,     // same reason as above
    pub position: usize,                // position of search result in the node
    pub pblock: u64,                    // physical block of search result
    pub pblock_of_node: usize,          // physical block of this node
}

/// load methods for Ext4ExtentHeader
impl Ext4ExtentHeader {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> &mut Self {
        let ptr = data.as_mut_ptr() as *mut Self;
        unsafe { &mut *ptr }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> &mut Self {
        let ptr = data.as_mut_ptr() as *mut Self;
        unsafe { &mut *ptr }
    }

    /// Is the node a leaf node?
    pub fn is_leaf(&self) -> bool {
        self.depth == 0
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut header = Ext4ExtentHeader {
            magic: 0,
            entries_count: 0,
            max_entries_count: 0,
            depth: 0,
            generation: 0,
        };
        header.magic = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        header.entries_count = u16::from_le_bytes(bytes[2..4].try_into().unwrap());
        header.max_entries_count = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        header.depth = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        header.generation = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        header
    }
}

/// load methods for Ext4ExtentIndex
impl Ext4ExtentIndex {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> Self {
        unsafe { core::ptr::read(data.as_mut_ptr() as *mut _) }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut idx = Ext4ExtentIndex::default();
        idx.first_block = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        idx.leaf_lo = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        idx.leaf_hi = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        idx.padding = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
        idx
    }
}

/// load methods for Ext4Extent
impl Ext4Extent {
    /// Load the extent header from u32 array.
    pub fn load_from_u32(data: &[u32]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u32 array mutably.
    pub fn load_from_u32_mut(data: &mut [u32]) -> Self {
        let ptr = data.as_mut_ptr() as *mut Self;
        unsafe { *ptr }
    }

    /// Load the extent header from u8 array.
    pub fn load_from_u8(data: &[u8]) -> Self {
        unsafe { core::ptr::read(data.as_ptr() as *const _) }
    }

    /// Load the extent header from u8 array mutably.
    pub fn load_from_u8_mut(data: &mut [u8]) -> Self {
        let ptr = data.as_mut_ptr() as *mut Self;
        unsafe { *ptr }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut ext = Ext4Extent::default();
        ext.first_block = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        ext.block_count = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
        ext.start_hi = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
        ext.start_lo = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        ext
    }
}

impl ExtentNode {
    /// Load the extent node from the data.
    pub fn load_from_data(data: &[u8], is_root: bool) -> Result<Self> {
        if is_root {
            if data.len() != 15 * 4 {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }

            let mut root_data = [0u32; 15];
            for (i, chunk) in data.chunks(4).enumerate() {
                root_data[i] = u32::from_le_bytes(chunk.try_into().unwrap());
            }

            let header = Ext4ExtentHeader::load_from_u32(&root_data);

            Ok(ExtentNode {
                header,
                data: NodeData::Root(root_data),
                is_root,
            })
        } else {
            if data.len() != BLOCK_SIZE {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }
            let header = Ext4ExtentHeader::load_from_u8(&data[..size_of::<Ext4ExtentHeader>()]);
            Ok(ExtentNode {
                header,
                data: NodeData::Internal(data.to_vec()),
                is_root,
            })
        }
    }

    /// Load the extent node from the data mutably.
    pub fn load_from_data_mut(data: &mut [u8], is_root: bool) -> Result<Self> {
        if is_root {
            if data.len() != 15 * 4 {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }

            let mut root_data = [0u32; 15];
            for (i, chunk) in data.chunks(4).enumerate() {
                root_data[i] = u32::from_le_bytes(chunk.try_into().unwrap());
            }

            let header = *Ext4ExtentHeader::load_from_u32_mut(&mut root_data);

            Ok(ExtentNode {
                header,
                data: NodeData::Root(root_data),
                is_root,
            })
        } else {
            if data.len() != BLOCK_SIZE {
                return_errno_with_message!(Errno::EINVAL, "Invalid data length for root node");
            }
            let mut header = *Ext4ExtentHeader::load_from_u8_mut(&mut data[..size_of::<Ext4ExtentHeader>()]);
            Ok(ExtentNode {
                header,
                data: NodeData::Internal(data.to_vec()),
                is_root,
            })
        }
    }
}

impl ExtentNode {
    /// Binary search for the extent that contains the given block.
    pub fn binsearch_extent(&mut self, lblock: Ext4Lblk) -> Option<(Ext4Extent, usize)> {
        // empty node
        if self.header.entries_count == 0 {
            match &self.data {
                NodeData::Root(root_data) => {
                    let extent = Ext4Extent::load_from_u32(&root_data[3..]);
                    return Some((extent, 0));
                }
                NodeData::Internal(internal_data) => {
                    let extent = Ext4Extent::load_from_u8(&internal_data[12..]);
                    return Some((extent, 0));
                }
            }
        }

        match &mut self.data {
            NodeData::Root(root_data) => {
                let header = self.header;
                let mut l = 1;
                let mut r = header.entries_count as usize - 1;
                while l <= r {
                    let m = l + (r - l) / 2;
                    let idx = 3 + m * 3;
                    let ext = Ext4Extent::load_from_u32(&root_data[idx..]);
                    if lblock < ext.first_block {
                        r = m - 1;
                    } else {
                        l = m + 1;
                    }
                }
                let idx = 3 + (l - 1) * 3;
                let ext = Ext4Extent::load_from_u32(&root_data[idx..]);
    
                Some((ext, l - 1))
            }
            NodeData::Internal(internal_data) => {
                let mut l = 1;
                let mut r = (self.header.entries_count - 1) as usize;
    
                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = size_of::<Ext4ExtentHeader>() + m * size_of::<Ext4Extent>();
                    let mut ext = Ext4Extent::load_from_u8_mut(&mut internal_data[offset..]);

                    if lblock < ext.first_block {
                        r = m - 1;
                    } else {
                        l = m + 1;  // Otherwise, move to the right half
                    }
                }
                let offset = size_of::<Ext4ExtentHeader>() + (l - 1) * size_of::<Ext4Extent>();
                let mut ext = Ext4Extent::load_from_u8_mut(&mut internal_data[offset..]);

                Some((ext, l - 1))
            }
        }
    }

    /// Binary search for the closest index of the given block.
    pub fn binsearch_idx(&self, lblock: Ext4Lblk) -> Option<usize> {
        if self.header.entries_count == 0 {
            return None;
        }

        match &self.data {
            NodeData::Root(root_data) => {
                // Root node handling
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let indexes = &root_data[start..];

                let mut l = 1; // Skip the first index
                let mut r = self.header.entries_count as usize - 1;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4ExtentIndex>() / 4; // Convert to u32 offset
                    let extent_index = Ext4ExtentIndex::load_from_u32(&indexes[offset..]);

                    if lblock < extent_index.first_block {
                        if m == 0 {
                            break; // Prevent underflow
                        }
                        r = m - 1;
                    } else {
                        l = m + 1;
                    }
                }

                if l == 0 {
                    return None;
                }

                Some(l - 1)
            }
            NodeData::Internal(internal_data) => {
                // Internal node handling
                let start = size_of::<Ext4ExtentHeader>();
                let indexes = &internal_data[start..];

                let mut l = 0;
                let mut r = (self.header.entries_count - 1) as usize;

                while l <= r {
                    let m = l + (r - l) / 2;
                    let offset = m * size_of::<Ext4ExtentIndex>();
                    let extent_index = Ext4ExtentIndex::load_from_u8(&indexes[offset..]);

                    if lblock < extent_index.first_block {
                        if m == 0 {
                            break; // Prevent underflow
                        }
                        r = m - 1;
                    } else {
                        l = m + 1;
                    }
                }

                if l == 0 {
                    return None;
                }

                Some(l - 1)
            }
        }
    }

    /// Get the index node at the given position.
    pub fn get_index(&self, pos: usize) -> Result<Ext4ExtentIndex> {
        match &self.data {
            NodeData::Root(root_data) => {
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let indexes = &root_data[start..];
                let offset = pos * size_of::<Ext4ExtentIndex>() / 4;
                Ok(Ext4ExtentIndex::load_from_u32(&indexes[offset..]))
            }
            NodeData::Internal(internal_data) => {
                let start = size_of::<Ext4ExtentHeader>();
                let indexes = &internal_data[start..];
                let offset = pos * size_of::<Ext4ExtentIndex>();
                Ok(Ext4ExtentIndex::load_from_u8(&indexes[offset..]))
            }
        }
    }

    /// Get the extent node at the given position.
    pub fn get_extent(&self, pos: usize) -> Option<Ext4Extent> {
        match &self.data {
            NodeData::Root(root_data) => {
                let start = size_of::<Ext4ExtentHeader>() / 4;
                let extents = &root_data[start..];
                let offset = pos * size_of::<Ext4Extent>() / 4;
                Some(Ext4Extent::load_from_u32(&extents[offset..]))
            }
            NodeData::Internal(internal_data) => {
                let start = size_of::<Ext4ExtentHeader>();
                let extents = &internal_data[start..];
                let offset = pos * size_of::<Ext4Extent>();
                Some(Ext4Extent::load_from_u8(&extents[offset..]))
            }
        }
    }
}

impl Ext4ExtentIndex {
    /// Get the physical block number to which this index points.
    pub fn get_pblock(&self) -> u64 {
        ((self.leaf_hi as u64) << 32) | (self.leaf_lo as u64)
    }

    /// Stores the physical block number to which this extent points.
    pub fn store_pblock(&mut self, pblock: u64) {
        self.leaf_lo = (pblock & 0xffffffff) as u32;
        self.leaf_hi = (pblock >> 32) as u16;
    }
}

impl Ext4Extent {
    /// Get the first block number(logical) of the extent.
    pub fn get_first_block(&self) -> u32 {
        self.first_block
    }

    /// Set the first block number(logical) of the extent.
    pub fn set_first_block(&mut self, first_block: u32) {
        self.first_block = first_block;
    }

    /// Get the starting physical block number of the extent.
    pub fn get_pblock(&self) -> u64 {
        let lo = u64::from(self.start_lo);
        let hi = u64::from(self.start_hi) << 32;
        lo | hi
    }

    /// Stores the physical block number to which this extent points.
    pub fn store_pblock(&mut self, pblock: u64) {
        self.start_lo = (pblock & 0xffffffff) as u32;
        self.start_hi = (pblock >> 32) as u16;
    }

    /// Returns true if the extent is unwritten.
    pub fn is_unwritten(&self) -> bool {
        self.block_count > EXT_INIT_MAX_LEN
    }

    /// Returns the actual length of the extent.
    pub fn get_actual_len(&self) -> u16 {
        if self.is_unwritten() {
            self.block_count - EXT_INIT_MAX_LEN
        } else {
            self.block_count
        }
    }

    /// Set the actual length of the extent.
    pub fn set_actual_len(&mut self, len: u16){
        self.block_count = len;
    }

    /// Marks the extent as unwritten.
    pub fn mark_unwritten(&mut self) {
        self.block_count |= EXT_INIT_MAX_LEN;
    }

    /// Get the last file block number that this extent covers.
    pub fn get_last_block(&self) -> u32 {
        self.first_block + self.block_count as u32 - 1
    }

    /// Set the last file block number for this extent.
    pub fn set_last_block(&mut self, last_block: u32) {
        self.block_count = (last_block - self.first_block + 1) as u16;
    }
}

impl Ext4ExtentHeader {
    pub fn new(magic: u16, entries: u16, max_entries: u16, depth: u16, generation: u32) -> Self {
        Self {
            magic,
            entries_count: entries,
            max_entries_count: max_entries,
            depth,
            generation,
        }
    }

    pub fn set_depth(&mut self, depth: u16) {
        self.depth = depth;
    }

    pub fn add_depth(&mut self) {
        self.depth += 1;
    }

    pub fn set_entries_count(&mut self, entries_count: u16) {
        self.entries_count = entries_count;
    }

    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn set_magic(&mut self) {
        self.magic = EXT4_EXTENT_MAGIC;
    }

    pub fn set_max_entries_count(&mut self, max_entries_count: u16) {
        self.max_entries_count = max_entries_count;
    }
}

impl SearchPath {
    pub fn new() -> Self {
        SearchPath {
            depth: 0,
            maxdepth: 4,
            path: vec![],
        }
    }
}

impl Default for SearchPath {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_load_from_data() {
        // Create a valid root node data
        let mut data: [u8; 15 * 4] = [0; 15 * 4];
        data[0..2].copy_from_slice(&EXT4_EXTENT_MAGIC.to_le_bytes()); // set magic number
        let node = ExtentNode::load_from_data(&data, true).expect("Failed to load root node");
        assert_eq!(node.header.magic, EXT4_EXTENT_MAGIC);

        // Create a valid internal node data
        let mut data: Vec<u8> = vec![0; BLOCK_SIZE];
        data[0..2].copy_from_slice(&EXT4_EXTENT_MAGIC.to_le_bytes()); // set magic number
        let node = ExtentNode::load_from_data(&data, false).expect("Failed to load internal node");
        assert_eq!(node.header.magic, EXT4_EXTENT_MAGIC);

        // Test invalid data length for root node
        let invalid_data: [u8; 10] = [0; 10];
        let result = ExtentNode::load_from_data(&invalid_data, true);
        assert!(result.is_err(), "Expected error for invalid root node data length");

        // Test invalid data length for internal node
        let invalid_data: [u8; BLOCK_SIZE - 1] = [0; BLOCK_SIZE - 1];
        let result = ExtentNode::load_from_data(&invalid_data, false);
        assert!(result.is_err(), "Expected error for invalid internal node data length");
    }

    #[test]
    fn test_binsearch_extent() {
        // Create a mock extent node
        let extents = [
            Ext4Extent {
                first_block: 0,
                block_count: 10,
                ..Default::default()
            },
            Ext4Extent {
                first_block: 10,
                block_count: 10,
                ..Default::default()
            },
        ];

        let internal_data: Vec<u8> = unsafe {
            let mut data = vec![0; BLOCK_SIZE];
            let header_ptr = data.as_mut_ptr() as *mut Ext4ExtentHeader;
            (*header_ptr).entries_count = 2;
            let extent_ptr = header_ptr.add(1) as *mut Ext4Extent;
            core::ptr::copy_nonoverlapping(extents.as_ptr(), extent_ptr, 2);
            data
        };

        let mut node = ExtentNode {
            header: Ext4ExtentHeader {
                entries_count: 2,
                ..Default::default()
            },
            data: NodeData::Internal(internal_data),
            is_root: false,
        };

        // Search for a block within the extents
        let result = node.binsearch_extent(5);
        assert!(result.is_some());
        let (extent, pos) = result.unwrap();
        assert_eq!(extent.first_block, 0);
        assert_eq!(pos, 0);

        // Search for a block within the second extent
        let result = node.binsearch_extent(15);
        assert!(result.is_some());
        let (extent, pos) = result.unwrap();
        assert_eq!(extent.first_block, 10);
        assert_eq!(pos, 1);

        // Search for a block outside the extents
        let result = node.binsearch_extent(20);
        assert!(result.is_none());
    }

    #[test]
    fn test_binsearch_idx() {
        // Create a mock index node
        let indexes = [
            Ext4ExtentIndex {
                first_block: 0,
                ..Default::default()
            },
            Ext4ExtentIndex {
                first_block: 10,
                ..Default::default()
            },
        ];

        let internal_data: Vec<u8> = unsafe {
            let mut data = vec![0; BLOCK_SIZE];
            let header_ptr = data.as_mut_ptr() as *mut Ext4ExtentHeader;
            (*header_ptr).entries_count = 2;
            let index_ptr = header_ptr.add(1) as *mut Ext4ExtentIndex;
            core::ptr::copy_nonoverlapping(indexes.as_ptr(), index_ptr, 2);
            data
        };

        let node = ExtentNode {
            header: Ext4ExtentHeader {
                entries_count: 2,
                ..Default::default()
            },
            data: NodeData::Internal(internal_data),
            is_root: false,
        };

        // Search for the closest index of the given block
        let result = node.binsearch_idx(5);
        assert!(result.is_some());
        let pos = result.unwrap();
        assert_eq!(pos, 0);

        // Search for the closest index of the given block
        let result = node.binsearch_idx(15);
        assert!(result.is_some());
        let pos = result.unwrap();
        assert_eq!(pos, 1);

        // Search for a block outside the indexes
        let result = node.binsearch_idx(20);
        assert!(result.is_some());
        let pos = result.unwrap();
        assert_eq!(pos, 1);
    }

    #[test]
    fn test_get_index() {
        // Create a mock index node
        let indexes = [
            Ext4ExtentIndex {
                first_block: 0,
                leaf_lo: 1,
                leaf_hi: 2,
                ..Default::default()
            },
            Ext4ExtentIndex {
                first_block: 10,
                leaf_lo: 11,
                leaf_hi: 12,
                ..Default::default()
            },
        ];

        let internal_data: Vec<u8> = unsafe {
            let mut data = vec![0; BLOCK_SIZE];
            let header_ptr = data.as_mut_ptr() as *mut Ext4ExtentHeader;
            (*header_ptr).entries_count = 2;
            let index_ptr = header_ptr.add(1) as *mut Ext4ExtentIndex;
            core::ptr::copy_nonoverlapping(indexes.as_ptr(), index_ptr, 2);
            data
        };

        let node = ExtentNode {
            header: Ext4ExtentHeader {
                entries_count: 2,
                ..Default::default()
            },
            data: NodeData::Internal(internal_data),
            is_root: false,
        };

        // Get the index at position 0
        let index = node.get_index(0).expect("Failed to get index at position 0");
        assert_eq!(index.first_block, 0);
        assert_eq!(index.leaf_lo, 1);
        assert_eq!(index.leaf_hi, 2);

        // Get the index at position 1
        let index = node.get_index(1).expect("Failed to get index at position 1");
        assert_eq!(index.first_block, 10);
        assert_eq!(index.leaf_lo, 11);
        assert_eq!(index.leaf_hi, 12);
    }

    #[test]
    fn test_get_extent() {
        // Create a mock extent node
        let extents = [
            Ext4Extent {
                first_block: 0,
                block_count: 10,
                ..Default::default()
            },
            Ext4Extent {
                first_block: 10,
                block_count: 10,
                ..Default::default()
            },
        ];

        let internal_data: Vec<u8> = unsafe {
            let mut data = vec![0; BLOCK_SIZE];
            let header_ptr = data.as_mut_ptr() as *mut Ext4ExtentHeader;
            (*header_ptr).entries_count = 2;
            let extent_ptr = header_ptr.add(1) as *mut Ext4Extent;
            core::ptr::copy_nonoverlapping(extents.as_ptr(), extent_ptr, 2);
            data
        };

        let node = ExtentNode {
            header: Ext4ExtentHeader {
                entries_count: 2,
                ..Default::default()
            },
            data: NodeData::Internal(internal_data),
            is_root: false,
        };

        // Get the extent at position 0
        let extent = node.get_extent(0).expect("Failed to get extent at position 0");
        assert_eq!(extent.first_block, 0);
        assert_eq!(extent.block_count, 10);

        // Get the extent at position 1
        let extent = node.get_extent(1).expect("Failed to get extent at position 1");
        assert_eq!(extent.first_block, 10);
        assert_eq!(extent.block_count, 10);
    }
}
