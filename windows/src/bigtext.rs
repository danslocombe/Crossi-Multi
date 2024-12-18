use crossy_multi_core::{crossy_ruleset::{AliveState, CrossyRulesetFST, RulesState}, math::V2, timeline::{self, Timeline}};

use crate::client::VisualEffects;

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
}

impl BigTextController {
    pub fn tick(&mut self, timeline: &Timeline) {
        let rules = &timeline.top_state().rules_state.fst;

        if let CrossyRulesetFST::RoundWarmup(state) = rules {
            let time_s = (state.remaining_us / 1_000_000) as i32;
            let prev_time_s = if let Some(CrossyRulesetFST::RoundWarmup(last_state)) = &self.last_rule_state_fst {
                (last_state.remaining_us / 1_000_000) as i32
            }
            else {
                -1
            };

            if (time_s != prev_time_s) {
                self.text = Some(BigText {
                    sprite: "countdown",
                    image_index: (3 - time_s) as usize,
                    lifetime: 60,
                });
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
                    let sprite = if (winner_id.0 == 1) {
                        "frog_dialogue"
                    }
                    else {
                        "mouse_dialogue_cute"
                    };

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
                    })
                }
                else {
                    self.text = Some(BigText {
                        sprite: "no_winner",
                        image_index: 0,
                        lifetime: 120,
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

        self.last_rule_state_fst = Some(rules.clone());
    }

    pub fn draw(&self) {
        if let Some(text) = self.text.as_ref() {
            // @Perf replace with constant
            let w_sprite = crate::sprites::get_sprite(&text.sprite)[0].width;
            // @Perf double lookup
            crate::sprites::draw(text.sprite, text.image_index, 80.0 - w_sprite as f32 * 0.5, 60.0);
        }

        if let Some(face) = self.face.as_ref() {
            face.draw();
        }
    }
}

struct BigText {
    sprite: &'static str,
    image_index: usize,
    lifetime: i32,
}