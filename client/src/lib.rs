use crossy_multi_core::*;

use std::io::Write;
use std::net::UdpSocket;
use num_traits::FromPrimitive;

static mut CLIENT : Option<client::Client> = None;

#[no_mangle]
pub unsafe fn create_client(port : f64) -> f64 {
    match client::Client::try_create(port as u16)
    {
        Ok(c) => {
            CLIENT = Some(c);
            0.0
        }
        Err(e) => {
            println!("Error initializing client {}", e);
            1.0
        }
    }
}

#[no_mangle]
pub unsafe fn create_local() -> f64 {
    0.0
}

#[no_mangle]
pub unsafe fn tick_local(dir : f64, _gm_dt_us : f64) -> f64 {
    let player_input = FromPrimitive::from_i32(dir as i32).unwrap();  
    let client = CLIENT.as_mut().unwrap();
    client.tick(player_input);

    0.0
}

unsafe fn get_player<'t>(id : f64) -> Option<&'t PlayerState>
{
    let player_id = FromPrimitive::from_u8(id as u8).unwrap();
    CLIENT.as_ref()
        .and_then(|x| x.timeline.current_state().get_player(player_id))
}

#[no_mangle]
pub unsafe fn get_player_x(id : f64) -> f64 {
    match get_player(id).map(|x| x.pos)
    {
        Some(Pos::Coord(coord)) => 
        {
            coord.x as f64
        },
        // TODO
        Some(Pos::Log(_)) => 0.0,
        None => f64::NAN,
    }
}

#[no_mangle]
pub unsafe fn get_player_y(id : f64) -> f64 {
    match get_player(id).map(|x| x.pos)
    {
        Some(Pos::Coord(coord)) => 
        {
            coord.y as f64
        },
        // TODO
        Some(Pos::Log(_)) => 0.0,
        None => f64::NAN,
    }
}

#[no_mangle]
pub unsafe fn get_player_count() -> f64 {
    match CLIENT.as_ref().map(|x| x.timeline.top_state().get_player_count())
    {
        Some(x) => x as f64,
        None => 0.0,
    }
}

#[no_mangle]
pub unsafe fn dump_file() {
    let mut file = std::fs::File::create("timeline.dump").unwrap();
    let s = format!("{:#?}", CLIENT.as_ref().unwrap().timeline);
    file.write_all(s.as_bytes()).unwrap();
}