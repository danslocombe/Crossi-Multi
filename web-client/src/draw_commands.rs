use crossy_multi_core::PreciseCoords;
use serde::Serialize;


#[derive(Default, Debug, Serialize, Clone)]
pub struct DrawCommands
{
    pub commands : Vec<DrawCommand>
}

#[derive(Debug, Serialize, Copy, Clone)]
pub struct DrawCoords {
    pub x : f32,
    pub y : f32,
}

impl DrawCoords {
    pub fn from_precise(precise : PreciseCoords) -> Self {
        Self {
            x: precise.x as f32, //x : precise.x as f32 + 0.5,
            y : precise.y as f32 //y : precise.y as f32 + 0.5,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DrawCommand
{
    pub pos : DrawCoords,
    pub draw_type : DrawType,
    pub colour : DrawColour,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum DrawType
{
    Line(DrawCoords),
    Cross,
    Tick,
    Circle,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum DrawColour
{
    Green,
    Red,
    Grey,
    White,
}