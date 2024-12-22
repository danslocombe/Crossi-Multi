use std::collections::{BTreeSet, VecDeque};

use froggy_rand::FroggyRand;
use num_traits::ops::inv;
use serde::{Deserialize, Serialize};
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
                            if rand.gen_unit(("remove", j, pos)) < 0.15 {
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
                            if rand.gen_unit(("add", j, pos)) < 0.15 {
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
                    return true;
                }
            }
        }
    }

    // Timed out of iterations trying to find a valid map :(
    false
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
    let start = graph.get_or_add_node(NodeType::Start);
    let end = graph.get_or_add_node(NodeType::End);

    for x in 0..block_map.full_width {
        let input = Input::Up;
        let mut pos = CoordPos::new(x, block_map.height() - 1);
        let mut prev = None;

        loop {
            if pos.y < 0 {
                // Outside
                // Can go from start to end
                graph.add_edge(start, end);
                break;
            }
            if block_map.get(pos) {
                if let Some(p) = prev {
                    // Hit something and last position was non-empty
                    // Add a link
                    let node = graph.get_or_add_node(NodeType::Pos(p));
                    graph.add_edge(start, node);
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
                graph.add_edge(end, start);
                break;
            }
            if block_map.get(pos) {
                if let Some(p) = prev {
                    // Hit something and last position was non-empty
                    // Add a link
                    let node = graph.get_or_add_node(NodeType::Pos(p));
                    graph.add_edge(end, node);
                }

                break;
            }

            prev = Some(pos);
            pos = pos.apply_input(input);
        }
    }

    /*
    for x in 0..block_map.full_width {
        let input = Input::Down;
        let pos = CoordPos::new(x, 0);
        let mut prev = None;

        while {
            pos = pos.apply_input(input);

            if !block_map.get(pos) {
                if let Some(p) = last {
                    // Hit something and last position was non-empty
                    // Add a link
                    let node = graph.get_or_add_node(NodeType::Pos(p));
                    graph.add_edge(end, node);
                }
            }
            prev = Some(pos);
        }
    }
    */

    for y in 0..block_map.height() {
        for x in 0..block_map.full_width {
            let pos = CoordPos::new(x, y);
            if block_map.get(pos) {
                continue;
            }

            let node = graph.get_or_add_node(NodeType::Pos(pos));
            for dir in ALL_INPUTS {
                let mut p = pos;
                loop {
                    let last = p;
                    p = p.apply_input(dir);
                    if (p.y < 0) {
                        // Hit the end
                        graph.add_edge(node, end);
                        break;
                    }
                    if (p.y == block_map.height()) {
                        // Hit the start
                        graph.add_edge(node, start);
                        break;
                    }

                    if (block_map.get(p)) {
                        let last_id = graph.get_or_add_node(NodeType::Pos(last));
                        graph.add_edge(node, last_id);
                        break;
                    }
                }
            }
        }
    }

    println!("Built graph nodes {} edges {}, w {} h {}", graph.nodes.len(), graph.edges.len(), block_map.full_width - 2*block_map.wall_width, block_map.inner.len());
    graph
}

#[derive(Default, Debug)]
pub struct IcyGraph {
    nodes: Vec<Node>,
    edges: BTreeSet<Edge>,
}

impl IcyGraph {
    pub fn try_get_node(&mut self, node: NodeType) -> Option<usize> {
        self.nodes.iter().enumerate().filter(|(_, x)| x.inner == node).next().map(|(i, _)| i)
    }

    pub fn get_or_add_node(&mut self, node: NodeType) -> usize {
        if let Some(n) = self.try_get_node(node) {
            n
        }
        else {
            let id = self.nodes.len();
            self.nodes.push(Node::new(node));
            id
        }
    }

    /*
    pub fn add_node(&mut self, node: Node) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }
    */

    pub fn add_edge(&mut self, from: usize, to: usize) {
        // Don't allow self edges
        // Add check here for cleaner upstream
        if from == to {
            return;
        }
        let edge = Edge {
            from,
            to
        };

        self.edges.insert(edge);
    }

    pub fn clear_marks(&mut self) {
        for node in &mut self.nodes {
            node.mark = false;
        }
    }

    pub fn get_marked(&self) -> Vec<Node> {
        self.nodes.iter().filter(|x| x.mark).cloned().collect()
    }

    pub fn start(&self) -> Option<(usize, Node)> {
        // @Cleanup use matches!
        self.nodes.iter().enumerate().find(|(_, x)| if let NodeType::Start = x.inner {true} else {false}).map(|(i, x)| (i, *x))
    }

