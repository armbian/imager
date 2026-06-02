/// Check if a bit is set in the bitmap
/// Parameter bmap: Bitmap array
/// Parameter bit: Bit index in the bitmap
pub fn ext4_bmap_is_bit_set(bmap: &[u8], bit: u32) -> bool {
    bmap[(bit >> 3) as usize] & (1 << (bit & 7)) != 0
}

/// Check if a bit is cleared in the bitmap
/// Parameter bmap: Bitmap array
/// Parameter bit: Bit index in the bitmap
pub fn ext4_bmap_is_bit_clr(bmap: &[u8], bit: u32) -> bool {
    !ext4_bmap_is_bit_set(bmap, bit)
}

/// Set a bit in the bitmap
/// Parameter bmap: Bitmap array
/// Parameter bit: Bit index in the bitmap
pub fn ext4_bmap_bit_set(bmap: &mut [u8], bit: u32) {
    bmap[(bit >> 3) as usize] |= 1 << (bit & 7);
}

/// Clear a bit in the bitmap
/// Parameter bmap: Bitmap array
/// Parameter bit: Bit index in the bitmap
pub fn ext4_bmap_bit_clr(bmap: &mut [u8], bit: u32) {
    bmap[(bit >> 3) as usize] &= !(1 << (bit & 7));
}

/// Find a free bit in the bitmap
/// Parameter bmap: Bitmap array
/// Parameter sbit: Start bit index
/// Parameter ebit: End bit index
/// Parameter bit_id: Reference to store the free bit index
pub fn ext4_bmap_bit_find_clr(bmap: &[u8], sbit: u32, ebit: u32, bit_id: &mut u32) -> bool {
    let mut i: u32;
    let mut bcnt = ebit - sbit;

    i = sbit;

    while i & 7 != 0 {
        if bcnt == 0 {
            return false;
        }

        if ext4_bmap_is_bit_clr(bmap, i) {
            *bit_id = i;
            return true;
        }

        i += 1;
        bcnt -= 1;
    }

    let mut byte_idx = (i >> 3) as usize;
    let mut bit_pos = i;
    
    while bcnt >= 8 {
        // 检查边界条件
        if byte_idx >= bmap.len() {
            return false;
        }
        
        if bmap[byte_idx] != 0xFF {
            for j in 0..8 {
                let bit_idx = bit_pos + j;
                if ext4_bmap_is_bit_clr(bmap, bit_idx) {
                    *bit_id = bit_idx;
                    return true;
                }
            }
        }

        byte_idx += 1;
        bcnt -= 8;
        bit_pos += 8;
    }

    while bcnt > 0 {
        if bit_pos >= ebit {
            return false;
        }
        
        if ext4_bmap_is_bit_clr(bmap, bit_pos) {
            *bit_id = bit_pos;
            return true;
        }
        
        bit_pos += 1;
        bcnt -= 1;
    }

    false
}

/// Clear a range of bits in the bitmap
/// Parameter bmap: Mutable reference to the bitmap array
/// Parameter start_bit: The start index of the bit range to clear
/// Parameter end_bit: The end index of the bit range to clear
pub fn ext4_bmap_bits_free(bmap: &mut [u8], start_bit: u32, end_bit: u32) {
    for bit in start_bit..=end_bit {
        ext4_bmap_bit_clr(bmap, bit);
    }
}