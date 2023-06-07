pub mod go_up;

use serde::Serialize;
use crossy_multi_core::{PlayerId, GameState, Input, PreciseCoords, game};
use crossy_multi_core::map::Map;

use crate::draw_commands::DrawCommands;

pub trait AIAgent : std::fmt::Debug
{
    fn think(&mut self, game_state: &GameState, map: &Map) -> Input;
    fn get_drawstate(&self) -> &DrawCommands;
}


#[derive(Debug)]
pub struct BackAndForth
{
    player_id : PlayerId,
    draw_state : DrawCommands,
}

impl BackAndForth
{
    pub fn new(player_id : PlayerId) -> Self {
        Self {
            player_id,
            draw_state : DrawCommands::default(),
        }
    }
}

impl AIAgent for BackAndForth
{
    fn think(&mut self, game_state : &GameState, _ : &Map) -> Input
    {
        /*
        if (game_state.frame_id % 60 == 0) {
            Input::Left
        }
        else if (game_state.frame_id % 60 == 30) {
            Input::Right
        }
        else {
            Input::None
        }
        */

        if (game_state.frame_id / 60) % 2 == 0 {
            Input::Left
        }
        else {
            Input::Right
        }

    }

    fn get_drawstate(&self) -> &DrawCommands {
        &self.draw_state
    }
}