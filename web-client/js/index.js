"use strict";

import { create_game }  from "./components/game.js"
import { Client } from "../pkg/index.js"
import ClipboardJS from 'clipboard';

const DEBUG = true;
const LOCAL_DEBUG = true;
const DEBUG_PLAY_LINK = true;

const query_string = window.location.search;
const url_params = new URLSearchParams(query_string);
var game_id = url_params.get('game_id');
var debug_bypass_lobby = url_params.get('debug_bypass_lobby');

var player_name = "Dan";
var socket_id = 0;

var client = undefined;
var ws = undefined;
var estimated_latency_us = 0;

var endpoint = "";
var ws_endpoint = "";

if (LOCAL_DEBUG)
{
    // Fetch from specific localhost / port in order to allow better debugging
    // (we host debug build from localhost:8081)
    // NOTE HAVE TO RUN CHROME WITH NO CORS
    endpoint = 'http://localhost:8080';
    ws_endpoint = 'ws://localhost:8080';
}
else if (DEBUG) {
    endpoint = 'http://51.6.233.191:8080';
    ws_endpoint = 'ws://51.6.233.191:8080';
}
else {
    endpoint = 'https://roadtoads.io';
    ws_endpoint = 'wss://roadtoads.io';
}

export function fetch_json(url) {
    return fetch(endpoint + url, {
        headers: {  'Accept': 'application/json' },
    });
}

function create_gameid_url()
{
    let x_endpoint = "";

    if (!DEBUG)
    {
        x_endpoint = endpoint;
    }

    return x_endpoint + '/?game_id=' + game_id;
}

function write_join_link(game_id) {
    const gameid_url = create_gameid_url();
    if (DEBUG && DEBUG_PLAY_LINK)
    {
        document.getElementById("debug_join").innerHTML = '<a href="' + gameid_url + '"> GameId: ' + game_id + '</a>';
        document.getElementById("joinlink").value = gameid_url;
    }
    else
    {
        document.getElementById("joinlink").value = gameid_url;
    }
}

function start_game() {
    if (game_id)
    {
        console.log("Joining game " + game_id);
        write_join_link(game_id);
        join();
    }
    else
    {
        //fetch_json('/new')
        var url = '/new'
        if (debug_bypass_lobby) {
            url += '?debug_bypass_lobby=true';
        }
        fetch_json(url)
        .then(response => response.json())
        .then(x => {
            console.log("Created game ");
            console.log(x);
            game_id = x.game_id;
            window.history.replaceState(null, '', create_gameid_url());
            write_join_link(game_id);
            join();
        });
    }
}
start_game();

function join() {

    console.log("Calling join...");

    fetch_json('/join?game_id=' + game_id + '&name=' + player_name)
        .then(response => response.json())
        .then(response => {
            console.log("/join response");
            console.log(response);
            socket_id = response.socket_id;

            console.log("Creating client");
            console.log("JS server_ms=" + response.server_time_us / 1000 + " estimated_latency=" + estimated_latency_us / 1000);
            client = new Client(game_id, response.server_frame_id, response.server_time_us, estimated_latency_us);

            play();
            connect_ws();
        });
}

function play() {
    fetch_json('/play?game_id=' + game_id + '&socket_id=' + socket_id)
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
        setup_game();
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

let key_event_source = {
    input_listeners : [],
    activate_listeners : [],
    on_keydown : function(e) {
        let code = e.keyCode;

        if (code == 32) {
            // Space
            for (const listener of this.activate_listeners) {
                listener.on_activate_keydown();
            }

            return;
        }

        let input = "None";
        switch (code) {
            case 37: input = "Left"; break;
            case 38: input = "Up"; break;
            case 39: input = "Right"; break;
            case 40: input = "Down"; break;
            case 48: input = "0"; break;
            case 49: input = "1"; break;
            case 50: input = "2"; break;
            case 51: input = "3"; break;
            case 70: input = "F"; break;
            case 71: input = "G"; break;
            case 80: input = "P"; break;
            case 223: input = "Kill"; break;
            default: break;
        }

        for (const listener of this.input_listeners) {
            listener.on_input_keydown(input);
        }
    },
    add_input_listener : function() {
        let new_listener = {};
        this.input_listeners.push(new_listener);
        return new_listener;
    },
    add_activate_listener : function() {
        let new_listener = {};
        this.activate_listeners.push(new_listener);
        return new_listener;
    }
}

window.addEventListener('keydown', (e) => key_event_source.on_keydown(e), false);

/////////////////////////////////////////////////////////////////////////////////////

let canvas = document.getElementById('canvas');
canvas.oncontextmenu = () => false;
let ctx = canvas.getContext('2d', { alpha: false });
ctx.imageSmoothingEnabled = false;

const canvasStyle = 
    "image-rendering: -moz-crisp-edges;" +
    "image-rendering: pixelated;" +
    "image-rendering: -webkit-crisp-edges;" +
    "image-rendering: crisp-edges;";

canvas.style = canvasStyle;

/////////////////////////////////////////////////////////////////////////////////////

let prev_frame_time_ms = window.performance.now();
const desired_fps = 60;
const ms_per_frame = 1000 / desired_fps;

function setup_game() {
    let game = create_game(ctx, client, ws, key_event_source);

    let tick = () => {
        window.requestAnimationFrame(tick);

        const now_ms = window.performance.now();
        const dt_ms = now_ms - prev_frame_time_ms;

        if (dt_ms < ms_per_frame) {
            return;
        }

        const excess_ms = dt_ms % ms_per_frame;
        prev_frame_time_ms = now_ms - excess_ms;
        
        game.tick();
        game.draw();
    }

    tick();
}

/////////////////////////////////////////////////////////////////////////////////////

const clipboard = new ClipboardJS('.clipboard_btn');

const join_link_copy = document.getElementById('join_link_copy');
join_link_copy.addEventListener('click', () => {
    join_link_copy.innerHTML = "Copied!";
    setTimeout(() => {
        join_link_copy.innerHTML = "Copy link";
    }, 1200);
})