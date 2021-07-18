//const { Client } = require("../pkg/index.js");
import { Client } from "../pkg/index.js"

var game_id = 1;
var player_name = "Dan";

var client = undefined; // new Client(100, 0, 10*1000, [], 4);

fetch('/new')
.then(response => {console.log(response); return response})
.then(response => response.json())
.then(id => {
        console.log("Created game " + id);
	game_id = id;
	join();
});

function join() {
    fetch('/join?game_id=' + game_id + '&name=' + player_name)
        .then(response => response.json())
        .then(response => {
            //init = true;
            console.log("Game ID : " + game_id);
            console.log(response);
            //printWords();
            connect_ws();

            client = new Client(100, 0, 10*100, [], 4);
        });
}

function connect_ws() {
    const player_id = 1;
    const ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id + '&player_id=' + player_id);
    ws.binaryType = "arraybuffer";
    //var ws = new WebSocket("ws://localhost:8080/ws?game_id=" + game_id);
    console.log("Opening ws");

    ws.onopen = () => {
        // Web Socket is connected, send data using send()
        ws.send("Message to send");
        console.log("Message is sent...");
    };

    ws.onmessage = evt => {
        const received_message = new Uint8Array(evt.data);
        if (client)
        {
		console.log(received_message);
            client.recv(received_message);
        }
    };

    ws.onclose = () => {
        // websocket is closed.
        console.log("Connection is closed..."); 
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
        const players = client.get_players_json();
        console.log(players);
    }

    window.requestAnimationFrame(tick)
}

tick();