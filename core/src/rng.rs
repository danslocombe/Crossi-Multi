use std::hash::{Hash, Hasher};
use std::num::Wrapping;
use std::fmt::Debug;

/// Take insperation from 
/// https://www.youtube.com/watch?v=e4b--cyXEsM
///
/// We want everything to be reproducable so add seeded rng
///
/// However if we use rngs with mutating internal state we are bound by the order
/// of generated numbers.
///
/// Instead we base our rng on SplitMix64 where input is an index into a sequence
/// Then we can hash values to produce an index into the sequence.
/// As long as the hash function is stable and the inputs to the hash are stable results will be reproducable
/// regardless of rng ordering.
#[derive(Debug, Clone)]
pub struct FroggyRng {
    seed : u64,
}

fn split_mix_64(index : u64) -> u64 {
    let mut z = Wrapping(index) + Wrapping(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)) * Wrapping(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * Wrapping(0x94D049BB133111EB);
    (z ^ (z >> 31)).0
}

#[inline]
fn hash<T : Hash>(x : T) -> u64 {
    //let mut hasher = deterministic_hash::DeterministicHasher::new(FroggyHash::new());
    let mut hasher = deterministic_hash::DeterministicHasher::new(hashers::jenkins::Lookup3Hasher::default());
    x.hash(&mut hasher);
    hasher.finish()
}

impl FroggyRng {
    pub fn new(seed : u64) -> Self {
        Self {seed}
    }

    pub fn from_hash<T : Hash>(x : T) -> Self {
        Self::new(hash(x))
    }

    pub fn gen<T : Hash + Debug>(&self, x : T) -> u64 {
        //debug_log!("Generating from {:?} + seed {}", x, self.seed);
        let hash = hash(x);
        let index = (Wrapping(self.seed) + Wrapping(hash)).0;
        let res = split_mix_64(index);
        //debug_log!("Generated={}", res);
        res
    }

    pub fn gen_unit<T : Hash + Debug>(&self, x : T) -> f64 {
        // Should be enough precision for a game
        (self.gen(x) % 1_000_000) as f64 / 1_000_000.0
    }

    pub fn gen_range<T : Hash + Debug>(&self, x : T, min : f64, max : f64) -> f64 {
        min + self.gen_unit(x) * (max - min)
    }

    pub fn choose<'a, T : Hash + Debug, X>(&self, x : T, choices : &'a [X]) -> &'a X {
        // usize can be aliased to u32 or u64 in wasm based on the compilation
        // for safety we restrict to u32 range.
        let index = self.gen(x) as u64 % u32::MAX as u64;
        let i = index as usize % choices.len();
        &choices[i]
    }

    // I dont know what a statistic is
    pub fn gen_froggy<T : Hash + Debug>(&self, x : T, min : f64, max : f64, n : u32) -> f64 {
        let mut sum = 0.;
        let gen_min = min / n as f64;
        let gen_max = max / n as f64;

        for i in 0..n {
            sum += self.gen_range((&x, i), gen_min, gen_max);
        }

        sum
    }
}

// We don't need a smart hash as this is just used as an input to splitmix.
// splitmix generates outputs in a uniform distribution, just need something 
// platform independent that gives varied results based on input order.
struct FroggyHash {
    value : Wrapping<u64>,
}

impl FroggyHash {
    #[inline]
    fn new() -> Self {
        Self {
            value : Wrapping(12345674357),
        }
    }
}

impl Hasher for FroggyHash {
    #[inline]
    fn finish(&self) -> u64 {
        self.value.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for x in bytes {
            self.value = Wrapping(self.value.0.rotate_left(1));
            self.value += Wrapping(*x as u64);
        }
    }
}