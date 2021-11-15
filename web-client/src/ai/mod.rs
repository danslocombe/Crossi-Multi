pub mod go_up;

use serde::Serialize;
use crossy_multi_core::{GameState, Input, PreciseCoords};
use crossy_multi_core::map::Map;

pub trait AIAgent : std::fmt::Debug
{
    fn think(&mut self, game_state: &GameState, map: &Map) -> Input;
    fn get_drawstate(&self) -> &AIDrawState;
}

#[derive(Debug, Serialize, Clone)]
pub struct AIDrawState
{
    draw_objs : Vec<AIDrawObj>
}

impl Default for AIDrawState {
    fn default() -> Self {
        Self {
            draw_objs : vec![],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
struct AIDrawObj
{
    precise_pos : PreciseCoords,
    draw_type : AIDrawType,
    colour : AIDrawColour,
}

#[derive(Debug, Serialize, Copy, Clone)]
enum AIDrawType
{
    Line(PreciseCoords),
    Cross,
    Tick,
    Circle,
}

#[derive(Debug, Serialize, Copy, Clone)]
enum AIDrawColour
{
    Green,
    Red,
}