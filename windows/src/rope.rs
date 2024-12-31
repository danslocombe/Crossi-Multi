// Copied from Crunda directly.
// TODO some internal package managing?

use crossy_multi_core::math::V2;

use crate::to_vector2;

#[derive(Default)]
pub struct RopeWorld {
    pub nodes: Vec<RopeNode>,
    pub ropes: Vec<Rope>,
    pub colliders: Vec<Collider>,
    pub forces: Vec<Box<dyn Force>>,
}

impl RopeWorld {
    #[inline]
    pub fn add_node_p(&mut self, p: V2) -> usize {
        self.nodes.push(RopeNode::new(p.x, p.y));
        self.nodes.len() - 1
    }

    pub fn add_node(&mut self, x: f32, y: f32) -> usize {
        self.nodes.push(RopeNode::new(x, y));
        self.nodes.len() - 1
    }

    pub fn add_rope(&mut self, from: usize, to: usize) -> usize {
        debug_assert!(from < self.nodes.len());
        debug_assert!(to < self.nodes.len());
        debug_assert!(from != to);

        self.ropes.push(Rope::new(from, to, &self));
        self.ropes.len() - 1
    }

    #[inline]
    pub fn get_node(&self, id: usize) -> &RopeNode {
        &self.nodes[id]
    }

    #[inline]
    pub fn get_node_mut(&mut self, id: usize) -> &mut RopeNode {
        &mut self.nodes[id]
    }

    pub fn get_rope(&self, id: usize) -> &Rope {
        &self.ropes[id]
    }

    pub fn get_rope_mut(&mut self, id: usize) -> &mut Rope {
        &mut self.ropes[id]
    }

    // Done here due to borrow pain
    fn tick_rope(&mut self, rope_id: usize) {
        let rope = &self.ropes[rope_id];

        if (rope.broken) {
            return;
        }

        let from_0 = self.nodes[rope.from].clone();
        let to_0 = self.nodes[rope.to].clone();
        let centre = from_0.pos + (to_0.pos - from_0.pos) * 0.5;

        // TODO trying to get ropes to break?
        ////let dist = from_0.pos.sub(to_0.pos).mag();
        ////if (dist > rope.length * 1.5) {
        ////  // Break!
        ////  self.ropes[rope_id].broken = true;
        ////  return;
        ////}

        let half_len = rope.length / 2.0;

        match (from_0.node_type, to_0.node_type) {
            (NodeType::Fixed, NodeType::Fixed) => {
                // Nothing to do, both ends fixed
                return;
            }
            (NodeType::Fixed, NodeType::Free) => {
                self.nodes[rope.to].pos = centre.project_dist_towards(to_0.pos, half_len);
            }
            (NodeType::Free, NodeType::Fixed) => {
                self.nodes[rope.from].pos = centre.project_dist_towards(from_0.pos, half_len);
            }
            _ => {
                self.nodes[rope.from].pos = centre.project_dist_towards(from_0.pos, half_len);
                self.nodes[rope.to].pos = centre.project_dist_towards(to_0.pos, half_len);
            }
        }
    }

