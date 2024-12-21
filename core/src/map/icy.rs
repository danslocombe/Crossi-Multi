use std::collections::{BTreeSet, VecDeque};

use froggy_rand::FroggyRand;
use num_traits::ops::inv;
use serde::{Deserialize, Serialize};
use crate::{bitmap::BitMap, map::RowType, CoordPos, Input, SCREEN_SIZE};

use super::{PathDescr, Row, RowId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcyDescr {
    pub path_descr : PathDescr,
    pub seed : u32,
    pub y : i32,
    pub blocks: BitMap,
}

pub fn gen_icy_section(rand: FroggyRand, row_id_0: RowId, rows: &mut VecDeque<Row>) {
    // Icy
    let width = *rand.choose("ice_len", &[3, 5, 6, 7, 8]);
    //println!("Icy {} width {}", row_id_0.to_y(), width);
    for i in 0..width {
        let rid = RowId(row_id_0.0 + i);
        let y = rid.to_y();
        let seed = rand.gen("icy_seed") as u32;
        let blocks = BitMap::default();
        rows.push_front(Row {
            row_id: rid,
            row_type: RowType::IcyRow(IcyDescr {
                path_descr: PathDescr {
                    // @TODO, do properly
                    wall_width: 4,
                },
                seed,
                y,
                blocks,
            }),
        });
    }
}

/*
impl IcyDescr {
    pub fn hydrate(&self) -> HydratedIcyRow {
        //let mut ice = Vec::new();
        //let mut blocks = Vec::new();
        let mut blocks = bitmaps::Bitmap::new();
        let rng = FroggyRand::from_hash((self.y, self.seed));

        for x in 0..SCREEN_SIZE {
            if (x <= self.path_descr.wall_width as i32 || x >= (SCREEN_SIZE - self.path_descr.wall_width as i32 - 1)) {
                continue;
            }

            //if (rng.gen_unit((x, 1)) < 0.45) {
                //ice.push(x);
            //}
            if (rng.gen_unit((x, 1)) < 0.45) {
                blocks.set(x as usize, true);
            }
        }

        HydratedIcyRow {
            //ice,
            blocks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydratedIcyRow {
    pub blocks : bitmaps::Bitmap<20>,
    //pub ice : Vec<i32>,
    //pub blocks : Vec<i32>,
}
    */

pub struct BlockMap {
    inner: Vec<BitMap>,
    full_width: i32,
    wall_width: i32,
}

impl BlockMap {
    #[inline]
    pub fn set(&mut self, pos: CoordPos, val: bool) {
        self.inner[pos.y as usize].set(pos.x, val)
    }

    #[inline]
    fn in_bounds(&self, pos: CoordPos) -> bool {
        pos.x >= self.wall_width
            && pos.x < self.full_width - self.wall_width
            && pos.y >= 0
            && pos.y < self.height()
    }

    pub fn get(&self, pos: CoordPos) -> bool {
        if !self.in_bounds(pos) {
            return true;
        }

        self.inner[pos.y as usize].get(pos.x)
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.inner.len() as i32
    }
}

pub fn verify_ice(block_map: &BlockMap) -> bool {
    let mut seen: BTreeSet<i32> = BTreeSet::new();

    // Fill nodes with end positions.
    let mut nodes: Vec<PosWithDir> = Vec::with_capacity(64);
    for x in 0..block_map.full_width {
        //let pos = CoordPos::new(x, block_map.height() - 1);
        let pos = CoordPos::new(x, 0);
        if (block_map.get(pos)) {
            continue;
        }

        let node = PosWithDir {
            pos,
            dir: Input::Up,
        };

        seen.insert(node.to_i32());
        nodes.push(node);
    }

    let mut timeout = 8;
    let mut iter = 0;
    while timeout > 0 {
        //println!("At Iter {}", iter);
        //for node in &nodes {
        //    println!("{:?}", node);
        //}
        ////println!("---------");
        timeout -= 1;
        iter += 1;

        let mut projected_positions: Vec<(CoordPos, Input)> = Vec::new();
        for node in &nodes {
            // Project backwards
            let inverted_dir = node.dir.invert();
            let mut pos = node.pos;
            loop {
                pos = pos.apply_input(inverted_dir);
                //println!("Projecting {:?} forming {:?}", node, pos);
                if (block_map.get(pos)) {
                    //println!("Hit {:?}", pos);
                    break;
                }

                if (inverted_dir == Input::Down && pos.y == block_map.height() - 1) {
                    // We made it!
                    //panic!("aa nodes {} seen {} iter {}", nodes.len(), seen.len(), iter);
                    return true;
                }

                projected_positions.push((pos, inverted_dir));
            }
        }

        // Filter valid
        nodes.clear();
        for (pos, inverted_dir) in projected_positions {
            let orth_0 = inverted_dir.orthogonal();

            let both_orth = [orth_0, orth_0.invert()];
            for orth in both_orth {
                let one_more = pos.apply_input(orth);

                let collision = block_map.get(one_more);
                if (!collision) {
                    continue;
                }

                let node = PosWithDir {
                    pos,
                    dir: orth,
                };

                if (!seen.contains(&node.to_i32())) {
                    seen.insert(node.to_i32());
                    nodes.push(node);
                }
            }
        }
    }

    debug_log!("Err: Verify Ice Timeout");
    //panic!("aa nodes {} seen {} iter {}", nodes.len(), seen.len(), iter);
    false
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosWithDir {
    pos: CoordPos,
    dir: Input,
}

impl PosWithDir {
    pub fn to_i32(self) -> i32 {
        debug_assert!(self.dir != Input::None);

        let dir_i: i32 = match self.dir {
            Input::Up => 0,
            Input::Down => 1,
            Input::Left => 2,
            Input::Right => 3,
            _ => panic!(),
        };

        dir_i + (self.pos.x + self.pos.y * 20) * 4
    }

    pub fn from_i32(mut val: i32) -> Self {
        let dir_i = val % 4;
        val = val / 4;
        let x = val % 20;
        let y = val / 20;

        let dir = match dir_i {
            0 => Input::Up,
            1 => Input::Down,
            2 => Input::Left,
            3 => Input::Right,
            _ => unreachable!()
        };

        Self {
            pos: CoordPos::new(x, y),
            dir
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pos_with_dir_roundtrip() {
        let pos_with_dir = PosWithDir {
            pos: CoordPos::new(12, 2),
            dir: Input::Left,
        };

        let serialized = pos_with_dir.to_i32();
        assert_eq!(PosWithDir::from_i32(serialized), pos_with_dir);
    }

    #[test]
    fn test_points() {
        let rows = [
            "X XXX",
        ];

        let map = generate_map(&rows);
        assert_eq!(map.full_width, 7);
        assert_eq!(map.wall_width, 1);
        assert!(map.get(CoordPos::new(0, 0)));
        assert!(map.get(CoordPos::new(1, 0)));
        assert!(!map.get(CoordPos::new(2, 0)));
        assert!(map.get(CoordPos::new(3, 0)));
        assert!(map.get(CoordPos::new(4, 0)));
        assert!(map.get(CoordPos::new(5, 0)));
        assert!(map.get(CoordPos::new(6, 0)));
    }

    #[test]
    fn icy_verification_trivial() {
        let rows = [
            "X  X",
            "X  X",
        ];

        let map = generate_map(&rows);
        assert!(verify_ice(&map));
    }

    #[test]
    fn icy_verification_very_simple() {
        let rows = [
            "XX X",
            "X  X",
            "X XX",
        ];

        let map = generate_map(&rows);
        assert!(verify_ice(&map));
    }

    #[test]
    fn icy_verification_simple_positive() {
        let rows = [
            "XX XX",
            "X   X",
            "X XXX",
        ];

        let map = generate_map(&rows);
        assert!(!verify_ice(&map));
    }

    #[test]
    fn icy_verification_simple_negative() {
        let rows = [
            "XX XX",
            "X   X",
            "X XXX",
        ];

        let map = generate_map(&rows);
        assert!(!verify_ice(&map));
    }

    #[test]
    fn icy_verification_positive() {
        let rows = [
            "xxx   xxxx",
            "x    x   x",
            "x  x x   x",
            "x  x     x",
            "x  xxxx  x",
            "x        x",
        ];

        let map = generate_map(&rows);
        assert!(verify_ice(&map));
    }


    #[test]
    fn icy_verification_negative() {
        let rows = [
            "xxx   xxxx",
            "x        x",
            "x        x",
            "x        x",
            "x  xxxx  x",
            "x        x",
        ];

        let map = generate_map(&rows);
        assert!(verify_ice(&map));
    }


    fn generate_map(rows: &[&str]) -> BlockMap {
        let full_width = (rows[0].len() + 2) as i32;
        let wall_width = 1;

        let mut inner = Vec::new();

        for row in rows {
            let mut row_map = BitMap::default();
            for (i, c) in row.chars().enumerate() {
                if c == 'X' {
                    row_map.set_bit(i as i32 + wall_width);
                }
            }

            inner.push(row_map)
        }

        BlockMap {
            full_width,
            wall_width,
            inner
        }
    }
}