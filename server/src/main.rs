extern crate crossy_multi_core;

use crossy_multi_core::interop::*;
use crossy_multi_core::game;

use std::io::Result;
use std::net::{UdpSocket, SocketAddr};
use std::time::{Duration, Instant};

const SERVER_VERSION : u8 = 1;

fn main() {
    let socket_config = "127.0.0.1:8081";
    println!("Binding socket to {}", socket_config);
    let socket = UdpSocket::bind(socket_config).unwrap();
    socket.set_nonblocking(true).unwrap();

    let mut s = Server {
        clients: vec![],
        socket : socket,
        game: game::Game::new(),
        prev_tick: Instant::now(),
        start: Instant::now(),
    };

    match s.run() {
        Err(e) => {
            println!("Err {}", e)
        },
        _ => {},
    }
}

struct Client
{
    addr : SocketAddr,
    id : game::PlayerId,
    offset_us : u32,
}

struct Server
{
    clients : Vec<Client>,
    socket : UdpSocket,
    game : game::Game,
    prev_tick : Instant,
    start : Instant,
}

// Run at >60hz
const DESIRED_TICK_TIME : Duration = Duration::from_millis(15);

impl Server
{
    fn receive_updates(&mut self) -> Result<(Vec<game::TimedInput>, Vec<game::PlayerId>)>
    {
        let mut client_updates = Vec::new();
        let mut new_players = Vec::new();

        loop
        {
            match crossy_receive(&mut self.socket)
            {
                Ok((update, src)) => match update
                {
                    CrossyMessage::Hello(hello) => {
                        println!("Player joined! {} {:?} looks ok: {}", src, &hello, hello.check(1));

                        let client_id = game::PlayerId(self.clients.len() as u8);
                        new_players.push(client_id);

                        let client = Client 
                        {
                            addr : src,
                            id : client_id,
                            // TODO ping the client and add that.
                            offset_us : Instant::now().saturating_duration_since(self.start).as_micros() as u32,
                        };

                        self.clients.push(client);

                        let response = CrossyMessage::HelloResponse(InitServerResponse
                        {
                            server_version : SERVER_VERSION,
                            player_count : self.clients.len() as u8,
                            player_id : client_id,
                        });

                        crossy_send(&response, &mut self.socket, &src)?;
                    },
                    CrossyMessage::ClientTick(t) => {
                        //println!("{:?}", &t);
                        match self.get_client_by_addr(&src)
                        {
                            Some(client) => {
                                client_updates.push(game::TimedInput
                                {
                                    time_us : t.time_us + client.offset_us,
                                    input : t.input,
                                    player_id : client.id,
                                });
                            }
                            None => {
                                println!("Did not recognise addr {}", &src);
                            }
                        }
                    },
                    _ => {},
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No more messages
                    return Ok((client_updates, new_players));
                },
                Err(e) => return Err(e),
            }

        }
    }

    fn get_client_by_addr(&self, addr : &SocketAddr) -> Option<&Client>
    {
        for client in &self.clients
        {
            if client.addr == *addr
            {
                return Some(client);
            }
        }

        None
    }

    // todo: return Result(!) 
    fn run(&mut self) -> Result<()>
    {
        loop
        {
            let tick_start = Instant::now();
            let (client_updates, new_players) = self.receive_updates()?;

            // Do simulations
            let simulation_time_start = Instant::now();
            let dt_simulation = simulation_time_start.saturating_duration_since(self.prev_tick);
            self.prev_tick = simulation_time_start;
            self.game.tick(None, dt_simulation.as_micros() as f64);
            for new_player in new_players
            {
                self.game.add_player(new_player);
            }
            self.game.propagate_inputs(client_updates);

            // Send responses
            let top_state = self.game.top_state();

            for client in &self.clients {
                let tick = CrossyMessage::ServerTick(ServerTick {
                    time_us : top_state.time_us - client.offset_us,
                    states : top_state.player_states.clone(),
                });
                crossy_send(&tick, &mut self.socket, &client.addr);
            }

            let now = Instant::now();
            let elapsed_time = now.saturating_duration_since(tick_start);

            //println!("Tick len {}", elapsed_time.as_micros());

            match DESIRED_TICK_TIME.checked_sub(elapsed_time)
            {
                Some(sleep_time) => {
                    //println!("Sleeping zzzz for {},", sleep_time.as_micros());
                    std::thread::sleep(sleep_time);
                },
                None => {
                    //println!("No time to waste!");
                }
            }
        }
    }
}