extern crate crossy_multi_core;
use crossy_multi_core::*;

use num_traits::FromPrimitive;


#[no_mangle]
pub unsafe fn create_local() -> f64 {
    LOCAL.game = Some(Game::new());
    0.0
}

#[no_mangle]
pub unsafe fn tick_local(dir : f64, dt_us : f64) -> f64 {
    let input = SimulationInput
    {
        inputs: vec![FromPrimitive::from_i32(dir as i32).unwrap()],
    };

    LOCAL.game.as_mut().unwrap().tick(input, dt_us);
    0.0
}

unsafe fn get_player<'t>(id : f64) -> &'t PlayerState
{
    let player_id = FromPrimitive::from_u8(id as u8).unwrap();
    let state = LOCAL.game.as_ref().unwrap().current_state();
    state.get_player(player_id)
}

#[no_mangle]
pub unsafe fn get_player_x(id : f64) -> f64 {
    match get_player(id).pos
    {
        Pos::Coord(coord) => 
        {
            coord.x as f64
        },
        // TODO
        Pos::Log(_) => 0.0,
    }
}

#[no_mangle]
pub unsafe fn get_player_y(id : f64) -> f64 {
    match get_player(id).pos
    {
        Pos::Coord(coord) => 
        {
            coord.y as f64
        },
        // TODO
        Pos::Log(_) => 0.0,
    }
}

struct Local
{
    game : Option<Game>
}

static mut LOCAL: Local = Local { game: None };