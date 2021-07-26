use rand_xorshift::XorShiftRng;
use rand_core::{RngCore, SeedableRng};

pub struct FroggyRng {
    rng : XorShiftRng,
}

impl FroggyRng {
    pub fn new(seed : u32) -> Self {
        let mut bytes : [u8; 16] = [0;16];
        let transmuted = unsafe { std::mem::transmute::<u32, [u8; 4]>(seed) };
        
        for i in 0..4 {
            let ibase = i * 4;
            bytes[ibase]     = transmuted[0];
            bytes[ibase + 1] = transmuted[1];
            bytes[ibase + 2] = transmuted[2];
            bytes[ibase + 3] = transmuted[3];
        }

        FroggyRng {
            rng: XorShiftRng::from_seed(bytes)
        }
    }

    pub fn next(&mut self) -> f64 {
        (self.rng.next_u32() as f64) / (u32::MAX as f64)
    }

    pub fn next_range(&mut self, min : f64, max : f64) -> f64 {
        min + self.next() * (max - min)
    }
}