#![allow(unused_parens)]
#![allow(dead_code)]

use crossy_multi_core::*;
use std::sync::Arc;

use warp::Filter;
use warp::http::StatusCode;
use warp::reply::{Reply, Response, self};
use warp::ws::{self, Message, WebSocket};
use warp::reject::{self, Rejection};
use serde_derive::{Deserialize};
use tokio_stream::wrappers::UnboundedReceiverStream;

use tokio::sync::Mutex;
use futures::{FutureExt, StreamExt};

mod crossy_server;

// https://github.com/seanmonstar/warp/blob/master/examples/todos.rs

#[derive(Clone)]
struct GameDbInner {
    id: u64,
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

    async fn new_game(&self) -> u64 {
        let mut games = self.games.lock().await;
        let prev_max_id = games.iter().map(|x| x.id).max().unwrap_or(0);
        let id = prev_max_id + 1;

        let game = Arc::new(crossy_server::Server::new(id));
        games.push(GameDbInner {
            id,
            game : game.clone(),
        });

        tokio::task::spawn(async move {
            game.run().await;
        });

        id
    }

    async fn get(&self, game_id : u64) -> Result<GameDbInner, Rejection> {
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

    // GET /join
    let get_join = warp::path!("join")
        .and(warp::get())
        .and(warp::query::<JoinOptions>())
        .and(with_db(games.clone()))
        .and_then(join_handler).boxed();
     
    let websocket =
    warp::path!("ws")
        .and(warp::ws())
        .and(warp::query::<WebSocketJoinOptions>())
        .and(with_db(games.clone()))
        .and_then(ws_handler).boxed();
   
    let routes = get_new
        .or(get_join)
        .or(site)
        .or(websocket);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080))
        .await;
}

#[derive(Debug, Clone, Deserialize)]
struct JoinOptions {
    pub game_id : u64, 
    pub name : String, 
}

#[derive(Debug, Clone, Deserialize)]
struct WebSocketJoinOptions {
    pub game_id : u64, 
    pub player_id : u64, 
}

fn with_db(db: GameDb) -> impl Filter<Extract = (GameDb,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

async fn new_game_handler(db: GameDb) -> Result<Response, std::convert::Infallible>  {
    let id = db.new_game().await;
    Ok(warp::reply::json(&id).into_response())
}

async fn join_handler(options : JoinOptions, db: GameDb) -> Result<Response, Rejection>  {
    let dbinner = db.get(options.game_id).await?;
    // TMP TO HACK SOMETHGING WORKING
    let hello = interop::ClientHello::new(15_000);
    let init_server_response = dbinner.game.join(&hello, crossy_server::SocketId(0)).await;
    Ok(reply::json(&init_server_response).into_response())
}

async fn ws_handler(ws : ws::Ws, options: WebSocketJoinOptions, db : GameDb) -> Result<Response, Rejection> {
    println!("WS Handler");
    let game = db.get(options.game_id).await?;
    let listener = game.game.get_listener();

    Ok(ws.on_upgrade(move |socket| {
        websocket_main(socket, listener)
    }).into_response())
}

async fn websocket_main(ws: WebSocket, mut tick_listener: tokio::sync::watch::Receiver<interop::CrossyMessage>) {
    //let (client_ws_sender, mut client_ws_rcv) = ws.
    println!("Websocket conencted");

    let (ws_tx, _ws_rx) = ws.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(rx.forward(ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    loop {
        match tick_listener.changed().await {
            Ok(_) => {
                let x : interop::CrossyMessage = (*tick_listener.borrow()).clone();
                let formatted = format!("{:#?}", x);
                match tx.send(Ok(Message::text(formatted))) {
                    Ok(_) => {},
                    Err(e) => {println!("{}", e)}
                }
            },
            // Handle dropped so game ended?
            Err(e) => {
                println!("1 Connection dropped? {}", e)
            }
        }
    }

    //println!("Connection ended");
}

