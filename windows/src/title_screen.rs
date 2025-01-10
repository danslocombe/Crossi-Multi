use crossy_multi_core::{math::V2};
use froggy_rand::FroggyRand;

use crate::{audio, client::VisualEffects, player_local::{g_all_skins, Skin}, rope::{self, ConstantForce, Lattice, NodeType, RopeWorld}, sprites, to_vector2};

pub struct TitleScreen {
    pub t: i32,
    text_pos: V2,
    left_curtain: Curtain,
    right_curtain: Curtain,

    pub goto_next_t: Option<i32>,

    pub draw_bg_tiles: bool,

    actor_controller: ActorController,
}

impl Default for TitleScreen {
    fn default() -> Self {
        {
            let mut actor_controller = ActorController::default();
            actor_controller.spawn_positions_grid.push((V2::new(20.0, 15.0), false));

            Self {
                t: 0,
                text_pos: V2::new(10.0, 60.0),
                left_curtain: Curtain::new(V2::new(-10.0, 20.0), V2::new(84.0, 20.0)),
                right_curtain: Curtain::new(V2::new(170.0, 20.0), V2::new(86.0, 20.0)),
                draw_bg_tiles: false,
                actor_controller,
                goto_next_t: None,
            }
        }
    }
}

impl TitleScreen {
    pub fn tick(&mut self, visual_effects: &mut VisualEffects, music_current_time_in_secs: f32) -> bool {
        if (self.t == 0) {
            //visual_effects.screenshake();
        }

        self.t += 1;
        let press = crate::input::goto_next_title();

        if (self.goto_next_t.is_none() && press) {
            audio::play("car");
            visual_effects.screenshake();
            self.goto_next_t = Some(self.t);
        }

        self.left_curtain.tick(self.goto_next_t);
        self.right_curtain.tick(self.goto_next_t);

        if self.t > self.goto_next_t.unwrap_or(self.t) {
            self.text_pos = crate::dan_lerp_v2(self.text_pos, V2::new(10.0, -80.0), 15.);
        }

        self.draw_bg_tiles = self.t > 100;

        self.actor_controller.tick(music_current_time_in_secs);

        self.t < 60 + self.goto_next_t.unwrap_or(self.t)
    }

    pub fn draw(&mut self) {
        let fade_out_t = ((self.t as f32 - self.goto_next_t.map(|x| x as f32 + 20.0).unwrap_or(self.t as f32)) / 40.0).clamp(0.0, 1.0);

        unsafe {
            let mut col = crate::BLACK;
            col.a = (255.0 * (1.0 - fade_out_t)) as u8;
            raylib_sys::DrawRectangle(0, 0, 200, 200, col);
        }

        unsafe {
            let rx = 50.0 * (1.0 - fade_out_t);
            let ry = 26.0 * (1.0 - fade_out_t);
            raylib_sys::DrawEllipse(80, 120, rx, ry, crate::BROWN);
            //raylib_sys::DrawRectangle(0, 100, 200, 100, crate::BROWN);
        }

        self.left_curtain.draw();
        self.right_curtain.draw();

        self.actor_controller.draw();

        sprites::draw("roadtoads", 0, self.text_pos.x, self.text_pos.y);
    }
}

struct Curtain {
    t: i32,
    rope_world: RopeWorld,
    lattice: Lattice,

    node_top_corner_wall: usize,
    node_top_corner_center: usize,

    on_left: bool,

    wind_norm: f32,
}

impl Curtain {
    pub fn new(top_corner_wall: V2, top_corner_center: V2) -> Self {
        let mut rope_world = RopeWorld::default();

        let width_world = (top_corner_center - top_corner_wall).x;
        let lattice = Lattice::create_rectangle(&mut rope_world, 12, 12, top_corner_wall, width_world, 100.0);

        let node_top_corner_wall = lattice.grid[0][0];
        let node_top_corner_center = *lattice.grid[0].last().unwrap();

        lattice.set_fixed(&mut rope_world, 0, 0);
        lattice.set_fixed(&mut rope_world, 4, 0);
        lattice.set_fixed(&mut rope_world, 8, 0);
        lattice.set_fixed(&mut rope_world, 11, 0);

        Self {
            t: 0,
            rope_world,
            node_top_corner_wall,
            node_top_corner_center,

            lattice,

            wind_norm: 0.0,

            on_left: top_corner_wall.x < top_corner_center.x,
        }
    }

