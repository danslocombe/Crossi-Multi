#[macro_use]
extern crate num_derive;

pub mod game;

use num_traits::FromPrimitive;
pub use game::*;



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