use crossy_multi_core::{crossy_ruleset::{player_in_lobby_ready_zone, AliveState}, map::RowType, math::V2, player::PlayerStatePublic, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, GameState, Input, PlayerId, PlayerInputs, Pos};
use froggy_rand::FroggyRand;

use crate::{client::VisualEffects, console, diff, entities::{Bubble, Corpse, Dust, Entity, EntityContainer, EntityType, IsEntity}, lerp_snap, sprites};

#[derive(Debug)]
pub struct PlayerLocal {
    pub entity_id: i32,
    pub player_id: PlayerId,
    pub pos: V2,
    pub moving: bool,
    pub x_flip: bool,
    pub image_index: i32,
    pub buffered_input: Input,
    pub created_corpse: bool,
    pub t : i32,
    pub skin: Skin,
}

const MOVE_T : i32 = 7 * (1000 * 1000 / 60);
const PLAYER_FRAME_COUNT: i32 = 5;

#[derive(Debug, Clone)]
pub struct Skin {
    pub sprite: &'static str,
    pub dead_sprite: &'static str,
}

impl Default for Skin {
    fn default() -> Self {
        Self {
            sprite: "frog",
            dead_sprite: "frog_dead",
        }
    }
}

impl PlayerLocal {
    pub fn new(entity_id: i32, pos: V2,) -> Self {
        Self {
            entity_id,
            player_id: PlayerId(0),
            pos,
            moving: false,
            x_flip: false,
            image_index: 0,
            buffered_input: Input::None,
            created_corpse: false,
            t: 0,
            skin: Skin::default(),
        }
    }

    pub fn reset(&mut self) {
        self.created_corpse = false;
    }

    pub fn set_from(&mut self, state: &PlayerStatePublic) {
        self.player_id = PlayerId(state.id);
        self.pos = V2::new(state.x as f32, state.y as f32);
    }

    pub fn update_inputs(&mut self, timeline: &Timeline, player_inputs: &mut PlayerInputs, input: Input) {
        if (input != Input::None) {
            self.buffered_input = input;

        }

        if (input == Input::Left) {
            self.x_flip = true;
        }

        if (input == Input::Right) {
            self.x_flip = false;
        }

        let top = timeline.top_state();
        if (top.player_states.get(self.player_id).unwrap().can_move()) {
            player_inputs.set(self.player_id, self.buffered_input);
            self.buffered_input = Input::None;
        }
    }

    pub fn tick(
        &mut self,
        player_state: &PlayerStatePublic,
        alive_state: AliveState,
        timeline: &Timeline,
        visual_effects: &mut VisualEffects,
        dust: &mut EntityContainer<Dust>,
        bubbles: &mut EntityContainer<Bubble>,
        corpses: &mut EntityContainer<Corpse>) {
        self.t += 1;

        let x0 = player_state.x as f32;
        let y0 = player_state.y as f32;

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        if (player_state.moving) {
            let lerp_t = 1.0 - (player_state.remaining_move_dur as f32 / MOVE_T as f32);

            let x1 = player_state.t_x as f32;
            let y1 = player_state.t_y as f32;

            self.image_index = (self.image_index + 1);
            if (self.image_index >= PLAYER_FRAME_COUNT) {
                self.image_index = PLAYER_FRAME_COUNT - 1;
            }

            x = x0 + lerp_t * (x1 - x0);
            y = y0 + lerp_t * (y1 - y0);
        }
        else {
            let new_p = lerp_snap(self.pos.x, self.pos.y, x0, y0);
            x = new_p.x;
            y = new_p.y;

            let delta = 8.0 * 0.01;
            if (diff(x, self.pos.x) > delta || diff(y, self.pos.y) > delta) {
                self.image_index = (self.image_index + 1) % PLAYER_FRAME_COUNT;
            }
            else {
                self.image_index = 0;
            }
        }

        if (player_state.moving && !self.moving) {
            // Started moving, do effects.
            let rand = FroggyRand::from_hash((self.player_id.0, self.t));
            for i in 0..2 {
                let rand = rand.subrand(i);
                let dust_off = rand.gen_unit("off") * 3.0;
                let dust_dir = rand.gen_unit("dir") * 3.141 * 2.0;
                let pos = self.pos * 8.0 + V2::new(4.0, 4.0) + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                //let pos = self.pos * 8.0 + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                let eid = dust.create_entity(Entity {
                    id: 0,
                    entity_type: EntityType::Dust,
                    pos: Pos::Absolute(pos),
                });
                let dust_part = dust.get_mut(eid).unwrap();
                dust_part.image_index = rand.gen_usize_range("frame", 0, 3) as i32;
                dust_part.scale = (0.5 + rand.gen_unit("scale") * 0.6) as f32;
            }
        }

        if (alive_state == AliveState::Dead && !self.created_corpse) {
            self.created_corpse = true;

            //let target_pos = V2::new((player_state.t_x * 8.0) as f32, player_state.t_y as f32 * 8.0);
            let corpse_pos = if player_state.moving {
                V2::new(player_state.t_x as f32, player_state.t_y as f32)
            }
            else {
                V2::new(player_state.x as f32, player_state.y as f32)
            } * 8.0;

            let top_state = timeline.top_state();
            let row = timeline.map.get_row(top_state.rules_state.fst.get_round_id(), player_state.y);
            if let RowType::River(_) = row.row_type {
                // Drowning.
                let rand = FroggyRand::from_hash((self.player_id.0, self.t));
                for i in 0..2 {
                    let rand = rand.subrand(i);
                    let dust_off = rand.gen_unit("off") * 3.0;
                    let dust_dir = rand.gen_unit("dir") * 3.141 * 2.0;
                    let pos = corpse_pos * 8.0 + V2::new(4.0, 4.0) + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                    //let pos = self.pos * 8.0 + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
                    let bubble_part = bubbles.create(Pos::Absolute(pos));
                    bubble_part.image_index = rand.gen_usize_range("frame", 0, 3) as i32;
                    bubble_part.scale = (0.5 + rand.gen_unit("scale") * 0.6) as f32;
                }
            }
            else {
                // Hit by car.
                let corpse = corpses.create(Pos::Absolute(corpse_pos));
                corpse.skin = self.skin.clone();
            }

            visual_effects.screenshake();
            visual_effects.whiteout();
        }

        self.pos.x = x;
        self.pos.y = y;
        self.moving = player_state.moving;
    }
}

impl IsEntity for PlayerLocal {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.entity_id,
            entity_type: EntityType::Player,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 * 8
    }

    fn draw(&mut self) {
        if (!self.created_corpse) {
            sprites::draw("shadow", 0, self.pos.x * 8.0, self.pos.y * 8.0);
            sprites::draw_with_flip(&self.skin.sprite, self.image_index as usize, self.pos.x * 8.0, self.pos.y * 8.0 - 2.0, self.x_flip);
        }
    }
}