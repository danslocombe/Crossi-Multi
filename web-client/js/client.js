"use strict";

const DEBUG = true;
//const { Client } = require("../pkg/index.js");
import { Client } from "../pkg/index.js"

const query_string = window.location.search;
const url_params = new URLSearchParams(query_string);
var game_id = url_params.get('game_id');

var player_name = "Dan";
var socket_id = 0;

var client = undefined;
var ws = undefined;

var endpoint = "";
if (DEBUG)
{
    // Fetch from specific localhost / port in order to allow better debugging
    // (we host debug build from localhost:8081)
    // NOTE HAVE TO RUN CHROME WITH NO CORS
    endpoint = 'http://localhost:8080'
}

function dan_fetch(url) {
    return fetch(endpoint + url, {
        headers: {  'Accept': 'application/json' },
        //mode : "no-cors"
    });
}

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

function join() {
    dan_fetch('/join?game_id=' + game_id + '&name=' + player_name)
        .then(response => response.json())
        .then(response => {
            //init = true;
            console.log("/join response");
            console.log(response);
            socket_id = response.socket_id;

            //printWords();

            console.log("Creating client");
            const estimated_latency = 25 * 1000;
            const seed = 0;
            const num_players = 4;
            client = new Client(seed, response.server_time_us, estimated_latency, num_players);

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
            // No op
            if (client)
            {
                client.join(response.player_id);
            }
        });
}

function connect_ws() {
    const player_id = 1;
    ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id + '&socket_id=' + socket_id);
    ws.binaryType = "arraybuffer";
    //var ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id);
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
        // websocket is closed.
        console.log("WS closed");
    };
}

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

function tick()
{
    ctx.fillStyle = "#BAEAAA";
    ctx.fillRect(0, 0, 256, 256);


    if (client)
    {
        client.set_local_input_json('"' + current_input + '"');
        current_input = "None";

        client.tick();

        // If ws in ready state
        // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
        if (ws.readyState == 1)
        {
            const client_tick = client.get_client_message();
            ws.send(client_tick);
        }

        const players_json = client.get_players_json();
        //console.log(players);

        const players = JSON.parse(players_json);

        for (const player of players) {
            const scale = 8;

            if (player.pos.Coord)
            {
                let x = player.pos.Coord.x;
                let y = player.pos.Coord.y;
                //if (x && y)
                {
                    ctx.fillStyle = "#4060f0";
                    ctx.fillRect(scale * x, scale*y, scale, scale);
                }
            }
        }

    }

    window.requestAnimationFrame(tick)
}

tick();

var current_input = "None";

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