use crossy_multi_core::interop::*;
use crossy_multi_core::game;

use std::io::Result;
use std::net::{UdpSocket, SocketAddr, Ipv4Addr, SocketAddrV4};
use std::time::{Instant, Duration};

pub struct Client
{
    server : SocketAddr, 
    socket : UdpSocket,
    pub game : game::Game,
    pub local_player_id : game::PlayerId,
    start : Instant,
    last_tick : Instant,
}

fn connect(socket: &mut UdpSocket, addr : &SocketAddr) -> Result<(ServerTick, u32, game::PlayerId)>
{
    println!("Connecting..");
    //socket.send_to(&INIT_MESSAGE, addr)?;
    let hello = CrossyMessage::Hello(ClientHello::new());
    crossy_send(&hello, socket, addr).unwrap();
    let init_response = match crossy_receive(socket)? {
        (CrossyMessage::HelloResponse(response), _) => response,
        x => panic!("Got unexpected response from server {:?}", x),
    };
    let first_tick = crossy_receive(socket)?;
    match first_tick {
        (CrossyMessage::ServerTick(tick), _) => {
            Ok((tick, init_response.seed, init_response.player_id))
        },
        x => panic!("Got unexpected response from server {:?}", x),
    }
}

impl Client
{
    pub fn try_create(port : u16) -> Result<Self>
    {
        let mut socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))?;
        let server = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8085));

        let (server_tick, seed, local_player_id) = connect(&mut socket, &server)?;
        println!("Connected! Our id {:?}, seed {}, server response {:?}", local_player_id, seed, server_tick);

        socket.set_nonblocking(true)?;

        Ok(Client {
            server : server,
            socket: socket,
            game : game::Game::from_server_parts(seed, server_tick.time_us, server_tick.states, 1),
            local_player_id : local_player_id,
            start : Instant::now(),
            last_tick : Instant::now(),
        })
    }

    pub fn tick(&mut self, input : game::Input)
    {
        let tick_start = Instant::now();
        let current_time = tick_start.saturating_duration_since(self.start);
        let dt = tick_start.saturating_duration_since(self.last_tick);
        self.last_tick = tick_start;

        let server_tick = self.recv().unwrap();
        server_tick.map(|x| {
            self.game.propagate_state(game::TimedState {
                time_us : x.time_us - crossy_multi_core::STATIC_LAG,
                player_states : x.states,
            }, self.local_player_id);
        });

        // Tick logic
        let mut player_inputs = self.game.get_last_player_inputs();
        player_inputs.set(self.local_player_id, input);
        self.game.tick_current_time(Some(player_inputs), current_time.as_micros() as u32);

        if (input != game::Input::None)
        {
            self.send(input, current_time.as_micros() as u32);
        }

        let server_tick = self.recv();
    }

    fn recv(&mut self) -> Result<Option<ServerTick>>
    {
        match crossy_receive(&mut self.socket) {
            Ok((CrossyMessage::ServerTick(server_tick), _)) =>
            {
                Ok(Some(server_tick))
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No messages
                Ok(None)
            },
            Err(e) => Err(e),
            _ => Ok(None),
        }
    }

    fn send(&mut self, input : game::Input, time_us : u32)
    {
        let client_update = CrossyMessage::ClientTick(ClientTick{
            time_us : time_us,
            input : input,
        });

        crossy_send(&client_update, &mut self.socket, &self.server).unwrap();
    }
}