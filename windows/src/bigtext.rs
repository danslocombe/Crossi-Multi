use std::pin;

use crossy_multi_core::{crossy_ruleset::{AliveState, CrossyRulesetFST, RulesState}, math::V2, timeline::{self, Timeline}, PlayerId};
use raylib_sys::Ray;

use crate::{client::{StateTransition, VisualEffects}, entities::EntityContainer, player_local::PlayerLocal, to_vector2};

struct Face {
    sprite: &'static str,
    t: i32,
    letterbox: f32,
    face_scale: f32,
    scale_factor: f32,
    face_x_off: f32,
    image_index: i32,
    t_end: i32,
    close_triggered: bool,
}

const fade_in_time: i32 = 16;
const fade_out_time: i32 = 24;
const target_letterbox: f32 = 30.0;
const target_face_scale: f32 = 1.25;
const face_x_off_max: f32 = 140.0;
const sound_delay: i32 = 80;

impl Face {
    pub fn tick(&mut self) {
        self.t += 1;

        if (self.t < fade_in_time) {
            self.scale_factor = crate::ease_in_quad(self.t as f32 / fade_in_time as f32);
            self.face_scale = self.scale_factor * target_face_scale;
        }
        else if (self.t < self.t_end) {
            if self.t < self.t_end - fade_out_time {
                // Steady state
                self.scale_factor = 1.0;

                // @TODO @Sound
                //  if (this.t > sound_delay && !this.played_sound) {
                //      this.played_sound = true;
                //      if (this.win_sound) {
                //          this.audio_manager.play(this.win_sound)
                //      }
                //  }
            }
            else {
                // Easing out
                self.scale_factor = crate::ease_in_quad((self.t_end - self.t) as f32 / fade_out_time as f32);
                self.face_x_off = (1.0 - self.scale_factor) * face_x_off_max;
                self.image_index = 1;
            }
        }
        else {
            // destroy.
        }

        self.letterbox = self.scale_factor * target_letterbox;
    }

    pub fn trigger_close(&mut self) {
        if (self.close_triggered) {
            return;
        }

        self.close_triggered = true;
        self.t_end = self.t_end.min(self.t + fade_out_time);
    }

    pub fn draw(&self) {
        unsafe {
            raylib_sys::DrawRectangle(0, 0, 160, self.letterbox as i32, crate::BLACK);
            raylib_sys::DrawRectangle(0, 160 - self.letterbox as i32, 160, 160, crate::BLACK);
        }

        let x = 150.0 + self.face_x_off;
        let y = 65.0;

        // @Perf
        let spr = crate::sprites::get_sprite(&self.sprite)[0];
        let interval = 50.0;
        let spin_interval = 115.0;
        let scale = self.face_scale * (1.0 + 0.12 * (self.t as f32 / interval).sin());

        let rotation = (360.0 / (2.0 * 3.141)) * 0.3 * (self.t as f32 / spin_interval);

        let mut pos = V2::new(x, y);
        pos -= V2::new(spr.width as f32, spr.height as f32) * scale *  0.5;
        crate::sprites::draw_ex(self.sprite, self.image_index as usize, pos, rotation, scale);
    }
}

#[derive(Default)]
pub struct BigTextController {
    text: Option<BigText>,
    face: Option<Face>,
    last_rule_state_fst: Option<CrossyRulesetFST>,
    pinwheel: Option<Pinwheel>,
}

