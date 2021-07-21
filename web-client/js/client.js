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
    "width: 60%;";

canvas.style = canvasStyle;

/////////////////////////////////////////////////////////////////////////////////////

function tick()
{
    ctx.fillStyle = "#BAEAAA";
    ctx.fillRect(0, 0, 256, 256);

    if (client)
    {
        client.set_local_input_json('"' + current_input + '"');
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

        const players_json = client.get_players_json();
        const players = JSON.parse(players_json);

        for (const player of players) {
            const scale = 8;

            if (player.pos.Coord)
            {
                let x = player.pos.Coord.x;
                let y = player.pos.Coord.y;
                ctx.fillStyle = "#4060f0";
                ctx.fillRect(scale * x, scale*y, scale, scale);
            }
        }
    }

    window.requestAnimationFrame(tick)
}

tick();