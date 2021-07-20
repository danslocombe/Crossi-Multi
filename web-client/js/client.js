const DEBUG = true;
//const { Client } = require("../pkg/index.js");
import { Client } from "../pkg/index.js"

var game_id = 1;
var player_name = "Dan";
var socket_id = 0;

var client = undefined;

var endpoint = "";
if (DEBUG)
{
    endpoint = 'http://localhost:8080'
}

function dan_fetch(url) {
    return fetch(endpoint + url, {
        headers: {  'Accept': 'application/json' },
        //mode : "no-cors"
    });
}

// Fetch from specific localhost / port in order to allow better debugging
// (we host debug build from localhost:8081)
// NOTE HAVE TO RUN CHROME WITH NO CORS
dan_fetch('/new')
.then(response => response.json())
.then(x => {
    console.log("Created game ");
    console.log(x);
	game_id = x.game_id;
	join();
});

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
    const ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id + '&socket_id=' + socket_id);
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
    ctx.fillStyle = "#4060f0";
    ctx.fillRect(8, 8, 8, 8);


    if (client)
    {
        client.tick();
        const players = client.get_players_json();
        //console.log(players);
    }

    window.requestAnimationFrame(tick)
}

tick();