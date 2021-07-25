use super::game;
use super::interop::*;
use super::timeline::Timeline;

use std::fs::File;
use std::io::{Result};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Instant;
use std::cell::RefCell;

use crate::{DEBUG_LOGGER};

const ENABLE_DEBUG_LOGGING: bool = false;

fn create_debug_logger(id: super::PlayerId) {
    if ENABLE_DEBUG_LOGGING {
        let now = std::time::Instant::now();
        let file = File::create(
            "C:\\users\\dan\\crossy_multi\\logs\\client_".to_owned()
                + &format!("{:?}", now)
                + "_"
                + &id.0.to_string()
                + ".log",
        )
        .unwrap();

        unsafe { DEBUG_LOGGER.file = Some(RefCell::new(file)) };
    }
}

pub struct Client {
    server: SocketAddr,
    socket: UdpSocket,
    pub timeline: Timeline,
    pub local_player_id: game::PlayerId,
    start: Instant,
    last_tick: u32,
}

fn estimate_offset(socket: &mut UdpSocket, ping_addr: &SocketAddr) -> Result<u32> {
    const COUNT: usize = 4;
    let start = Instant::now();
    let mut offsets = Vec::new();

    for _ in 0..COUNT {
        println!("Pinging");
        let t0 = Instant::now().saturating_duration_since(start).as_micros() as u32;
        let ping = CrossyMessage::OffsetPing();
        crossy_send(&ping, socket, ping_addr).unwrap();

        if let (CrossyMessage::OffsetPong(pong), _) = crossy_receive(socket)? {
            let t3 = Instant::now().saturating_duration_since(start).as_micros() as u32;
            let t1 = pong.us_server;
            // Assume no time difference between server send / receive
            //let t2 = pong.us_server;
            println!("Server time {}", t1);

            //let offset = ((t1 - t0) + (t2 - t3)) / 2;
            let offset = (t3 - t0) / 2;
            offsets.push(offset);
            println!("Offset {}", offset);
        }
    }

    offsets.sort();
    let offset = offsets[COUNT / 2];
    println!("Estimated Offset {}", offset);
    Ok(offset)
}

fn connect(
    socket: &mut UdpSocket,
    addr: &SocketAddr,
    ping_addr: &SocketAddr,
) -> Result<(ServerTick, u32, game::PlayerId, Instant)> {
    println!("Connecting..");
    let latency = estimate_offset(socket, ping_addr)?;
    let time_start = Instant::now();
    let hello = CrossyMessage::Hello(ClientHello::new(latency));
    crossy_send(&hello, socket, addr).unwrap();

    let init_response = match crossy_receive(socket)? {
        (CrossyMessage::HelloResponse(response), _) => response,
        x => panic!("Got unexpected response from server {:?}", x),
    };

    // TODO this is wrong because it messes up timings
    // but also need to sync start
    //let time_start = Instant::now();

    let first_tick = crossy_receive(socket)?;
    match first_tick {
        (CrossyMessage::ServerTick(tick), _) => Ok((
            tick,
            init_response.seed,
            init_response.player_id,
            time_start,
        )),
        x => panic!("Got unexpected response from server {:?}", x),
    }
}

impl Client {
    pub fn try_create(port: u16) -> Result<Self> {
        let mut socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))?;
        let server = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8085));
        let server_ping = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8086));

        let (server_tick, seed, local_player_id, time_start) =
            connect(&mut socket, &server, &server_ping)?;

        println!(
            "Connected! Our id {:?}, seed {}, server response {:?}",
            local_player_id, seed, server_tick
        );

        socket.set_nonblocking(true)?;

        let timeline = Timeline::from_server_parts(
            seed,
            server_tick.latest.time_us,
            server_tick.latest.states,
            crate::crossy_ruleset::CrossyRulesetFST::start(),
        );

        create_debug_logger(local_player_id);
        crate::debug_log(&format!("Hello! Player {:?}", local_player_id));

        Ok(Client {
            server,
            socket,
            timeline,
            local_player_id,
            start: time_start,
            last_tick: server_tick.latest.time_us,
        })
    }

    pub fn tick(&mut self, input: game::Input) {
        let tick_start = Instant::now();
        let current_time = tick_start.saturating_duration_since(self.start);
        self.last_tick = current_time.as_micros() as u32;

        // Tick logic
        let mut player_inputs = self.timeline.get_last_player_inputs();
        player_inputs.set(self.local_player_id, input);
        self.timeline
            .tick_current_time(Some(player_inputs), current_time.as_micros() as u32);

        // Pop all server messages off queue
        // Take last
        let mut server_tick = None;
        while let Some(tick) = self.recv().unwrap() {
            server_tick = Some(tick);
        }

        server_tick.map(|server_tick| {
            //DEBUG_LOGGER
                //.log(&format!("Client Last = {:?}", &x.last_client_sent));

            self.timeline.propagate_state(
                &server_tick.latest,
                None,
                server_tick.last_client_sent.get(self.local_player_id),
                Some(self.local_player_id));

            //DEBUG_LOGGER
                //.log(&format!("Top: {:?}", &self.timeline.top_state()));
        });

        self.send(input, current_time.as_micros() as u32);
    }

    fn recv(&mut self) -> Result<Option<ServerTick>> {
        match crossy_receive(&mut self.socket) {
            Ok((CrossyMessage::ServerTick(server_tick), _)) => Ok(Some(server_tick)),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No messages
                Ok(None)
            }
            Err(e) => Err(e),
            _ => Ok(None),
        }
    }

    fn send(&mut self, input: game::Input, time_us: u32) {
        let client_update = CrossyMessage::ClientTick(ClientTick {
            time_us,
            input,
            lobby_ready : false,
        });

        crossy_send(&client_update, &mut self.socket, &self.server).unwrap();
    }
}
