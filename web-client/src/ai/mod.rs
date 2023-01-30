pub mod go_up;

use serde::Serialize;
use crossy_multi_core::{PlayerId, GameState, Input, PreciseCoords, game};
use crossy_multi_core::map::Map;

pub trait AIAgent : std::fmt::Debug
{
    fn think(&mut self, game_state: &GameState, map: &Map) -> Input;
    fn get_drawstate(&self) -> &AIDrawState;
}

#[derive(Debug, Serialize, Clone)]
pub struct AIDrawState
{
    pub draw_objs : Vec<AIDrawObj>
}

impl Default for AIDrawState {
    fn default() -> Self {
        Self {
            draw_objs : vec![],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct AIDrawObj
{
    pub precise_pos : PreciseCoords,
    pub draw_type : AIDrawType,
    pub colour : AIDrawColour,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum AIDrawType
{
    Line(PreciseCoords),
    Cross,
    Tick,
    Circle,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum AIDrawColour
{
    Green,
    Red,
    White,
}

#[derive(Debug)]
pub struct BackAndForth
{
    player_id : PlayerId,
    draw_state : AIDrawState,
}

impl BackAndForth
{
    pub fn new(player_id : PlayerId) -> Self {
        Self {
            player_id,
            draw_state : AIDrawState::default(),
        }
    }
}

impl AIAgent for BackAndForth
{
    fn think(&mut self, game_state : &GameState, _ : &Map) -> Input
    {
        if (game_state.frame_id % 60 == 0) {
            Input::Left
        }
        else if (game_state.frame_id % 60 == 30) {
            Input::Right
        }
        else {
            Input::None
        }
    }

    fn get_drawstate(&self) -> &AIDrawState {
        &self.draw_state
    }
}