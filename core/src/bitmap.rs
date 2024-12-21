use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct BitMap {
    pub inner: u64, 
}

impl BitMap {
    fn get_mask(i : u8) -> u8 {
        0x01 << i
    }

    pub fn get(self, i: i32) -> bool {
        debug_assert!(i >= 0 && i < 64);
        let bytes = unsafe { std::mem::transmute::<u64, [u8;8]>(self.inner) };
        let byte = bytes[i as usize / 8];
        let mask = Self::get_mask((i % 8) as u8);

        byte & mask != 0
    }

    #[inline]
    pub fn set(&mut self, i: i32, value: bool) {
        if (value) {
            self.set_bit(i);
        }
        else {
            self.unset_bit(i);
        }
    }

    pub fn set_bit(&mut self, i: i32) {
        debug_assert!(i >= 0 && i < 64);
        let bytes = unsafe { std::mem::transmute::<&mut u64, &mut [u8;8]>(&mut self.inner) };
        let byte_index = i as usize / 8;
        let mask = Self::get_mask((i % 8) as u8);
        bytes[byte_index] = bytes[byte_index] | mask;
    }

    pub fn unset_bit(&mut self, i: i32) {
        debug_assert!(i >= 0 && i < 64);
        let bytes = unsafe { std::mem::transmute::<&mut u64, &mut [u8;8]>(&mut self.inner) };
        let byte_index = i as usize / 8;
        let mask = Self::get_mask((i % 8) as u8);
        bytes[byte_index] = bytes[byte_index] & (bytes[byte_index] ^ mask);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set() {
        let mut map = BitMap::default();
        for i in 0..64 {
            assert!(!map.get(i));
        }

        map.set(32, true);
        map.set(33, true);
        map.set(12, true);
        map.set(8, true);

        for i in 0..64 {
            if i == 32 || i == 33 || i == 12 || i == 8 {
                assert!(map.get(i));
            }
            else {
                assert!(!map.get(i));
            }
        }


        map.set(32, false);
        map.set(33, false);
        map.set(12, false);
        map.set(8, false);

        for i in 0..64 {
            assert!(!map.get(i));
        }

        assert_eq!(map.inner, 0);
    }
}