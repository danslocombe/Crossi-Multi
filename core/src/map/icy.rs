use std::{collections::{BTreeMap, BTreeSet, VecDeque}, num::Wrapping, time::Instant};

use froggy_rand::FroggyRand;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use crate::{bitmap::BitMap, map::RowType, CoordPos, Input, ALL_INPUTS, SCREEN_SIZE};

use super::{PathDescr, Row, RowId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcyDescr {
    pub path_descr : PathDescr,
    pub seed : u32,
    pub y : i32,
    pub blocks: BitMap,
}

pub fn try_gen_icy_section(rand: FroggyRand, row_id_0: RowId, rows: &mut VecDeque<Row>) -> bool {
    // Icy
    println!("Icy seed = {}", rand.get_seed());
    let start = Instant::now();
    let height = *rand.choose("ice_len", &[5, 7, 7, 9, 9, 13]);

    'outer: for i in 0..256 {
        let rand = rand.subrand(i);
        let mut map = generate_ice_single(rand, 20, 4, height);

        for j in 0..8 {
            //if (verify_ice(&map)) {
            match (verify_ice_graph(&map)) {
                VerifyResult::Bad_Zork => {
                    continue 'outer;
                },
                VerifyResult::Bad_DoesntReachEnd => {
                    // Remove things.
                    for y in 0..height {
                        for x in 0..map.full_width {
                            let pos = CoordPos::new(x, y);
                            //if rand.gen_unit(("remove", j, pos)) < 0.15 {
                            if gen_unit_perf(rand, j * 1024 + pos.x + pos.y * 128) < 0.15 {
                                map.inner[y as usize].unset_bit(x);
                            }
                        }
                    }
                    continue;
                },
                VerifyResult::Bad_Trivial => {
                    // Add things
                    for y in 0..height {
                        for x in 0..map.full_width {
                            let pos = CoordPos::new(x, y);
                            if gen_unit_perf(rand, 1 + j * 1024 + pos.x + pos.y * 128) < 0.15 {
                                map.inner[y as usize].set_bit(x);
                            }
                        }
                    }
                    continue;
                },
                VerifyResult::Success => {
                    // Got a map!
                    println!("Verified an icy section of height {}, y = {}", height, row_id_0.to_y());

                    for row in 0..height {
                        let rid = RowId(row_id_0.0 + row as u32);
                        let y = rid.to_y();
                        let seed = rand.gen("icy_seed") as u32;
                        let blocks = map.inner[row as usize];
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

                    // Success
                    let time = start.elapsed();
                    println!("Generated in {}ms", time.as_millis());
                    return true;
                }
            }
        }
    }

    // Timed out of iterations trying to find a valid map :(
    false
}

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
        pos.x > self.wall_width
            && pos.x < self.full_width - 1 - self.wall_width
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
    false
}

