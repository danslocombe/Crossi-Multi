"use strict";

import { Client } from "../pkg/index.js"

const DEBUG = true;

const query_string = window.location.search;
const url_params = new URLSearchParams(query_string);
var game_id = url_params.get('game_id');

var player_name = "Dan";
var socket_id = 0;

var client = undefined;
var ws = undefined;
var estimated_latency_us = 0;
var current_input = "None";

var endpoint = "";
var ws_endpoint = "";

if (DEBUG)
{
    // Fetch from specific localhost / port in order to allow better debugging
    // (we host debug build from localhost:8081)
    // NOTE HAVE TO RUN CHROME WITH NO CORS
    endpoint = 'http://localhost:8080'
    ws_endpoint = 'ws://localhost:8080'
}

function dan_fetch(url) {
    return fetch(endpoint + url, {
        headers: {  'Accept': 'application/json' },
    });
}

/////////////////////////////////////////////////////////////////////////////////////

// Ping server to estimate latency before we start
console.log("Starting pinging");
var ping_ws = new WebSocket(ws_endpoint + "/ping");
ping_ws.onopen = evt => ping(true);
ping_ws.onmessage = evt => ping(false)
ping_ws.onclose = evt => {}
var ping_responses = [];
var remaining_pings = 5;

// t0 - time to send ping
// t1 - time server receives ping
// t2 - time server sends pong
// t3 - time client receives pong
// Assume zero time on the server for simplicity, so t2-t1 = 0
var ping_t0 = undefined;

function ping(initial_ping) {
    if (!initial_ping) {
        const ping_t3 = performance.now();
        const latency_ms = (ping_t3 - ping_t0) / 2.0;
        console.log("Ping with offset " + latency_ms + "ms");
        ping_responses.push(latency_ms);
        remaining_pings--;
    }

    if (remaining_pings > 0) {
        ping_t0 = performance.now();
        ping_ws.send("ping");
    }
    else {
        ping_ws.close();
        
        ping_responses.sort();
        console.log(ping_responses);
        const estimated_latency_ms = ping_responses[Math.floor(ping_responses.length / 2)];
        estimated_latency_us = estimated_latency_ms * 1000.0;

        console.log("Estimated Offset " + estimated_latency_us + "us");
        start_game();
    }
}

/////////////////////////////////////////////////////////////////////////////////////

function start_game() {
    if (game_id)
    {
        console.log("Joining game " + game_id);
        join();
    }
    else
    {
        dan_fetch('/new')
        .then(response => response.json())
        .then(x => {
            console.log("Created game ");
            console.log(x);
            game_id = x.game_id;
            join();
        });
    }
}


function join() {
    dan_fetch('/join?game_id=' + game_id + '&name=' + player_name)
        .then(response => response.json())
        .then(response => {
            console.log("/join response");
            console.log(response);
            socket_id = response.socket_id;

            console.log("Creating client");
            const seed = 0;
            const num_players = 4;
            client = new Client(seed, response.server_time_us, estimated_latency_us, num_players);

            play();
            connect_ws();
        });
}

function play() {
    dan_fetch('/play?game_id=' + game_id + '&socket_id=' + socket_id)
        .then(response => response.json())
        .then(response => {
            console.log("/play response");
            console.log(response);
            if (client)
            {
                client.join(response.player_id);
            }
        });
}

function connect_ws() {
    ws = new WebSocket(ws_endpoint + "/ws?game_id=" + game_id + '&socket_id=' + socket_id);
    ws.binaryType = "arraybuffer";
    console.log("Opening ws");

    ws.onopen = () => {
        console.log("WS Open");
    };

    ws.onmessage = evt => {
        const received_message = new Uint8Array(evt.data);
        if (client)
        {
            client.recv(received_message);
        }
    };

    ws.onclose = () => {
        console.log("WS closed");
    };
}

/////////////////////////////////////////////////////////////////////////////////////

function check(e) {
    var code = e.keyCode;
    switch (code) {
        case 37: current_input = "Left"; break;
        case 38: current_input = "Up"; break;
        case 39: current_input = "Right"; break;
        case 40: current_input = "Down"; break;
        default: break;
    }

    if (client) {
        const local_player_id = client.get_local_player_id();
        if (local_player_id > 0) {
            if (current_input == "Left") {
                player_info.set(local_player_id, "x_flip", -1);
            }
            if (current_input == "Right") {
                player_info.set(local_player_id, "x_flip", 1);
            }
        }
    }
}

window.addEventListener('keydown',check,false);

/////////////////////////////////////////////////////////////////////////////////////

let canvas = document.getElementById('canvas');
canvas.oncontextmenu = () => false;
let ctx = canvas.getContext('2d', { alpha: false });
ctx.imageSmoothingEnabled = false;

const canvasStyle = 
    "image-rendering: -moz-crisp-edges;" +
    "image-rendering: pixelated;" +
    "image-rendering: -webkit-crisp-edges;" +
    "image-rendering: crisp-edges;" +
    "bottom: 0px;" +
    "left: 0px;" +
    "width: 40%;";

