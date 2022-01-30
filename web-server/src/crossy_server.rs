use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

use crossy_multi_core::game;
use crossy_multi_core::timeline::{Timeline, RemoteInput, RemoteTickState};
use crossy_multi_core::interop::*;
use crossy_multi_core::player_id_map::PlayerIdMap;


const SERVER_VERSION: u8 = 1;
const DESIRED_TICK_TIME: Duration = Duration::from_millis(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketId(pub u32);

struct PlayerClient {
    id: game::PlayerId,
    last_tick_us : u32,
}

struct Client {
    player_client : Option<PlayerClient>,
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
    next_socket_id : SocketId,
}

impl Server {
    pub fn new(id : u64) -> Self {
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
                timeline: Timeline::from_seed(1000 + id as u32),
                prev_tick: start,
                start,
                next_socket_id : SocketId(0),
            }),
        }
    }

    pub async fn queue_message(&self, message : CrossyMessage, player : SocketId) {
        let mut guard = self.queued_messages.lock().await;
        guard.push((message, player));
    }

    pub async fn get_server_description(&self) -> ServerDescription {
        let inner = self.inner.lock().await;
        ServerDescription {
            server_version : SERVER_VERSION,
            seed : inner.timeline.map.get_seed(),
        }
    }

    pub async fn join(&self) -> SocketId {
        println!("/join");
        let mut inner = self.inner.lock().await;
        inner.add_client()
    }

    pub async fn play(&self, hello : &ClientHello, socket_id: SocketId) -> Option<InitServerResponse>
    {
        println!(
            "/play {:?} {:?} looks ok: {}",
            socket_id,
            &hello,
            hello.check(1)
        );

        let mut inner = self.inner.lock().await;

        let client_id = game::PlayerId(inner.clients.len() as u8);
        inner.new_players.push(client_id);

        // Fails if socket_id not found
        // In prod version dont crash here?
        let mut client = inner.get_client_mut_by_addr(socket_id).expect("client tried to /play without calling /join");
        client.player_client = Some(PlayerClient {
            id: client_id,
            last_tick_us : 0,
        });

        Some(InitServerResponse {
            server_version: SERVER_VERSION,
            //player_count: inner.timeline.player_count,
            // unused I think, clean up
            player_count: 0,
            seed: inner.timeline.map.get_seed(),
            player_id: client_id,
        })
    }

    pub fn get_listener(&self) -> tokio::sync::watch::Receiver<CrossyMessage> {
        self.outbound_rx.clone()
    }

    pub async fn get_last_frame_time_us(&self) -> u32 {
        let inner = self.inner.lock().await;
        inner.timeline.top_state().time_us
    }

    pub async fn run(&self) {
        // Still have client listeners
        while !self.outbound_tx.is_closed() {

            let tick_start = Instant::now();
            let (client_updates, dropped_players, ready_players) = self.receive_updates().await;

            let mut inner = self.inner.lock().await;

            // Fetch + clear list of new players
            let new_players = std::mem::take(&mut inner.new_players);

            // Do simulations
            let simulation_time_start = Instant::now();
            let dt_simulation = simulation_time_start.saturating_duration_since(inner.prev_tick);
            inner.prev_tick = simulation_time_start;

            inner.timeline.tick(None, dt_simulation.as_micros() as u32);
            inner.timeline.propagate_inputs(client_updates);

            for new_player in new_players {
                // We need to make sure this gets propagated properly
                // Weird edge case bugs
                println!("In run, adding a new player {new_player:?}");
                let spawn_pos = find_spawn_pos(inner.timeline.top_state());
                println!("Spawning new player at {spawn_pos:?}");
                inner.timeline.add_player(new_player, spawn_pos);
            }

            for dropped_player in dropped_players {
                println!("Dropping player {dropped_player:?}");
                inner.timeline.remove_player(dropped_player);
            }

            for (ready_player, ready) in ready_players {
                inner.timeline.set_player_ready(ready_player, ready);
            }

            // Send responses
            let top_state = inner.timeline.top_state();

            let mut last_client_sent = PlayerIdMap::new();
            for client in (&inner.clients).iter().filter_map(|x| x.player_client.as_ref()) {
                inner.timeline.get_state_before_eq_us(client.last_tick_us).map(|x| {
                    last_client_sent.set(client.id, RemoteTickState {
                        time_us: x.time_us,
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

                // If an input comes in late that affects state change then this ignores it
                // Do we care?
                // Do we need some lookback period here? 
                rule_state : top_state.get_rule_state().clone(),
            });

            if top_state.frame_id as usize % 300 == 0 {
                //println!("Sending tick {:?}", tick);
            }

            self.outbound_tx.send(tick).unwrap();

            let now = Instant::now();
            let elapsed_time = now.saturating_duration_since(tick_start);
            if let Some(sleep_time) = DESIRED_TICK_TIME.checked_sub(elapsed_time) {
                tokio::time::sleep(sleep_time).await;
            }
        }
    }

    async fn receive_updates(&self) -> (Vec<RemoteInput>, Vec<game::PlayerId>, Vec<(game::PlayerId, bool)>) {
        let mut queued_messages = Vec::with_capacity(8);

        let mut guard = self.queued_messages.lock().await;
        std::mem::swap(&mut queued_messages, &mut guard);
        drop(guard);

        let mut client_updates = Vec::new();
        let mut ready_players = Vec::new();

        let mut inner = self.inner.lock().await;

        while let Some((message, socket_id)) = queued_messages.pop()
        {
            match message {
                CrossyMessage::ClientTick(t) => match inner.get_client_mut_by_addr(socket_id) {
                    Some(client) => {
                        if let Some(player_client) = client.player_client.as_mut()
                        {
                            let client_time = t.time_us;
                            player_client.last_tick_us = client_time;

                            ready_players.push((player_client.id, t.lobby_ready));

                            client_updates.push(RemoteInput {
                                time_us: client_time,
                                input: t.input,
                                player_id: player_client.id,
                            });
                        }
                        else {
                            println!("Received client update from client who has not called /play");
                        }
                    }
                    None => {
                        println!("Did not recognise addr {:?}", &socket_id);
                    }
                },
                _ => {}
            }
        }

        // TODO dropped players
        let dropped_players = vec![];
        (client_updates, dropped_players, ready_players)
    }
}

impl ServerInner {
    fn add_client(&mut self) -> SocketId {
        let socket_id = self.next_socket_id;
        self.next_socket_id = SocketId(socket_id.0 + 1);
        self.clients.push(Client {
            player_client : None,
            socket_id,
        });

        socket_id
    }

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

fn find_spawn_pos(game_state : &crossy_multi_core::game::GameState) -> crossy_multi_core::Pos {
    for x in 7..=13 {
        for y in 7..=13 {
            let spawn_pos = game::Pos::new_coord(x, y);
            if (!game_state.space_occupied_with_player(spawn_pos, None))
            {
                return spawn_pos;
            }
        }
    }

    panic!("Impossible, without 36 players");
}