    pub fn tick(&mut self, goto_next_t: Option<i32>) {
        self.t += 1;

        if let Some(goto_next_t) = goto_next_t {
            if self.t > goto_next_t {
                let w = self.lattice.grid[0].len();
                for x in 0..w {
                    if x % 2 == 0 || x == w-1 {
                        let id = self.lattice.grid[0][x];
                        let pos = self.rope_world.get_node(id).pos;
                        let wall = self.rope_world.get_node(self.node_top_corner_wall).pos;
                        let new_pos = crate::dan_lerp_v2(pos, wall, 25.0);
                        self.rope_world.get_node_mut(id).pos = new_pos;
                    }
                }
            }
        }

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

        self.rope_world.forces.clear();
        self.rope_world.forces.push(Box::new(ConstantForce {
            force: V2::new(self.wind_norm * 0.03, 0.03),
        }));

        self.rope_world.tick(1.0);
    }

    pub fn draw(&self) {
        // Pole
        unsafe {
            let p0 = self.rope_world.get_node(self.node_top_corner_center).pos;
            let p1 = self.rope_world.get_node(self.node_top_corner_wall).pos;
            raylib_sys::DrawLineV(
                to_vector2(p0),
                to_vector2(p1),
                crate::BEIGE);
        }

        self.lattice.draw_shadow(&self.rope_world, V2::new(0.0, 6.0));
        self.lattice.draw_curtain(&self.rope_world, self.on_left);
    }
}

#[derive(Debug, Clone)]
pub struct Actor {
    skin: Skin,
    pos_grid: V2,
    pos_target: V2,
    image_index: i32,
    t_since_move: i32,
    move_right: bool,
}

impl Actor {
    pub fn new(pos_grid: V2, move_right: bool, rand: FroggyRand) -> Self {
        let skin = rand.choose("skin", &g_all_skins);
        Self {
            skin: Skin::from_enum(*skin),
            pos_grid,
            pos_target: pos_grid,
            image_index: 0,
            t_since_move: 0,
            move_right,
        }
    }

    pub fn tick(&mut self, move_all: bool,) {
        if (move_all) {
            self.image_index = 0;
            if self.move_right {
                self.pos_target.x += 1.0;
            }
            else {
                self.pos_target.x -= 1.0;
            }
            self.t_since_move = 0;
        }
        else {
            self.t_since_move += 1;
            if self.t_since_move < 8 {
                self.image_index = self.t_since_move / 4;
            }
            else {
                self.image_index = 0;
            }

            self.pos_grid = crate::dan_lerp_v2(self.pos_grid, self.pos_target, 4.0);
        }
    }

    pub fn alive(&self) -> bool {
        if self.move_right {
            self.pos_grid.x < 22.
        }
        else {
            self.pos_grid.x > -2.
        }
    }

    pub fn draw(&self) {
        let xx = self.pos_grid.x * 8.0;
        let yy = self.pos_grid.y * 8.0;
        sprites::draw("shadow", 0, xx, yy);
        sprites::draw_with_flip(&self.skin.sprite, self.image_index as usize, xx, yy - 2.0, !self.move_right);
    }
}

#[derive(Default)]
pub struct ActorController {
    t: i32,
    t_music: f32,
    beat: i32,
    actors: Vec<Actor>,
    pub spawn_positions_grid: Vec<(V2, bool)>,
}

impl ActorController {
    pub fn tick(&mut self, music_current_time_in_secs: f32) {
        self.t += 1;

        // Small offset so characters are moving on the beat.
        let t_music = music_current_time_in_secs - 0.15;

        let bps = 60.0 / 100.0;
        let k = 4.0 * bps;
        let prev_rounded = (self.t_music * k).floor();
        let cur_rounded = (t_music * k).floor();

        let music_hit = cur_rounded != prev_rounded;

        if (music_hit) {
            self.beat += 1;
        }

        self.t_music = t_music;

        if (music_hit && self.beat % 2 == 0) {
            for (spawn_pos, walk_dir) in self.spawn_positions_grid.iter() {
                self.actors.push(Actor::new(*spawn_pos, *walk_dir, FroggyRand::new(self.t as u64)));
            }
        }

        for actor in self.actors.iter_mut() {
            actor.tick(music_hit);
        }

        // @Perf
        let mut new_actors = self.actors.iter().filter(|x| x.alive()).cloned().collect();
        std::mem::swap(&mut new_actors, &mut self.actors);
    }

    pub fn reset(&mut self) {
        self.actors.clear();
    }

    pub fn draw(&self) {
        for actor in self.actors.iter() {
            actor.draw();
        }
    }
}