fn generate_ice_single(rand: FroggyRand, full_width: i32, wall_width: i32, height: i32) -> BlockMap {
    let mut inner = Vec::new();

    for y in 0..height {
        let mut row_map = BitMap::default();
        for x in 0..full_width {
            if x < wall_width || x > full_width - wall_width {
                continue;
            }

            let pos = CoordPos::new(x, y);
            if rand.gen_unit(pos) < 0.6 {
                row_map.set_bit(x);
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

pub fn build_graph(block_map: &BlockMap) -> IcyGraph {
    let mut graph = IcyGraph::default();

    for x in 0..block_map.full_width {
        let input = Input::Up;
        let mut pos = CoordPos::new(x, block_map.height() - 1);
        let mut prev = None;

        loop {
            if pos.y < 0 {
                // Outside
                // Can go from start to end
                graph.add_edge(Node::start(), Node::end());
                break;
            }
            if block_map.get(pos) {
                if let Some(p) = prev {
                    // Hit something and last position was non-empty
                    // Add a link
                    graph.add_edge(Node::start(), Node::pos(p));
                }

                break;
            }

            prev = Some(pos);
            pos = pos.apply_input(input);
        }
    }

    // @Dedup with above
    for x in 0..block_map.full_width {
        let input = Input::Down;
        let mut pos = CoordPos::new(x, 0);
        let mut prev = None;

        loop {
            if pos.y > block_map.height() - 1 {
                // Outside
                // Can go from start to end
                graph.add_edge(Node::end(), Node::start());
                break;
            }
            if block_map.get(pos) {
                if let Some(p) = prev {
                    // Hit something and last position was non-empty
                    // Add a link
                    //let node = graph.get_or_add_node(NodeType::Pos(p));
                    graph.add_edge(Node::end(), Node::pos(p));
                }

                break;
            }

            prev = Some(pos);
            pos = pos.apply_input(input);
        }
    }

    for y in 0..block_map.height() {
        for x in 0..block_map.full_width {
            let pos = CoordPos::new(x, y);
            if block_map.get(pos) {
                continue;
            }

            //let node = graph.get_or_add_node(NodeType::Pos(pos));
            for dir in ALL_INPUTS {
                let mut p = pos;
                loop {
                    let last = p;
                    p = p.apply_input(dir);
                    if (p.y < 0) {
                        // Hit the end
                        graph.add_edge(Node::pos(pos), Node::end());
                        break;
                    }
                    if (p.y == block_map.height()) {
                        // Hit the start
                        graph.add_edge(Node::pos(pos), Node::start());
                        break;
                    }

                    if (block_map.get(p)) {
                        //let last_id = graph.get_or_add_node(NodeType::Pos(last));
                        graph.add_edge(Node::pos(pos), Node::pos(last));
                        break;
                    }
                }
            }
        }
    }

    graph
}

#[derive(Default, Debug)]
pub struct IcyGraph {
    edges: BTreeMap<Node, smallvec::SmallVec<[Node; 4]>>,
}

impl IcyGraph {
    pub fn add_edge(&mut self, from: Node, to: Node) {
        // Don't allow self edges
        // Add check here for cleaner upstream
        if from == to {
            return;
        }

        if let Some(existing) = self.edges.get_mut(&from) {
            existing.push(to);
        }
        else {
            let mut v = SmallVec::new();
            v.push(to);
            self.edges.insert(from, v);
        }
    }

    pub fn mark_forward_from_start(&self) -> BTreeSet<Node> {
        self.mark_forward_from_start_debug(false)
    }

    pub fn mark_forward_from_start_debug(&self, debug: bool) -> BTreeSet<Node> {
        let mut marked = BTreeSet::new();
        marked.insert(Node::start());

        let mut wavefront = vec![Node::start()];
        if (debug) {
            println!("Hello!");
        }

        while (!wavefront.is_empty()) {
            if debug {
                println!("Iter: {:?}", wavefront);
            }
            // @Perf reuse vecs
            let mut new_wavefront = Vec::new();

            for nid in &wavefront {
                if let Some(edges) = self.edges.get(nid) {
                    for e in edges {
                        if (marked.insert(*e)) {
                            if (debug) {
                                println!("Found edges {:?}->{:?}", nid, e);
                            }
                            new_wavefront.push(*e);
                        }
                    }
                }
            }

            wavefront = new_wavefront;
        }

        marked
    }

    pub fn unmark_inverted_from_start(&self, marked: &mut BTreeSet<Node>) {
        marked.remove(&Node::start());
        marked.remove(&Node::end());

        let mut wavefront = vec![Node::start()];

        while (!wavefront.is_empty()) {
            // @Perf reuse vecs
            let mut new_wavefront = Vec::new();

            for nid in &wavefront {
                for (from, edge_set) in &self.edges {
                    for to in edge_set {
                        if *to != *nid {
                            continue;
                        }

                        if (marked.remove(from)) {
                            new_wavefront.push(*from);
                        }
                    }
                }
            }

            wavefront = new_wavefront;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeType {
    Start,
    End,
    Pos(CoordPos),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    inner: NodeType,
}

impl Node {
    pub fn new(inner: NodeType) -> Self {
        Self {
            inner,
        }
    }

    pub fn start() -> Self {
        Self::new(NodeType::Start)
    }

    pub fn end() -> Self {
        Self::new(NodeType::End)
    }

    pub fn pos(pos: CoordPos) -> Self {
        Self::new(NodeType::Pos(pos))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    from: Node,
    to: Node,
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

pub fn verify_ice_graph(block_map: &BlockMap) -> VerifyResult {
    let graph = build_graph(&block_map);

    if let Some(edges) = graph.edges.get(&Node::start()) {
        if (edges.contains(&Node::end())) {
            return VerifyResult::Bad_Trivial;
        }
    }

    let mut marked = graph.mark_forward_from_start();
    if !marked.contains(&Node::end()) {
        // Didnt reach end
        return VerifyResult::Bad_DoesntReachEnd;
    }

    graph.unmark_inverted_from_start(&mut marked);
    if (!marked.is_empty()) {
        VerifyResult::Bad_Zork
    }
    else {
        VerifyResult::Success
    }
}

pub enum VerifyResult {
    Success,
    Bad_Trivial,
    Bad_DoesntReachEnd,
    Bad_Zork,
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
    fn icy_verification_small_negative() {
        let rows = [
            "xx  xx",
            "x    x",
            "x xx x",
            "x    x",
        ];

        let map = generate_map(&rows);
        assert!(!verify_ice(&map));
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
        assert!(!verify_ice(&map));
    }

    #[test]
    fn icy_graph_trivial() {
        let rows = [
            "X X",
            "X X",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);
        println!("{:#?}", graph);
        //assert_eq!(graph.nodes, Vec::default());
        let marked = graph.mark_forward_from_start();
        assert!(marked.contains(&Node::end()));
        //assert!(graph.end().unwrap().1.mark)
    }

    #[test]
    fn icy_graph_simple() {
        let rows = [
            "xX x",
            "x  x",
        ];

        let map = generate_map(&rows);
        let graph = build_graph(&map);
        //println!("{:#?}", graph);
        let mut reachable = graph.mark_forward_from_start();

        assert!(reachable.contains(&Node::end()));
        //println!("{:#?}", graph);

        assert_eq!(2 + 2, reachable.len());

        graph.unmark_inverted_from_start(&mut reachable);
        println!("{:#?}", graph);
        assert!(reachable.is_empty());
    }

    #[test]
    fn icy_graph_zork() {
        let rows = [
            "xxx xX",
            "x    X",
            "xx  xX",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);
        //println!("{:#?}", graph);

        let mut marked = graph.mark_forward_from_start();
        assert!(marked.contains(&Node::end()));
        graph.unmark_inverted_from_start(&mut marked);

        println!("{:#?}", graph);
        assert_eq!(2, marked.len());

        // Expect 3 states zork, marked with Z

        //  "xx x",
        //  "ZZ Z",
        //  "x  x",
    }

    #[test]
    fn icy_graph_positive() {
        let rows = [
            "xxx   xxxx",
            "x    x   x",
            "x  x x   x",
            "x  x     x",
            "x  xxxx  x",
            "x        x",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);

        let mut marked = graph.mark_forward_from_start();
        assert!(marked.contains(&Node::end()));
        graph.unmark_inverted_from_start(&mut marked);
        assert!(marked.is_empty());
    }

    #[test]
    fn icy_graph_negative() {
        let rows = [
            "xx  xxx",
            "x     x",
            "x xx  x",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);
        let marked = graph.mark_forward_from_start();
        //println!("{:#?}", graph);
        assert!(!marked.contains(&Node::end()));
    }

    //#[test]
    fn test_harness() {
        let rand = FroggyRand::new(12375972415461437779);
        let mut rows = Default::default();
        try_gen_icy_section(rand, RowId::from_y(0), &mut rows);

        for row in rows.iter() {
            if let RowType::IcyRow(descr) = &row.row_type {
                for x in 0..20 {
                    if x < descr.path_descr.wall_width {
                        print!("X");
                    }
                    else if descr.blocks.get(x as i32) {
                        print!("X");
                    }
                    else {
                        print!(" ");
                    }
                }

                println!("");
            }
        }

        assert!(false);
    }

    fn generate_map(rows: &[&str]) -> BlockMap {
        let full_width = (rows[0].len() + 2) as i32;
        let wall_width = 1;

        let mut inner = Vec::new();

        for row in rows {
            let mut row_map = BitMap::default();
            for (i, c) in row.chars().enumerate() {
                if c == 'X' || c == 'x' {
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

fn gen_perf(rand: FroggyRand, seed: i32) -> u64 {
    let index = (Wrapping(rand.get_seed()) + Wrapping(seed as u64)).0;
    split_mix_64(index)
}

fn gen_unit_perf(rand: FroggyRand, seed: i32) -> f32 {
    (gen_perf(rand, seed) % 1_000_000) as f32 / 1_000_000.0
}

fn split_mix_64(index : u64) -> u64 {
    let mut z = Wrapping(index) + Wrapping(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)) * Wrapping(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * Wrapping(0x94D049BB133111EB);
    (z ^ (z >> 31)).0
}