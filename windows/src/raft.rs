use crossy_multi_core::{math::V2, Pos};
use froggy_rand::FroggyRand;

use crate::{entities::{Entity, EntityType, IsEntity}, rope::{Lattice, RopeWorld}, to_vector2};

pub struct RaftSail {
    pub id : i32,
    pub pos: V2,
    pub t: i32,
    pub flag_rope_world: RopeWorld,
    pub flag_lattice: Lattice,

    pub sail_rope_world: RopeWorld,
    pub sail_lattice: Lattice,

    pub wind_norm: f32,
}

impl RaftSail {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            t: 0,

            flag_rope_world: Default::default(),
            flag_lattice: Default::default(),

            sail_rope_world: Default::default(),
            sail_lattice: Default::default(),

            wind_norm: 0.0,
        }
    }

    pub fn setup(&mut self) {
        assert!(self.flag_lattice.grid.is_empty());
        assert!(self.flag_rope_world.nodes.is_empty());
        assert!(self.flag_rope_world.ropes.is_empty());

        assert!(self.sail_lattice.grid.is_empty());
        assert!(self.sail_rope_world.nodes.is_empty());
        assert!(self.sail_rope_world.ropes.is_empty());

        self.sail_lattice = Lattice::create_triangle(&mut self.sail_rope_world, 6, 6, V2::default(), 16.0, 32.0);
        self.sail_lattice.set_fixed(&mut self.sail_rope_world, 0, 0);
        self.sail_lattice.set_fixed(&mut self.sail_rope_world, 0, 5);

        self.flag_lattice = Lattice::create_rectangle(&mut self.flag_rope_world, 6, 6, V2::default(), 10.0, 8.0);
        for i in 0..6 {
            self.flag_lattice.set_fixed(&mut self.flag_rope_world, 0, i);
        }
    }

    pub fn tick(&mut self, pos: V2) {
        self.t += 1;

        self.wind_norm *= 0.9;
        let rand = FroggyRand::new(self.t as u64);
        if rand.gen_unit(0) < 0.01 {
            if rand.gen_unit(1) < 0.5 {
                self.wind_norm += 1.0;
            }
            else {
                self.wind_norm += -1.0;
            }
        }

        let delta = (pos - self.pos).x;
        // Add delta to the wind norm

        self.wind_norm -= delta * 1.5;

        self.flag_rope_world.forces.clear();
        self.flag_rope_world.forces.push(Box::new(crate::rope::ConstantForce {
            force: V2::new(self.wind_norm * 0.03, 0.03),
        }));

        self.sail_rope_world.forces.clear();
        self.sail_rope_world.forces.push(Box::new(crate::rope::ConstantForce {
            force: V2::new(self.wind_norm * 0.03, 0.03),
        }));


        self.pos = pos;
        self.flag_rope_world.tick(1.0);
        self.sail_rope_world.tick(1.0);
    }
}

impl IsEntity for RaftSail {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::RaftSail,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        //self.pos.y as i32 + 40 - 8
        self.pos.y as i32 + 40
    }

    fn draw(&mut self) {
        self.t += 1;
        {
            //let xx = self.pos.x - 4.0;
            //let xx = self.pos.x + 2.0;
            //let yy = self.pos.y + 8.0;
            //sprites::draw("raft_sail_frame", 0, xx, yy);
            //sprites::draw("raft", 0, xx, yy);
        }

        const brown_frame: raylib_sys::Color = crate::hex_color("8f563b".as_bytes());

        unsafe {
            let base_pos = self.pos + V2::new(2.0, 16.0);
            raylib_sys::DrawLineV(to_vector2(base_pos), to_vector2(base_pos + V2::new(0.0, 16.0)), brown_frame);

            for edge in self.flag_rope_world.ropes.iter() {
                let from_pos = base_pos + self.flag_rope_world.nodes[edge.from].pos;
                let to_pos = base_pos + self.flag_rope_world.nodes[edge.to].pos;
                raylib_sys::DrawLineV(to_vector2(from_pos), to_vector2(to_pos), crate::WHITE);
            }

            self.flag_lattice.draw_flag(&self.flag_rope_world, base_pos);
        }

        let base_pos = self.pos + V2::new(18.0, 20.0);
        self.sail_lattice.draw_shadow(&self.sail_rope_world, base_pos + V2::new(0.0, 3.0));
        self.sail_lattice.draw_sail(&self.sail_rope_world, base_pos);
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        true
    }
}