canvas.style = canvasStyle;

/////////////////////////////////////////////////////////////////////////////////////

const scale = 8;

let spr_frog = new Image(8,8);
spr_frog.src = "/sprites/spr_frog.png";

let spr_frog_flipped = new Image(8,8);
spr_frog_flipped.src = "/sprites/spr_frog_flipped.png";

var snd_frog_move = new Audio('/sounds/snd_move1.wav');
snd_frog_move.volume = 0.15;

let player_info = {
    set : function(id, k, v) {
        if (!this[id]) {
            this[id] = {};
        }

        this[id][k] = v
    },
    get : function(id, k) {
        if (this[id]) {
            return this[id][k];
        }

        return undefined;
    }
};

let spr_dust = new Image(8,8);
spr_dust.src = "/sprites/spr_dust.png";
const spr_smoke_count = 4;

function create_dust(x, y) {
    return {
        frame_id : Math.floor(Math.random() * spr_smoke_count),
        scale : 0.5 + Math.random() * 0.6,
        x : x,
        y : y,
        tick : function() {
            this.scale -= 0.025;
        },

        alive: function() {
            return this.scale > 0;
        },

        draw : function() {
            const x = scale*(this.x + 0.25) + (1-this.scale);
            const y = scale*(this.y + 0.25) + (1-this.scale);
            ctx.drawImage(spr_dust, scale*this.frame_id, 0, scale, scale, x, y, scale*this.scale, scale*this.scale);
        }
    }
}

let simple_entities = [];

function tick()
{
    ctx.fillStyle = "#BAEAAA";
    ctx.fillRect(0, 0, 256, 256);

    if (client)
    {
        client.buffer_input_json('"' + current_input + '"');
        current_input = "None";

        client.tick();

        // Check if ws in ready state
        // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
        const ws_ready = ws.readyState == 1

        if (ws_ready)
        {
            const client_tick = client.get_client_message();
            ws.send(client_tick);
        }

        const rows = JSON.parse(client.get_rows_json());
        for (const row of rows) {
            let y = row.y;
            if (row.row_type.River) {
                ctx.fillStyle = "#BAEAAA";
            }
            else {
                ctx.fillStyle = "#BAE666";
            }
            //ctx.fillStyle = "#4060f0";
            ctx.fillRect(0, 256 - scale*y, 256, scale);
        }

        let simple_entities_new = [];//new Array(simple_entities.length);
        for (let entity of simple_entities) {
            entity.tick(); 
            entity.draw();
            if (entity.alive()) {
                simple_entities_new.push(entity);
            }
        }

        simple_entities = simple_entities_new;


        const players_json = client.get_players_json();
        const players = JSON.parse(players_json);

        for (const player of players) {

            if (player.pos.Coord)
            {
                const x0 = player.pos.Coord.x;
                const y0 = player.pos.Coord.y;

                // frog has duplicate frame for some reason
                //const frame_count = 6;
                const frame_count = 5;
                let frame_id = 0;
                let x,y;

                if (player.move_state != "Stationary") {
                    const moving = player.move_state.Moving;
                    // TODO don't replicate this constant
                    console.log();
                    const MOVE_T = 7 * (1000 * 1000 / 60);
                    const lerp_t = (1 - moving.remaining_us / MOVE_T);

                    let x1 = x0;
                    let y1 = y0;
                    if (moving.target.Coord) {
                        x1 = moving.target.Coord.x;
                        y1 = moving.target.Coord.y;
                    }

                    x = x0 + lerp_t * (x1 - x0);
                    y = y0 + lerp_t * (y1 - y0);
                    frame_id = Math.floor(lerp_t * (frame_count - 1));

                    if (player_info.get(player.id, "stationary")) {
                        // Make dust
                        for (let i = 0; i < 2; i++) {
                            const dust_off = Math.random() * (3 / 8);
                            const dust_dir = Math.random() * 2 * 3.141;
                            const dust_x = x + dust_off * Math.cos(dust_dir);
                            const dust_y = y + dust_off * Math.sin(dust_dir);
                            simple_entities.push(create_dust(dust_x, dust_y));
                        }

                        snd_frog_move.play();
                    }
                    player_info.set(player.id, "stationary", false);
                }
                else {
                    x = x0;
                    y = y0;

                    player_info.set(player.id, "stationary", true);
                }

                //ctx.fillStyle = "#4060f0";
                //ctx.fillRect(scale * x, scale*y, scale, scale);
                let sprite = spr_frog;
                if (player_info.get(player.id, "x_flip") == -1) {
                    sprite = spr_frog_flipped;
                }
                ctx.drawImage(sprite, scale*frame_id, 0, scale, scale, scale*x, scale*y, scale, scale);
            }
        }

        const rule_state = client.get_rule_state_json()
        document.getElementById("state").innerHTML = rule_state;
    }

    window.requestAnimationFrame(tick)
}

tick();