    pub fn end(&self) -> Option<(usize, Node)> {
        self.nodes.iter().enumerate().find(|(_, x)| if let NodeType::End = x.inner {true} else {false}).map(|(i, x)| (i, *x))
    }

    pub fn mark_forward_from_start(&mut self) {
        let start = self.start().unwrap().0;
        self.nodes[start].mark = true;

        let mut wavefront = vec![start];

        //println!("self: {:#?}", self);
        while (!wavefront.is_empty()) {
            //println!("Iter: {:?}", wavefront);
            // @Perf reuse vecs
            let mut new_wavefront = Vec::new();

            for nid in &wavefront {
                for edge in &self.edges {
                    if edge.from != *nid {
                        continue;
                    }
                    //println!("Found edge: {:?}", edge);
                    //println!("Found edge: {:?} -> {:?})", self.nodes[edge.from], self.nodes[edge.to]);

                    let node = &mut self.nodes[edge.to];
                    if !node.mark {
                        node.mark = true;
                        new_wavefront.push(edge.to);
                    }
                }
            }

            wavefront = new_wavefront;
        }
    }

    pub fn unmark_inverted_from_start(&mut self) {
        //println!("self: {:#?}", self);
        //println!("STARTING UNMARK =====");

        let start = self.start().unwrap().0;
        self.nodes[start].mark = false;

        let end = self.end().unwrap().0;
        self.nodes[end].mark = false;

        let mut wavefront = vec![start];

        while (!wavefront.is_empty()) {
            //println!("Iter: {:?}", wavefront);
            // @Perf reuse vecs
            let mut new_wavefront = Vec::new();

            for nid in &wavefront {
                for edge in &self.edges {
                    if edge.to != *nid {
                        continue;
                    }
                    //println!("Found edge: {:?}", edge);

                    let node = &mut self.nodes[edge.from];
                    if node.mark {
                        node.mark = false;
                        new_wavefront.push(edge.from);
                    }
                }
            }

            wavefront = new_wavefront;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Start,
    End,
    Pos(CoordPos),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    inner: NodeType,
    mark: bool,
}

impl Node {
    pub fn new(inner: NodeType) -> Self {
        Self {
            inner,
            mark: false,
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
    from: usize,
    to: usize,
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
    let mut graph = build_graph(&block_map);
    graph.mark_forward_from_start();
    if (!graph.end().unwrap().1.mark) {
        // Didnt reach end
        println!("Doesnt reach end");
        return VerifyResult::Bad_DoesntReachEnd;
    }

    let start_i = graph.start().unwrap().0;
    let end_i = graph.end().unwrap().0;
    if (graph.edges.contains(&Edge {from: start_i, to: end_i})) {
        // Temp if you can directly go then the generated ice is not
        // interesting.
        println!("Trivial");
        return VerifyResult::Bad_Trivial;
    }

    graph.unmark_inverted_from_start();
    let marked = graph.get_marked();
    if (!marked.is_empty()) {
        println!("Unreachable");
        //println!("Unreachable {:?}", marked);
        VerifyResult::Bad_Zork
    }
    else {
        VerifyResult::Success
    }
}

enum VerifyResult {
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
        graph.mark_forward_from_start();
        assert!(graph.end().unwrap().1.mark)
    }

    #[test]
    fn icy_graph_simple() {
        let rows = [
            "X ",
            "  ",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);
        //println!("{:#?}", graph);
        graph.mark_forward_from_start();
        let reachable = graph.get_marked();

        //println!("{:#?}", graph);

        assert_eq!(2 + 2, reachable.len());

        graph.unmark_inverted_from_start();
        let reachable = graph.get_marked();
        println!("{:#?}", graph);
        assert!(reachable.is_empty());
    }

    #[test]
    fn icy_graph_zork() {
        let rows = [
            "xx x",
            "    ",
            "x  x",
        ];

        let map = generate_map(&rows);
        let mut graph = build_graph(&map);
        //println!("{:#?}", graph);
        graph.mark_forward_from_start();
        let _reachable = graph.get_marked();
        graph.unmark_inverted_from_start();
        let zork_states = graph.get_marked();

        println!("{:#?}", graph);
        assert_eq!(2, zork_states.len());

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
        graph.mark_forward_from_start();
        assert!(graph.end().unwrap().1.mark);

        graph.unmark_inverted_from_start();
        let reachable = graph.get_marked();
        assert!(reachable.is_empty());
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
        graph.mark_forward_from_start();
        //println!("{:#?}", graph);
        assert!(!graph.end().unwrap().1.mark);
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