use crossy_multi_core::game;
use crossy_multi_core::timeline::{Timeline, RemoteInput, RemoteTickState};
use crossy_multi_core::interop::*;

use std::io::Result;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const SERVER_VERSION: u8 = 1;
const DESIRED_TICK_TIME: Duration = Duration::from_millis(14);

fn main() {
    let socket_config = "127.0.0.1:8085";
    println!("Binding socket to {}", socket_config);
    let socket = UdpSocket::bind(socket_config).unwrap();
    socket.set_nonblocking(true).unwrap();

    let start = Instant::now();

    std::thread::spawn(move || ping_server(start));

    let mut s = Server {
        clients: vec![],
        timeline: Timeline::new(),
        socket,
        prev_tick: start,
        start: start,
    };

    match s.run() {
        Err(e) => {
            println!("Err {}", e)
        }
        _ => {}
    }
}

fn ping_server(start: Instant) {
    let ping_server_config = "127.0.0.1:8086";
    println!("Setting up ping socket on {}", ping_server_config);
    let mut socket = UdpSocket::bind(ping_server_config).unwrap();
    loop {
        match crossy_receive(&mut socket) {
            Ok((update, src)) => match update {
                CrossyMessage::OffsetPing() => {
                    let time_us =
                        Instant::now().saturating_duration_since(start).as_micros() as u32;
                    let pong = CrossyMessage::OffsetPong(OffsetPong { us_server: time_us });

                    crossy_send(&pong, &mut socket, &src).unwrap();
                }
                _ => {}
            },
            _ => {}
        }
    }
}

struct Client {
    addr: SocketAddr,
    id: game::PlayerId,
    offset_us: u32,
    last_tick_us : u32,
}

struct Server {
    clients: Vec<Client>,
    socket: UdpSocket,
    timeline: Timeline,
    prev_tick: Instant,
    start: Instant,
}

impl Server {
    fn receive_updates(
        &mut self,
        tick_start: &Instant,
    ) -> Result<(Vec<RemoteInput>, Vec<game::PlayerId>)> {
        let mut client_updates = Vec::new();
        let mut new_players = Vec::new();

        loop {
            match crossy_receive(&mut self.socket) {
                Ok((update, src)) => match update {
                    CrossyMessage::Hello(hello) => {
                        println!(
                            "Player joined! {} {:?} looks ok: {}",
                            src,
                            &hello,
                            hello.check(1)
                        );

                        let client_id = game::PlayerId(self.clients.len() as u8);
                        new_players.push(client_id);

                        let client_offset_us =
                            tick_start.saturating_duration_since(self.start).as_micros() as u32
                                - hello.latency_us;
                        println!("Client offset us {}", client_offset_us);

                        let client = Client {
                            addr: src,
                            id: client_id,
                            // TODO ping the client and add that.
                            offset_us: client_offset_us,
                            last_tick_us : 0,
                        };

                        self.clients.push(client);

                        let response = CrossyMessage::HelloResponse(InitServerResponse {
                            server_version: SERVER_VERSION,
                            player_count: self.timeline.player_count,
                            seed: self.timeline.seed,
                            player_id: client_id,
                        });

                        crossy_send(&response, &mut self.socket, &src)?;
                    }
                    CrossyMessage::ClientTick(t) => match self.get_client_mut_by_addr(&src) {
                        Some(client) => {
                            let client_time = t.time_us + client.offset_us;
                            client.last_tick_us = t.time_us;
                            client_updates.push(RemoteInput {
                                time_us: client_time,
                                input: t.input,
                                player_id: client.id,
                            });
                        }
                        None => {
                            println!("Did not recognise addr {}", &src);
                        }
                    },
                    _ => {}
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No more messages
                    return Ok((client_updates, new_players));
                }
                // Connection closed, todo cleanup player
                Err(ref e) if e.kind() == std::io::ErrorKind::ConnectionAborted => {
                    println!("Connection aborted")
                }
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    //println!("Connection reset {:?}", &e);
                    // tmp
                    //return Err(e);
                    //self.clients.retain(|x| x.addr != src);
                    // Clear the client
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn get_client_mut_by_addr(&mut self, addr: &SocketAddr) -> Option<&mut Client> {
        for client in &mut self.clients {
            if client.addr == *addr {
                return Some(client);
            }
        }

        None
    }

    fn get_client_by_addr(&self, addr: &SocketAddr) -> Option<&Client> {
        for client in &self.clients {
            if client.addr == *addr {
                return Some(client);
            }
        }

        None
    }

    // todo: return Result(!)
    fn run(&mut self) -> Result<()> {
        loop {
            let tick_start = Instant::now();
            let (client_updates, new_players) = self.receive_updates(&tick_start)?;

            // Do simulations
            let simulation_time_start = Instant::now();
            let dt_simulation = simulation_time_start.saturating_duration_since(self.prev_tick);
            self.prev_tick = simulation_time_start;
            self.timeline.tick(None, dt_simulation.as_micros() as u32);
            for new_player in new_players {
                self.timeline.add_player(new_player, game::Pos::new_coord(10, 10));
            }
            self.timeline.propagate_inputs(client_updates);

            // Send responses
            let top_state = self.timeline.top_state();

            for client in &self.clients {
                if client.offset_us > top_state.time_us {
                    panic!(
                        "Oh fuck, client offset {}, top state offset {}",
                        client.offset_us, top_state.time_us
                    )
                }

                let client_last_tick_state = self.timeline.get_state_before_eq_us(client.last_tick_us).map(|x| {
                    RemoteTickState {
                        time_us: client.last_tick_us,
                        states: x.get_valid_player_states(),
                    }
                });

                let tick = CrossyMessage::ServerTick(ServerTick {
                    latest: RemoteTickState {
                        time_us: top_state.time_us - client.offset_us,
                        states: top_state.get_valid_player_states(),
                    },
                    last_client_sent: client_last_tick_state,
                });

                println!("Sending tick {:?}", tick);

                crossy_send(&tick, &mut self.socket, &client.addr)?;
            }

            let now = Instant::now();
            let elapsed_time = now.saturating_duration_since(tick_start);

            match DESIRED_TICK_TIME.checked_sub(elapsed_time) {
                Some(sleep_time) => {
                    std::thread::sleep(sleep_time);
                }
                None => {},
            }
        }
    }
}
