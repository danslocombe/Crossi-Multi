extern crate crossy_multi_core;

mod client;

use crossy_multi_core::*;

use std::net::UdpSocket;
use num_traits::FromPrimitive;

struct LocalState
{
    socket : Option<UdpSocket>,
    game : Option<Game>
}

static mut LOCAL : LocalState = LocalState { socket: None, game: None };
static mut CLIENT : Option<client::Client> = None;

#[no_mangle]
pub unsafe fn create_client() -> f64 {
    CLIENT = client::Client::try_create().ok();

    if CLIENT.is_some() {1.0} else {0.0}
}

#[no_mangle]
pub unsafe fn create_local() -> f64 {
    /*
    let socket = UdpSocket::bind("127.0.0.1:8080");
    if socket.is_err()
    {
        return 1.0;
    }

    LOCAL.socket = Some(socket.unwrap());
    */
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