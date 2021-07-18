use crossy_multi_core::game;
use crossy_multi_core::timeline::{Timeline, RemoteInput, RemoteTickState};
use crossy_multi_core::interop::*;

use std::time::{Duration, Instant};

use tokio::sync::Mutex;

const SERVER_VERSION: u8 = 1;
const DESIRED_TICK_TIME: Duration = Duration::from_millis(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SocketId(pub u32);

struct Client {
    id: game::PlayerId,
    offset_us: u32,
    last_tick_us : u32,
    socket_id : SocketId,
}

pub struct Server {
    queued_messages : Mutex<Vec<(CrossyMessage, SocketId)>>,
    inner : Mutex<ServerInner>,

    outbound_tx : tokio::sync::watch::Sender<CrossyMessage>,
    outbound_rx : tokio::sync::watch::Receiver<CrossyMessage>,
}

pub struct ServerInner {
    new_players : Vec<game::PlayerId>,
    start: Instant,
    prev_tick: Instant,
    clients: Vec<Client>,
    timeline: Timeline,
}

impl Server {
    pub fn new(_id : u64) -> Self {
        let start = Instant::now();
        let init_message = CrossyMessage::EmptyMessage();
        let (outbound_tx, outbound_rx) = tokio::sync::watch::channel(init_message);

        Server {
            queued_messages : Mutex::new(Vec::new()),
            outbound_tx,
            outbound_rx,
            inner : Mutex::new(ServerInner {
                clients: Vec::new(),
                new_players: Vec::new(),
                timeline: Timeline::new(),
                prev_tick: start,
                start: start,
            }),
        }
    }

    pub async fn queue_message(&self, message : CrossyMessage, player : SocketId) {
        let mut guard = self.queued_messages.lock().await;
        guard.push((message, player));
    }

    pub async fn join(&self) -> ServerDescription {
        let mut inner = self.inner.lock().await;
        ServerDescription {
            server_version : SERVER_VERSION,
            seed : inner.timeline.seed,
        }
    }

    pub async fn play(&self, hello : &ClientHello, socket_id: SocketId) -> InitServerResponse
    {
        println!(
            "Player joined! {:?} {:?} looks ok: {}",
            socket_id,
            &hello,
            hello.check(1)
        );

        let mut inner = self.inner.lock().await;

        let client_id = game::PlayerId(inner.clients.len() as u8);
        inner.new_players.push(client_id);

        // TODO do we care about sub-frame time here?
        let now = Instant::now();
        let client_offset_us =
            now.saturating_duration_since(inner.start).as_micros() as u32
                - hello.latency_us;
        println!("Client offset us {}", client_offset_us);

        let client = Client {
            socket_id,
            id: client_id,
            // TODO ping the client and add that.
            offset_us: client_offset_us,
            last_tick_us : 0,
        };

        inner.clients.push(client);

        InitServerResponse {
            server_version: SERVER_VERSION,
            player_count: inner.timeline.player_count,
            seed: inner.timeline.seed,
            player_id: client_id,
        }
    }

    pub fn get_listener(&self) -> tokio::sync::watch::Receiver<CrossyMessage> {
        self.outbound_rx.clone()
    }

    pub async fn run(&self) {
        // Still have client listeners
        while !self.outbound_tx.is_closed() {

            let tick_start = Instant::now();
            let (client_updates, dropped_players) = self.receive_updates(&tick_start).await;

            let mut inner = self.inner.lock().await;

            // Fetch + clear list of new players
            let mut new_players = Vec::new();
            std::mem::swap(&mut inner.new_players, &mut new_players);

            // Do simulations
            let simulation_time_start = Instant::now();
            let dt_simulation = simulation_time_start.saturating_duration_since(inner.prev_tick);
            inner.prev_tick = simulation_time_start;

            inner.timeline.tick(None, dt_simulation.as_micros() as u32);
            inner.timeline.propagate_inputs(client_updates);

            for new_player in new_players {
                // We need to make sure this gets propagated properly
                // Weird edge case bugs
                inner.timeline.add_player(new_player, game::Pos::new_coord(10, 10));
            }

            for dropped_player in dropped_players {
                inner.timeline.remove_player(dropped_player);
            }

            // Send responses
            let top_state = inner.timeline.top_state();

            let mut last_client_sent = crossy_multi_core::interop::LastClientSentTicks::new();
            for client in &inner.clients {
                inner.timeline.get_state_before_eq_us(client.last_tick_us).map(|x| {
                    last_client_sent.set(client.id, RemoteTickState {
                        time_us: x.time_us - client.offset_us,
                        states: x.get_valid_player_states(),
                    });
                });
            }

            let tick = CrossyMessage::ServerTick(ServerTick {
                latest: RemoteTickState {
                    time_us: top_state.time_us,
                    states: top_state.get_valid_player_states(),
                },
                last_client_sent,
            });

            if top_state.frame_id as usize % 300 == 0 {
                println!("Sending tick {:?}", tick);
            }

            self.outbound_tx.send(tick).unwrap();

            let now = Instant::now();
            let elapsed_time = now.saturating_duration_since(tick_start);
            match DESIRED_TICK_TIME.checked_sub(elapsed_time) {
                Some(sleep_time) => {
                    tokio::time::sleep(sleep_time).await;
                }
                None => {},
            }
        }
    }

    async fn receive_updates(&self, tick_start: &Instant) -> (Vec<RemoteInput>, Vec<game::PlayerId>) {
        let mut queued_messages = Vec::with_capacity(8);

        let mut guard = self.queued_messages.lock().await;
        std::mem::swap(&mut queued_messages, &mut guard);
        drop(guard);

        let mut client_updates = Vec::new();
        let mut dropped_players = Vec::new();

        let mut inner = self.inner.lock().await;

        while let Some((message, socket_id)) = queued_messages.pop()
        {
            match message {

                CrossyMessage::ClientTick(t) => match inner.get_client_mut_by_addr(socket_id) {
                    Some(client) => {
                        let client_time = t.time_us + client.offset_us;
                        client.last_tick_us = client_time;

                        client_updates.push(RemoteInput {
                            time_us: client_time,
                            input: t.input,
                            player_id: client.id,
                        });
                    }
                    None => {
                        println!("Did not recognise addr {:?}", &socket_id);
                    }
                },
                _ => {}
            }
        }

        (client_updates, dropped_players)
    }
}

/*
fn get_client_mut_by_addr<'a>(guard : &'a mut tokio::sync::MutexGuard<Vec<Client>>, id: SocketId) -> Option<&'a mut Client> {
    for client in &mut (**guard) {
        if client.socket_id == id {
            return Some(client);
        }
    }

    None
}

fn get_client_by_addr<'a>(guard : &'a tokio::sync::MutexGuard<Vec<Client>>, id: SocketId) -> Option<&'a Client> {
    for client in &(**guard) {
        if client.socket_id == id {
            return Some(client);
        }
    }

    None
}
*/

impl ServerInner {
    fn get_client_mut_by_addr(&mut self, id: SocketId) -> Option<&mut Client> {
        for client in &mut self.clients {
            if client.socket_id == id {
                return Some(client);
            }
        }
    
        None
    }
    fn get_client_by_addr(&self, id: SocketId) -> Option<&Client> {
        for client in &self.clients {
            if client.socket_id == id {
                return Some(client);
            }
        }
    
        None
    }
}