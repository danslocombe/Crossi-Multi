use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct BitMap {
    pub inner: u64, 
}

impl BitMap {
    #[inline]
    fn get_mask(i : u64) -> u64 {
        0x01 << i
    }

    #[inline]
    pub fn get(self, i: i32) -> bool {
        //debug_assert!(i >= 0 && i < 64);

        let mask = Self::get_mask(i as u64);
        self.inner & mask != 0
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

    #[inline]
    pub fn set_bit(&mut self, i: i32) {
        //debug_assert!(i >= 0 && i < 64);

        let mask = Self::get_mask(i as u64);
        self.inner = self.inner | mask;
    }

    #[inline]
    pub fn unset_bit(&mut self, i: i32) {
        //debug_assert!(i >= 0 && i < 64);

        let mask = Self::get_mask(i as u64);
        self.inner = self.inner & (self.inner ^ mask);
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
                if i != 0 {
                    assert!(!map.get(i), "Failed on {}", i);
                }
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