    pub fn tick(&mut self, dt_norm: f32) {
        for node in &mut self.nodes {
            node.tick(&self.forces, dt_norm);
        }

        const SIM_ITERS: usize = 8;
        for _ in 0..SIM_ITERS {
            for rid in 0..self.ropes.len() {
                self.tick_rope(rid);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    Fixed,
    Free,
}

#[derive(Debug, Clone)]
pub struct RopeNode {
    pub node_type: NodeType,
    pub pos: V2,
    prev_pos: V2,
}

impl RopeNode {
    fn new(x: f32, y: f32) -> Self {
        Self {
            node_type: NodeType::Free,
            pos: V2::new(x, y),
            prev_pos: V2::new(x, y),
        }
    }

    fn tick(&mut self, forces: &[Box<dyn Force>], _dt_norm: f32) {
        if (self.node_type == NodeType::Fixed) {
            return;
        }

        //let mut vel = self.pos.sub(self.prev_pos).mult(dt_norm);
        let mut vel = self.pos - self.prev_pos;

        const FRIC: f32 = 0.985;
        vel = vel.mult(FRIC);

        for force in forces {
            vel += force.get_force(self.pos);
        }

        self.prev_pos = self.pos;
        //self.pos = self.pos.add(vel.mult(dt_norm));
        self.pos += vel;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rope {
    pub from: usize,
    pub to: usize,
    length: f32,
    pub broken: bool,
}

impl Rope {
    fn new(from: usize, to: usize, world: &RopeWorld) -> Self {
        let length = world.get_node(from).pos.dist(world.get_node(to).pos);
        Self {
            from,
            to,
            length,
            broken: false,
        }
    }
}

pub struct Collider {}

pub trait Force {
    fn get_force(&self, rope_node_pos: V2) -> V2;
}

pub struct ConstantForce {
    pub force: V2,
}

impl Force for ConstantForce {
    fn get_force(&self, _: V2) -> V2 {
        self.force
    }
}

pub struct InverseSquareForce {
    pub strength: f32,
    pub pos: V2,
}

impl Force for InverseSquareForce {
    fn get_force(&self, node_pos: V2) -> V2 {
        let delta = self.pos - node_pos;
        let d2 = delta.mag2();
        if (d2 == 0.0) {
            return V2::default();
        }

        let d = delta.mag();
        let mag = self.strength / d2;
        delta.mult(mag / d)
    }
}

/*
pub struct SweptForce {
    pub push_vector: V2,
    pub line_p0: V2,
    pub line_p1: V2,
    pub capsule_shape_r : f32,
}

impl Force for SweptForce {
    fn get_force(&self, node_pos: V2) -> V2 {
        if (!crossy_multi_core::math::is_close_to_line_segment(node_pos, self.line_p0, self.line_p1, self.capsule_shape_r)) {
            return V2::default();
        }

        self.push_vector
    }
}
    */

#[derive(Default)]
pub struct Lattice {
    pub grid: Vec<Vec<usize>>,
}

impl Lattice {
    pub fn create_rectangle(rope_world: &mut RopeWorld, node_width: usize, node_height: usize, p_base: V2, world_width: f32, world_height: f32) -> Self {
        let mut grid: Vec<Vec<usize>> = Vec::new();
        for y in 0..node_height {
            let p = p_base + V2::new(0.0, world_height * ((y as f32) * 1.0/(node_height as f32)));

            let mut row = Vec::new();
            for x in 0..node_width {
                let p = p + V2::new(world_width * ((x as f32) * 1.0/((node_width - 1) as f32)), 0.0);
                let id = rope_world.add_node_p(p);
                row.push(id);

                let id = *row.last().unwrap();

                if y > 0 {
                    let above = grid[y - 1][x];
                    rope_world.add_rope(above, id);
                }

                if x > 0 {
                    let left = row[row.len() - 2];
                    rope_world.add_rope(left, id);
                }
            }

            grid.push(row);
        }

        Self {
            grid,
        }
    }

    pub fn create_triangle(rope_world: &mut RopeWorld, node_width: usize, node_height: usize, p_base: V2, world_width: f32, world_height: f32) -> Self {
        let mut grid: Vec<Vec<usize>> = Vec::new();

        for y in 0..node_height {
            let p = p_base + V2::new(0.0, world_height * ((y as f32) * 1.0/((node_height) as f32)));
            let mut row = Vec::new();
            for x in 0..=y {
                let p = p + V2::new(world_width * (-(x as f32) * 1.0/((node_width - 1) as f32)), 0.0);
                let id = rope_world.add_node_p(p);

                row.push(id);

                if y > 0 {
                    if (x < grid[y - 1].len()) {
                        let above = grid[y - 1][x];
                        rope_world.add_rope(above, id);
                    }
                    else {
                        // Triangular
                        let above = *grid[y - 1].last().unwrap();
                        rope_world.add_rope(above, id);
                    }
                }

                if x > 0 {
                    let left = row[row.len() - 2];
                    rope_world.add_rope(left, id);
                }
            }

            grid.push(row);
        }

        Self {
            grid,
        }
    }

    pub fn set_fixed(&self, rope_world: &mut RopeWorld, x: usize, y: usize) {
        let id = self.grid[y][x];
        rope_world.nodes[id].node_type = NodeType::Fixed;
    }

    pub fn get_quad(&self, rope_world: &RopeWorld, x: usize, y: usize) -> LatticeQuad {
        LatticeQuad {
            top_left: rope_world.get_node(self.grid[y-1][x-1]).pos,
            top_right: rope_world.get_node(self.grid[y-1][x]).pos,
            bot_left: rope_world.get_node(self.grid[y][x-1]).pos,
            bot_right: rope_world.get_node(self.grid[y][x]).pos,
        }
    }

    pub fn get_quad_handle_triangle(&self, rope_world: &RopeWorld, x: usize, y: usize) -> LatticeQuad {
        LatticeQuad {
            top_left: rope_world.get_node(self.grid[y-1][x-1]).pos,
            top_right: if (x < self.grid[y-1].len()) {
                rope_world.get_node(self.grid[y-1][x]).pos
            }
            else {
                rope_world.get_node(*self.grid[y-1].last().unwrap()).pos
            },
            bot_left: rope_world.get_node(self.grid[y][x-1]).pos,
            bot_right: rope_world.get_node(self.grid[y][x]).pos,
        }
    }

    pub fn draw_shadow(&self, rope_world: &RopeWorld, p_base: V2) {
        // Shadow
        let h = self.grid.len();
        let w = self.grid[h-1].len();

        for x in 1..w {
            let y = h - 1;

            let mut quad = self.get_quad_handle_triangle(rope_world, x, y);
            quad.offset(p_base);

            quad.draw_left(crate::BLACK);
            quad.draw_right(crate::BLACK);
        }
    }

    pub fn draw_curtain(&self, rope_world: &RopeWorld, on_left: bool) {
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in 1..h {
            for x in 1..w {
                let quad = self.get_quad(rope_world, x, y);

                //const curtain_lighter: raylib_sys::Color = crate::hex_color("e94476".as_bytes());
                const curtain_darker: raylib_sys::Color = crate::hex_color("be3d64".as_bytes());

                let col = if x % 2 == 0 {
                    curtain_darker
                }
                else {
                    crate::RED
                };

                if (on_left) {
                    quad.draw_left(col);
                }
                else {
                    quad.draw_right(col);
                }
            }
        }
    }

    pub fn draw_flag(&self, rope_world: &RopeWorld, base_pos: V2) {
        let h = self.grid.len();
        let w = self.grid[0].len();
        for y in 1..h {
            for x in 1..w {
                let mut quad = self.get_quad(rope_world, x, y);
                quad.offset(base_pos);

                let col_a = if (x + y) % 2 == 0 {
                    crate::WHITE
                }
                else {
                    crate::GREEN
                };

                let col_b = if (x + y) % 2 == 0 {
                    crate::WHITE
                }
                else {
                    crate::RED
                };

                quad.draw_left(col_b);
                quad.draw_right(col_a);
            }
        }
    }

    pub fn draw_sail(&self, rope_world: &RopeWorld, base_pos: V2) {
        let h = self.grid.len();
        for y in 1..h {
            let w = self.grid[y].len();
            for x in 1..w {
                let mut quad = self.get_quad_handle_triangle(rope_world, x, y);
                quad.offset(base_pos);

                quad.draw_left(crate::WHITE);
                quad.draw_right(crate::WHITE);
            }
        }
    }
}

pub struct LatticeQuad {
    pub top_left: V2,
    pub top_right: V2,
    pub bot_left: V2,
    pub bot_right: V2,
}

impl LatticeQuad {
    pub fn offset(&mut self, offset: V2) {
        self.top_left += offset;
        self.top_right += offset;
        self.bot_left += offset;
        self.bot_right += offset;
    }

    pub fn draw_left(&self, col: raylib_sys::Color) {
        unsafe {
            raylib_sys::DrawTriangle(
                to_vector2(self.top_left),
                to_vector2(self.bot_left),
                to_vector2(self.top_right),
                col);
            raylib_sys::DrawTriangle(
                to_vector2(self.bot_right),
                to_vector2(self.top_right),
                to_vector2(self.bot_left),
                col);
        }
    }

    pub fn draw_right(&self, col: raylib_sys::Color) {
        unsafe {
            raylib_sys::DrawTriangle(
                to_vector2(self.top_left),
                to_vector2(self.top_right),
                to_vector2(self.bot_left),
                col);
            raylib_sys::DrawTriangle(
                to_vector2(self.bot_right),
                to_vector2(self.bot_left),
                to_vector2(self.top_right),
                col);
        }
    }
}