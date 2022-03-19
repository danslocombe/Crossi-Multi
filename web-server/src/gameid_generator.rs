use std::collections::HashSet;
use froggy_rand::FroggyRand;

use crate::GameId;

const WORDS_TXT : &'static str = include_str!("../wordlist.txt");

lazy_static! {
    static ref WORDS : Vec<&'static str> = WORDS_TXT.lines().collect();
}

pub struct GameIdGenerator
{
    froggy_rand : FroggyRand,
    i : usize,
    used : HashSet<GameId>,
}

impl GameIdGenerator
{
    pub fn new() -> Self {
        Self {
            froggy_rand : FroggyRand::new(0),
            i : 0,
            used : Default::default(),
        }
    }

    pub fn next(&mut self) -> GameId {
        loop {
            self.i += 1;
            let w0 = self.froggy_rand.gen_usize_range((self.i, 0), 0, WORDS.len());
            let w1 = self.froggy_rand.gen_usize_range((self.i, 1), 0, WORDS.len());
            let w2 = self.froggy_rand.gen_usize_range((self.i, 2), 0, WORDS.len());

            let generated = GameId(format!("{}_{}_{}", WORDS[w0], WORDS[w1], WORDS[w2]));

            if (!self.used.contains(&generated))
            {
                self.used.insert(generated.clone());
                return generated;
            }
        }
    }
}