impl BigTextController {
    pub fn tick(&mut self, timeline: &Timeline, players: &EntityContainer<PlayerLocal>, transitions: &StateTransition, new_players: &[PlayerId]) {
        let rules = &timeline.top_state().rules_state.fst;

        if let CrossyRulesetFST::Lobby { .. } = rules {
            if let Some(new_player) = new_players.iter().next() {
                let player = players.inner.iter().find(|x| x.player_id == *new_player).unwrap();
                self.face = Some(Face {
                    sprite: player.skin.dialogue_sprite,
                    t: 0,
                    letterbox: 0.0,
                    face_scale: 0.0,
                    scale_factor: 0.0,
                    face_x_off: 0.0,
                    image_index: 0,
                    t_end: 90,
                    close_triggered: false,
                });
            }
        }

        if let CrossyRulesetFST::RoundWarmup(state) = rules {
            let time_s = (state.remaining_us / 1_000_000) as i32;
            let full_s = (state.time_full_us / 1_000_000) as i32;
            let prev_time_s = if let Some(CrossyRulesetFST::RoundWarmup(last_state)) = &self.last_rule_state_fst {
                (last_state.remaining_us / 1_000_000) as i32
            }
            else {
                -1
            };

            if (time_s != prev_time_s) {
                let m_image_index = time_s;
                //let m_image_index = ((time_s - 3) - full_s);
                if (m_image_index <= 2 && m_image_index >= 0) {
                    self.text = Some(BigText {
                        sprite: "countdown",
                        image_index: 2 - (m_image_index as usize),
                        lifetime: 60,
                        offset: V2::default(),
                    });
                }
            }
        }

        if transitions.into_round {
            self.text = Some(BigText {
                sprite: "countdown",
                image_index: 3,
                lifetime: 60,
                offset: V2::default(),
            });
        }

        if transitions.into_winner {
            self.text = Some(BigText {
                sprite: "champion",
                image_index: 0,
                lifetime: 180,
                offset: V2::new(0.0, 32.0),
            });

            self.pinwheel = Some(Pinwheel {
                pos: V2::default(),
                t: 0,
                theta: 0.0,
                angle_vel: angle_vel_base,
                color: crate::BEIGE,
                visible: false,
            });
        }
        else {
            if let CrossyRulesetFST::EndWinner(state) = rules {
                if let Some(pinwheel) = self.pinwheel.as_mut() {
                    if let Some(winner) = players.inner.iter().find(|x| x.player_id == state.winner_id) {
                        pinwheel.pos = winner.pos * 8.0 + V2::new(4.0, 4.0);
                        pinwheel.color = winner.skin.color;
                        pinwheel.visible = true;
                    }
                }
            }
            else {
                self.pinwheel = None;
            }
        }

        if let CrossyRulesetFST::RoundCooldown(state) = rules {
            let mut winner = None;
            for (player_id, alive_state) in state.round_state.alive_states.iter() {
                if let AliveState::Alive = alive_state {
                    winner = Some(player_id);
                    break;
                }
            }

            if let Some(CrossyRulesetFST::Round(_)) = &self.last_rule_state_fst {
                // Moving into round end

                if let Some(winner_id) = winner {
                    // @Hack
                    let sprite = players.inner.iter().find(|x| x.player_id == winner_id).unwrap().skin.dialogue_sprite;

                    self.face = Some(Face {
                        sprite,
                        t: 0,
                        letterbox: 0.0,
                        face_scale: 0.0,
                        scale_factor: 0.0,
                        face_x_off: 0.0,
                        image_index: 0,
                        t_end: 120,
                        close_triggered: false,
                    });

                    self.text = Some(BigText {
                        sprite: "winner",
                        image_index: 0,
                        lifetime: 120,
                        offset: V2::default(),
                    })
                }
                else {
                    self.text = Some(BigText {
                        sprite: "no_winner",
                        image_index: 0,
                        lifetime: 120,
                        offset: V2::default(),
                    })
                }
            }
            else {
                // Check for death to trigger no winner.
                if (winner.is_none()) {
                    if let Some(face) = self.face.as_mut() {
                        face.trigger_close();
                    }

                    self.text = Some(BigText {
                        sprite: "no_winner",
                        image_index: 0,
                        lifetime: 120,
                        offset: V2::default(),
                    })
                }
            }
        }

        if let Some(text) = self.text.as_mut() {
            text.lifetime -= 1;
            if text.lifetime < 0 {
                self.text = None;
            }
        }

        if let Some(face) = self.face.as_mut() {
            face.tick();
            if (face.t > face.t_end) {
                self.face = None;
            }
        }

        if let Some(pinwheel) = self.pinwheel.as_mut() {
            pinwheel.tick();
        }

        self.last_rule_state_fst = Some(rules.clone());
    }

    pub fn draw_lower(&self) {
        if let Some(pinwheel) = self.pinwheel.as_ref() {
            pinwheel.draw();
        }
    }

    pub fn draw(&self) {
        if let Some(text) = self.text.as_ref() {
            // @Perf replace with constant
            let w_sprite = crate::sprites::get_sprite(&text.sprite)[0].width;
            // @Perf double lookup
            let pos = V2::new(80.0 - w_sprite as f32 * 0.5, 60.0) + text.offset;
            crate::sprites::draw(text.sprite, text.image_index, pos.x, pos.y);
        }

        if let Some(face) = self.face.as_ref() {
            face.draw();
        }
    }
}

#[derive(Debug)]
struct BigText {
    sprite: &'static str,
    image_index: usize,
    lifetime: i32,
    offset: V2,
}

const angle_vel_fast: f32 = 0.0825;
const angle_vel_base: f32 = 0.0125;
pub struct Pinwheel {
    pos: V2,
    t: i32,
    theta: f32,
    angle_vel: f32,
    visible: bool,
    color: raylib_sys::Color,
}

impl Pinwheel {
    pub fn set_vel_norm(&mut self, x: f32) {
        self.angle_vel = x * (angle_vel_fast - angle_vel_base) + angle_vel_base;
    }

    pub fn tick(&mut self) {
        self.theta += self.angle_vel;
        self.t += 1;

        self.set_vel_norm((1.0 - (self.t as f32 / 120.0)).max(0.0));
    }

    pub fn draw(&self) {
        if (!self.visible) {
            return;
        }

        let n = 8;
        let len = ((160.0 * 160.0 + 160.0 * 160.0) as f32).sqrt();

        let mut angle = self.theta;

        for i in 0..n {
            let pos1 = self.pos + V2::norm_from_angle(angle) * len;
            angle += 3.141 * 2.0 / n as f32;

            let pos2 = self.pos + V2::norm_from_angle(angle) * len;
            angle += 3.141 * 2.0 / n as f32;

            unsafe {
                raylib_sys::DrawTriangle(
                    to_vector2(self.pos),
                    to_vector2(pos2),
                    to_vector2(pos1),
                    self.color);
            }
        }
    }
}