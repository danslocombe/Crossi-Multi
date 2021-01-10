#[macro_use]
extern crate num_derive;

mod game;

use num_traits::FromPrimitive;
use game::*;

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

/*
struct Client
{
    last_server_tick : u32,
    last_input : Input,
    game : Game,
}

impl Client
{
    fn tick(&mut self, input : Input)
    {
        if input != self.last_input
        {
            self.last_input = input;
            self.send_server();
        }
    }

    fn send_server(&self)
    {
    }
}

struct ClientMessage
{
    tick: u32,
    state: PlayerState,
}

struct Server
{
    tick : u32,
    game : Game,
}

impl Server
{
    fn tick(&mut self)
    {

    }

    fn receive_message(self, msg : &ClientMessage)
    {
    }
}

struct ServerMessage
{
    tick: u32,
    states : Vec<PlayerState>
}
*/