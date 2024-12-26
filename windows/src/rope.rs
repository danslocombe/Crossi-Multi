// Copied from Crunda directly.
// TODO some internal package managing?

use crossy_multi_core::math::V2;

#[derive(Default)]
pub struct RopeWorld {
    pub nodes: Vec<RopeNode>,
    pub ropes: Vec<Rope>,
    pub colliders: Vec<Collider>,
    pub forces: Vec<Box<dyn Force>>,
}

impl RopeWorld {
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

    pub fn get_node(&self, id: usize) -> &RopeNode {
        &self.nodes[id]
    }

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