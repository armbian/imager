use crate::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct SystemZone {
    pub group: u32,
    pub start_blk: u64,
    pub end_blk: u64,
}

pub struct Ext4 {
    pub block_device: Arc<dyn BlockDevice>,
    pub super_block: Ext4Superblock,
    pub system_zone_cache: Option<Vec<SystemZone>>,
}
