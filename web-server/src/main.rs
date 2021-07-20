#![allow(unused_parens)]
#![allow(dead_code)]

use crossy_multi_core::*;
use std::sync::Arc;

use warp::Filter;
use warp::http::StatusCode;
use warp::reply::{Reply, Response, self};
use warp::ws::{self, Message, WebSocket};
use warp::reject::{self, Rejection};
use tokio_stream::wrappers::UnboundedReceiverStream;

use serde::{Serialize, Deserialize};
use std::error::Error;

use tokio::sync::Mutex;
use futures::{FutureExt, StreamExt};

mod crossy_server;

// https://github.com/seanmonstar/warp/blob/master/examples/todos.rs

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameId(u64);

#[derive(Clone)]
struct GameDbInner {
    id: GameId,
    game : Arc<crossy_server::Server>,
}

#[derive(Clone)]
struct GameDb {
    games : Arc<Mutex<Vec<GameDbInner>>>,
}

impl GameDb {
    fn new() -> Self {
        GameDb {
            games: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn new_game(&self) -> GameId {
        let mut games = self.games.lock().await;
        let prev_max_id = games.iter().map(|x| x.id.0).max().unwrap_or(0);
        let id = GameId(prev_max_id + 1);

        let game = Arc::new(crossy_server::Server::new(id.0));
        games.push(GameDbInner {
            id,
            game : game.clone(),
        });

        tokio::task::spawn(async move {
            game.run().await;
        });

        id
    }

    async fn get(&self, game_id : GameId) -> Result<GameDbInner, Rejection> {
        let games = self.games.lock().await;
        let m_game = games.iter().filter(|x| x.id == game_id).next();
        match m_game {
            Some(x) => {
                Ok(x.clone())
            },
            None => {
                Err(reject::not_found())
            },
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let games = GameDb::new();

    println!("Serving...");

    let serve_dir = "C:\\Users\\Dan\\crossy_multi\\web-client\\dist";
    let site = warp::fs::dir(serve_dir).with(warp::compression::gzip()).boxed();

    // GET /new
    let get_new = warp::path!("new")
        .and(warp::get())
        .and(with_db(games.clone()))
        .and(warp::any())
        .and_then(new_game_handler).boxed();

    // GET /join?game_id=1&name=dan
    let get_join = warp::path!("join")
        .and(warp::get())
        .and(warp::query::<JoinOptions>())
        .and(with_db(games.clone()))
        .and_then(join_handler).boxed();

    // GET /play?game_id=1&socket_id=1
    let get_play = warp::path!("play")
        .and(warp::get())
        .and(warp::query::<PlayOptions>())
        .and(with_db(games.clone()))
        .and_then(play_handler).boxed();
     
    // WS /ws?game_id=1&socket_id=1
    let websocket =
    warp::path!("ws")
        .and(warp::ws())
        .and(warp::query::<WebSocketJoinOptions>())
        .and(with_db(games.clone()))
        .and_then(ws_handler).boxed();
   
    let routes = get_new
        .or(get_join)
        .or(get_play)
        .or(site)
        .or(websocket);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080))
        .await;
}

fn with_db(db: GameDb) -> impl Filter<Extract = (GameDb,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

#[derive(Debug, Clone, Serialize)]
pub struct NewGameResponse
{
    pub game_id : GameId,
}

async fn new_game_handler(db: GameDb) -> Result<Response, std::convert::Infallible>  {
    let game_id = db.new_game().await;
    //let response = serde_json::to_string(&id).unwrap();
    let new_game_response = NewGameResponse { game_id };
    let mut response = warp::reply::json(&new_game_response).into_response();
    let _ = response.headers_mut().insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    let _ = response.headers_mut().insert("Access-Control-Allow-Credentials", "true".parse().unwrap());
    //println!("/new {:?}", &response_json);
    println!("/new {:?}", &response);
    Ok(response)
}

#[derive(Debug, Clone, Deserialize)]
struct JoinOptions {
    pub game_id : GameId, 
    pub name : String, 
}

#[derive(Debug, Clone, Serialize)]
struct JoinResponse {
    pub socket_id : crossy_server::SocketId,
    pub server_description : interop::ServerDescription,
    pub server_time_us : u32,
}

async fn join_handler(options : JoinOptions, db: GameDb) -> Result<Response, Rejection>  {
    let dbinner = db.get(options.game_id).await?;
    let server_description = dbinner.game.get_server_description().await;
    let last_frame_time_us = dbinner.game.get_last_frame_time_us().await;
    let socket_id = dbinner.game.join().await;
    let response = JoinResponse {
        socket_id,
        server_description,
        server_time_us : last_frame_time_us,
    };

    Ok(reply::json(&response).into_response())
}

#[derive(Debug, Clone, Deserialize)]
struct PlayOptions {
    pub game_id : GameId, 
    pub socket_id : crossy_server::SocketId,
}

async fn play_handler(options: PlayOptions, db: GameDb) -> Result<Response, Rejection>  {
    println!("Play with options {:?}", options);
    let dbinner = db.get(options.game_id).await?;
    let hello = interop::ClientHello::new(15_000);
    let init_server_response = dbinner.game.play(&hello, options.socket_id).await;
    Ok(reply::json(&init_server_response).into_response())
}

#[derive(Debug, Clone, Deserialize)]
struct WebSocketJoinOptions {
    pub game_id : GameId, 
    pub socket_id : crossy_server::SocketId, 
}

async fn ws_handler(ws : ws::Ws, options: WebSocketJoinOptions, db : GameDb) -> Result<Response, Rejection> {
    println!("WS Handler");

    let game = db.get(options.game_id).await?;

    Ok(ws.on_upgrade(move |socket| {
        websocket_main(socket, game, options.socket_id)
    }).into_response())
}

async fn websocket_main(ws: WebSocket, db : GameDbInner, socket_id : crossy_server::SocketId) {
    //let (client_ws_sender, mut client_ws_rcv) = ws.
    println!("Websocket conencted");

    let mut tick_listener = db.game.get_listener();

    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(rx.forward(ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    tokio::task::spawn(async move {
        loop {
            match tick_listener.changed().await {
                Ok(_) => {
                    //let x : interop::CrossyMessage = (*tick_listener.borrow()).clone();
                    //let formatted = format!("{:#?}", x);
                    let serialized = flexbuffers::to_vec(&(*tick_listener.borrow())).unwrap();
                    //match tx.send(Ok(Message::text(formatted))) {
                    match tx.send(Ok(Message::binary(serialized))) {
                        Ok(_) => {},
                        Err(e) => {println!("send error {}", e); break;}
                    }
                },
                // Handle dropped so game ended?
                Err(e) => {
                    println!("1 Connection dropped? {}", e);
                    break;
                }
            }
        }
    });

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) =>
            {
                match parse_client_message(&msg)
                {
                    Some(message) => {
                        //println!("{:?}", message);
                        db.game.queue_message(message, socket_id).await;
                    }
                    _ => {},
                }
            }
            Err(e) => {
                println!("Client receive err {}", e);
                break;
            }
        }
    }

    println!("Client disconnected");
    //println!("Connection ended");
}

fn parse_client_message(ws_message : &warp::ws::Message) -> Option<interop::CrossyMessage>
{
    let bytes = ws_message.as_bytes();
    let r = flexbuffers::Reader::get_root(bytes).map_err(|e| println!("{}", e)).ok()?;
    interop::CrossyMessage::deserialize(r).map_err(|e| println!("{}", e)).ok()
}

