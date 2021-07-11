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

async fn crossy_server_loop()
{

}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let games = GameDb::new();

    println!("Serving...");

    let serve_dir = "C:\\Users\\Dan\\decrypto\\site";
    let site = warp::fs::dir(serve_dir).with(warp::compression::gzip()).boxed();

    // GET /new

    let get_new = warp::path!("new")
        .and(warp::get())
        .and(with_db(games.clone()))
        .and(warp::any())
        .and_then(new_game_handler).boxed();
    
    let routes = get_new
        .or(site)
        //.or(get_join_game)
        //.or(get_words)
        //.or(get_state)
        //.or(post_ready)
        //.or(post_clues)
        //.or(websocket);
        ;

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080))
        .await;
}

/*
#[derive(Debug, Clone, Deserialize)]
struct JoinOptions {
    pub game_id : u64, 
    pub name : String, 
}

#[derive(Debug, Clone, Deserialize)]
struct GamePlayerOptions {
    pub game_id : u64, 
    pub player_id : u64, 
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    let games = GameDb::new();

    let words_path = "C:\\Users\\Dan\\decrypto\\wordlist.txt";
    let words = Arc::new(WordList::read_file(words_path));

    println!("Serving...");

    let serve_dir = "C:\\Users\\Dan\\decrypto\\site";
    let site = warp::fs::dir(serve_dir).with(warp::compression::gzip()).boxed();

    // GET /new
    let get_new = warp::path!("new")
        .and(warp::get())
        .and(with_db(games.clone()))
        .and(warp::any()
        .map(move || words.clone()))
        .and_then(new_game_handler).boxed();
    
    // GET /join?game_id=5
    let get_join_game = warp::path!("join")
        .and(warp::get())
        .and(warp::query::<JoinOptions>())
        .and(with_db(games.clone()))
        .and_then(join_handler).boxed();

    // GET /words?game_id=5&player_id=3
    let get_words = 
        warp::path!("words")
            .and(warp::get())
            .and(warp::query::<GamePlayerOptions>())
            .and(with_db(games.clone()))
            .and_then(get_words_handler).boxed();

    // GET /state?game=5&player_id=3
    let get_state = 
        warp::path!("state")
            .and(warp::get())
            .and(warp::query::<GamePlayerOptions>())
            .and(with_db(games.clone()))
            .and_then(get_state_handler).boxed();

    // POST /ready?game_id=5&player_id=3
    // { ready : true }
    let post_ready = 
        warp::path!("ready")
            .and(warp::post())
            .and(warp::query::<GamePlayerOptions>())
            .and(warp::filters::body::json())
            .and(with_db(games.clone()))
            .and_then(post_ready_handler).boxed();

    // POST /clues?game_id=5&player_id=3
    // { clues : ["one", "two", "three"] }
    let post_clues = 
        warp::path!("clues")
            .and(warp::post())
            .and(warp::query::<GamePlayerOptions>())
            .and(warp::filters::body::json())
            .and(with_db(games.clone()))
            .and_then(post_clues_handler).boxed();
    
    let websocket =
        warp::path!("ws")
            .and(warp::ws())
            .and(warp::query::<GamePlayerOptions>())
            .and(with_db(games.clone()))
            .and_then(ws_handler).boxed();

    let routes = get_new
        .or(site)
        .or(get_join_game)
        .or(get_words)
        .or(get_state)
        .or(post_ready)
        .or(post_clues)
        .or(websocket);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080))
        .await;
}
*/

fn with_db(db: GameDb) -> impl Filter<Extract = (GameDb,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

async fn new_game_handler(db: GameDb) -> Result<Response, std::convert::Infallible>  {
    let id = db.new_game().await;
    Ok(warp::reply::json(&id).into_response())
}

/*
async fn join_handler(options : JoinOptions, db: GameDb) -> Result<Response, Rejection>  {
    let game = db.get(options.game_id).await?;
    let id = game.game.lock().await.add_player(options.name);
    Ok(reply::json(&id).into_response())
}

async fn get_words_handler(options: GamePlayerOptions, db: GameDb) -> Result<Response, Rejection>  {
    let game = db.get(options.game_id).await?;
    let g = game.game.lock().await;

    match g.get_team(options.player_id) {
        Some(t) => {
            Ok(reply::json(&t.words).into_response())
        }
        None => {
            Ok(StatusCode::FORBIDDEN.into_response())
        }
    }
}

async fn get_state_handler(options: GamePlayerOptions, db: GameDb) -> Result<Response, Rejection>  {
    let game = db.get(options.game_id).await?;
    let state = game.game.lock().await.get_visible_state();
    Ok(reply::json(&state).into_response())
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
struct PostReadyParams {
    ready : bool,
}

async fn post_ready_handler(options: GamePlayerOptions, params : PostReadyParams, db: GameDb) -> Result<Response, Rejection>  {
    let inner = db.get(options.game_id).await?;
    let mut game = inner.game.lock().await;
    if game.submit_ready_state(options.player_id, params.ready) {
        Ok(StatusCode::OK.into_response())
    }
    else {
        Ok(StatusCode::FORBIDDEN.into_response())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PostCluesParams {
    clues : [String;3],
}

async fn post_clues_handler(options: GamePlayerOptions, params : PostCluesParams, db: GameDb) -> Result<Response, Rejection>  {
    let inner = db.get(options.game_id).await?;
    let mut game = inner.game.lock().await;
    println!("guess req {:?} {:?}", &options, &params);
    if game.submit_clues(options.player_id, params.clues) {
        println!("Submitting guesses ok");
        Ok(StatusCode::OK.into_response())
    }
    else {
        println!("invalid guess req");
        Ok(StatusCode::FORBIDDEN.into_response())
    }
}

async fn ws_handler(ws : ws::Ws, options: GamePlayerOptions, db : GameDb) -> Result<Response, Rejection> {
    println!("WS Handler");
    let game = db.get(options.game_id).await?;
    let listener = game.game.lock().await.get_listener();

    Ok(ws.on_upgrade(move |socket| {
        websocket_main(socket, listener)
    }).into_response())
}

async fn websocket_main(ws: WebSocket, mut change_listener: tokio::sync::watch::Receiver<GameNotification>) {
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
        match change_listener.changed().await {
            Ok(_) => {
                match tx.send(Ok(Message::text("Update!"))) {
                    Ok(_) => {},
                    Err(e) => {println!("{}", e)}
                }
            }
            // Handle dropped so game ended?
            Err(e) => {
                println!("1 Connection dropped? {}", e)
            }
        }
    }

    //println!("Connection ended");
}
*/