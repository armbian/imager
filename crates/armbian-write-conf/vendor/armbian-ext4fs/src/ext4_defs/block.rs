use crate::prelude::*;

pub trait BlockDevice: Send + Sync + Any {
    fn read_offset(&self, offset: usize) -> Vec<u8>;
    fn write_offset(&self, offset: usize, data: &[u8]);
}

pub struct Block {
    pub disk_offset: usize,
    pub data: Vec<u8>,
}

impl Block {
    /// Load the block from the disk.
    pub fn load(block_device: &Arc<dyn BlockDevice>, offset: usize) -> Self {
        let data = block_device.read_offset(offset);
        Block {
            disk_offset: offset,
            data,
        }
    }

    /// Load the block from inode block
    pub fn load_inode_root_block(data: &[u32; 15]) -> Self {
        let data_bytes: &[u8; 60] = unsafe {
            core::mem::transmute(data)
        };
        Block {
            disk_offset: 0, 
            data: data_bytes.to_vec(),
        }
    }

    /// Read the block as a specific type.
    pub fn read_as<T: Copy>(&self) -> T {
        unsafe {
            let ptr = self.data.as_ptr() as *const T;
            ptr.read_unaligned()
        }
    }

    /// Read the block as a specific type at a specific offset.
    pub fn read_offset_as<T: Copy>(&self, offset: usize) -> T {
        unsafe {
            let ptr = self.data.as_ptr().add(offset) as *const T;
            ptr.read_unaligned()
        }
    }

    /// Read the block as a specific type mutably.
    pub fn read_as_mut<T: Copy>(&mut self) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            &mut *ptr
        }
    }

    /// Read the block as a specific type mutably at a specific offset.
    pub fn read_offset_as_mut<T: Copy>(&mut self, offset: usize) -> &mut T {
        unsafe {
            let ptr = self.data.as_mut_ptr().add(offset) as *mut T;
            &mut *ptr
        }
    }

    /// Write data to the block starting at a specific offset.
    pub fn write_offset(&mut self, offset: usize, data: &[u8], len: usize) {
        let end = offset + len;
        if end <= self.data.len() {
            let slice_end = len.min(data.len());
            self.data[offset..end].copy_from_slice(&data[..slice_end]);
        } else {
            panic!("Write would overflow the block buffer");
        }
    }
}

impl Block{
    pub fn sync_blk_to_disk(&self, block_device: &Arc<dyn BlockDevice>){
        block_device.write_offset(self.disk_offset, &self.data);
    }
}