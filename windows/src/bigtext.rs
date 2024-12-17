use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, RulesState}, timeline::{self, Timeline}};

use crate::client::VisualEffects;


pub struct BigFaceController {
    dialogue: Option<Face>,
}

impl BigFaceController {
    pub fn tick(&mut self, timeline: &Timeline, visual_effects: &mut VisualEffects) {
        let rule_state = &timeline.top_state().rules_state;
    }
}


struct Face {

}

#[derive(Default)]
pub struct BigTextController {
    text: Option<BigText>,
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
            if let Some(CrossyRulesetFST::Round(_)) = &self.last_rule_state_fst {
                // Moving into round end
            }
        }

        if let Some(text) = self.text.as_mut() {
            text.lifetime -= 1;
            if text.lifetime < 0 {
                self.text = None;
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
    }
}

struct BigText {
    sprite: &'static str,
    image_index: usize,
    lifetime: i32,
}