use std::hash::{Hash, Hasher};
use std::num::Wrapping;

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

fn hash<T : Hash>(x : T) -> u64 {
    // TODO jenkins hasher chosen as it gives deterministic results across wasm/x64
    // Look at others.
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

    pub fn gen<T : Hash>(&self, x : T) -> u64 {
        let index = (Wrapping(self.seed) + Wrapping(hash(x))).0;
        split_mix_64(index)
    }

    pub fn gen_unit<T : Hash>(&self, x : T) -> f64 {
        // Should be enough precision for a game
        (self.gen(x) % 1_000_000) as f64 / 1_000_000.0
    }

    pub fn gen_range<T : Hash>(&self, x : T, min : f64, max : f64) -> f64 {
        min + self.gen_unit(x) * (max - min)
    }

    pub fn choose<'a, T : Hash, X>(&self, x : T, choices : &'a [X]) -> &'a X {
        let i = self.gen(x) as usize % choices.len();
        &choices[i]
    }
}