import { SCALE} from "./constants"
import { create_player_remote, create_player_local } from "./player";
import { create_background_controller } from "./background";
import { create_car } from "./car";
import { create_camera } from "./camera";
import { create_countdown, create_countdown_font, create_game_winner_ui } from "./game_ui";
import { create_dialogue_controller } from "./dialogue";
import { create_lillipad } from "./lillipad";
import { create_prop_controller } from "./props";
import { create_from_ai_overlay } from "./ai_overlay";
import { create_from_lilly_overlay } from "./lilly_move_hints"
import { create_font_controller } from "./font"
import { create_intro_ui, create_intro_ui_bg } from "./intro_ui"
import { create_audio_manager } from "./audio_manager"
import { create_graph } from "./graphs";


const PLAY_CROWD_SOUNDS = false;

const audio_crowd = new Audio('/sounds/snd_win.wav');
const audio_crowd_max = 0.325;
audio_crowd.addEventListener('timeupdate', function(){
    const buffer = .44
    if (this.currentTime > this.duration - buffer) {
        this.currentTime = 0;
        if (PLAY_CROWD_SOUNDS) {
            this.play();
        }
    }
});

function create_entities_container()
{
    return {
        simple_entities: [],
        bushes : [],
    }
}

export function create_game(ctx, client, ws, key_event_source) {
    let font_controller = create_font_controller();
    let audio_manager = create_audio_manager();

    let view = {
        client : client,
        ws : ws,
        ctx : ctx,
        froggy_draw_ctx : {ctx: ctx, x_off: 0, y_off: 0},
        key_event_source : key_event_source,
        current_input : "None",
        entities : create_entities_container(),
        rules_state : undefined,
        players : new Map(),
        camera : create_camera(),
        countdown : create_countdown_font(audio_manager, font_controller),
        dialogue : create_dialogue_controller(audio_manager, font_controller),
        prop_controller : create_prop_controller(),
        intro_ui : create_intro_ui(font_controller, client),
        intro_ui_bg : create_intro_ui_bg(),
        font_controller : font_controller,
        audio_manager : audio_manager,
        background_controller : create_background_controller(),

        in_lobby : true,
        in_warmup : false,

        tick : function()
        {
            if (this.client)
            {
                if (this.current_input === "0") {
                    this.client.set_ai("none");
                }
                else if (this.current_input === "1") {
                    this.client.set_ai("go_up");
                }
                else if (this.current_input === "2") {
                    this.client.set_ai("back_and_forth");
                }
                else
                {
                    this.client.buffer_input_json('"' + this.current_input + '"');
                }

                if (this.current_input != "P")
                {
                    this.client.tick();
                    this.current_input = "None";

                    // DEBUG HACK
                    //const html_elem = document.getElementById('invite_text_id');
                    //html_elem.innerHTML = (this.client.get_rules_state_json());
                }

                // Check if ws in ready state
                // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
                const ws_ready = this.ws.readyState == 1

                if (ws_ready)
                {
                    const client_tick = this.client.get_client_message();
                    this.ws.send(client_tick);

                    if (this.client.has_telemetry_messages())
                    {
                        const telemetry_message = this.client.get_telemetry_message();
                        this.ws.send(telemetry_message);
                    }

                    if (this.client.should_get_time_request())
                    {
                        const time_request = this.client.get_time_request();
                        this.ws.send(time_request);
                    }
                }

                const rules_state_json = this.client.get_rules_state_json()

                let moving_into_lobby = false;
                let moving_into_new_round = false;
                let moving_into_end = false;
                if (rules_state_json) {
                    const new_rules_state = JSON.parse(rules_state_json);
                    const new_state = new_rules_state.fst.type;
                    let old_state = "";

                    if (this.rules_state)
                    {
                        old_state = this.rules_state.fst.type;

                        if (new_state === "Lobby")
                        {
                            this.intro_ui.set_in_lobby();
                        }
                        else
                        {
                            this.intro_ui.set_in_game();
                        }

                        if (new_state !== old_state)
                        {
                            if (new_state === "Lobby") {
                                console.log("Moving into lobby state...");
                                moving_into_lobby = true;
                            }

                            if (new_state === "RoundWarmup") {
                                console.log("State is 'RoundWarmup' moving into new round via warmup...");
                                moving_into_new_round = true;
                            }

                            if (new_state === "EndWinner") {
                                console.log("State is 'EndWinner' movnig into end state...");
                                moving_into_end = true;
                            }

                            if (new_state === "EndAllLeft") {
                                console.log("State is 'EndAllLeft' movnig into end state...");
                                moving_into_end = true;
                            }

                            if (new_state === "Round" && old_state !== "RoundWarmup") {
                                console.log("State is 'Round' moving into new round skipping warmup...");
                                moving_into_new_round = true;
                            }
                        }
                    }

                    this.rules_state = new_rules_state;
                    this.in_lobby = new_state === "Lobby" || new_state === "EndWinner" || new_state === "EndAllLeft";
                    this.in_warmup = new_state === "RoundWarmup";
                }

                const players_json = this.client.get_players_json();
                const current_player_states = JSON.parse(players_json);

                if (moving_into_lobby) {
                    this.entities = create_entities_container();
                    this.background_controller.reset();
                }
                else if (moving_into_new_round) {
                    if (PLAY_CROWD_SOUNDS) {
                        audio_crowd.play();
                    }

                    this.entities = create_entities_container();

                    for (const [_, player] of this.players) {
                        if (player) {
                            player.new_round(this.rules_state.fst, this.entities.simple_entities);
                        }
                    }

                    this.background_controller.reset();
                }
                else if (moving_into_end)
                {
                    this.entities = create_entities_container();
                    let winner_name = "";
                    if (this.rules_state && this.rules_state.fst.type === "EndWinner") {
                        winner_name = this.players.get(this.rules_state.fst.winner_id).name;
                    }
                    else
                    {
                        // One player left, get their name
                        const local_player_id = this.client.get_local_player_id();
                        if (local_player_id >= 0)
                        {
                            winner_name = this.players.get(this.client.get_local_player_id()).name;
                        }
                    }

                    this.entities.simple_entities.push(create_game_winner_ui(this.font_controller, winner_name));
                    this.background_controller.reset();
                }

                this.background_controller.tick(this.in_lobby, this.in_warmup, this.entities, this.client);

                let players_with_values = new Set();

                const local_player_id = this.client.get_local_player_id();
                if (local_player_id >= 0) {
                    for (const current_player_state of current_player_states) {
                        if (!this.players.get(current_player_state.id)) {
                            console.log("creating player");
                            if (current_player_state.id === local_player_id) {
                                console.log("creating local player");
                                // Create local player
                                this.players.set(current_player_state.id, create_player_local(this.client, this.key_event_source, this.audio_manager));
                            }
                            else {
                                // Create remote player
                                this.players.set(current_player_state.id, create_player_remote(this.client, current_player_state.id, this.audio_manager));
                            }
                        }

                        let player = this.players.get(current_player_state.id);
                        player.tick(current_player_state, this.entities, this.rules_state);

                        players_with_values.add(current_player_state.id);
                    }

                    let filtered_players = new Map();
                    for (const [player_id, player] of this.players) {
                        if (players_with_values.has(player_id)) {
                            filtered_players.set(player_id, player);
                        }
                    }

                    this.players = filtered_players;
                }

                audio_crowd.volume = audio_crowd_max / (1 - 0.25 * this.camera.y);

                let simple_entities_still_alive = [];

                // TODO Fix the issue on respawn when camera is moving down
                // before pruning by camera_y_max
                const CAMERA_Y_MAX_BUGFIX = 160;
                let camera_y_max = CAMERA_Y_MAX_BUGFIX;
                if (this.rules_state && this.rules_state.fst.type === "Round") {
                    // Do proper pruning if we are in a round where camera cant go down
                    camera_y_max = (this.camera.y + 20) * SCALE;
                }

                for (let entity of this.entities.simple_entities) {
                    entity.tick(); 

                    if (entity.alive(camera_y_max)) {
                        simple_entities_still_alive.push(entity);
                    }
                }

                this.entities.simple_entities = simple_entities_still_alive;

                this.camera.tick(this.rules_state);
                this.froggy_draw_ctx.x_off = Math.round(-SCALE * this.camera.x);
                this.froggy_draw_ctx.y_off = Math.round(-SCALE * this.camera.y);

                this.countdown.tick(this.rules_state);
                this.dialogue.tick(this.rules_state, this.players, this.entities.simple_entities);
                this.intro_ui.tick(this.players);
                this.intro_ui_bg.tick(this.rules_state);

                this.prop_controller.tick(this.rules_state, this.entities.simple_entities, this.client);
                this.font_controller.tick();
            }
        },

        draw : function() {
            this.background_controller.draw(this.froggy_draw_ctx, this.client)
            this.intro_ui_bg.draw(this.froggy_draw_ctx);

            if (this.client) {
                let draw_with_depth = [];

                for (let entity of this.entities.simple_entities) {
                    draw_with_depth.push(entity);
                }

                if (!this.in_lobby)
                {
                    const cars = JSON.parse(this.client.get_cars_json());
                    for (const car of cars) {
                        draw_with_depth.push(create_car(car));
                    }

                    const lillipads = JSON.parse(this.client.get_lillipads_json());
                    for (const lillipad of lillipads) {
                        draw_with_depth.push(create_lillipad(lillipad));
                    }

                    const ai_overlay_json = this.client.get_ai_drawstate_json();
                    if (ai_overlay_json && ai_overlay_json.length > 0)
                    {
                        const ai_overlay = JSON.parse(ai_overlay_json);
                        const created = create_from_ai_overlay(ai_overlay);
                        for (const c of created) {
                            draw_with_depth.push(c);
                        }
                    }
                    const lilly_drawstate_json = this.client.get_lilly_drawstate_json();
                    if (lilly_drawstate_json && lilly_drawstate_json.length > 0)
                    {
                        const lilly_drawstate = JSON.parse(lilly_drawstate_json);
                        const created = create_from_lilly_overlay(lilly_drawstate);
                        for (const c of created) {
                            draw_with_depth.push(c);
                        }
                    }
                }

                for (const [_, player] of this.players) {
                    if (player) {
                        draw_with_depth.push(player);
                    }
                }

                const server_offset_graph_json = this.client.get_server_time_offset_graph_json();
                if (server_offset_graph_json && server_offset_graph_json.length > 0) {
                    const server_offset_graph = JSON.parse(server_offset_graph_json);
                    draw_with_depth.push(create_graph(10, 16, server_offset_graph));
                }

                const server_message_count_graph_json = this.client.get_server_message_count_graph_json();
                if (server_message_count_graph_json && server_message_count_graph_json.length > 0) {
                    const server_message_count_graph = JSON.parse(server_message_count_graph_json);
                    draw_with_depth.push(create_graph(10, 60, server_message_count_graph));
                }

                draw_with_depth.sort(sort_depth);

                for (const drawable of draw_with_depth) {
                    drawable.draw(this.froggy_draw_ctx);
                }

                this.dialogue.draw(this.froggy_draw_ctx);
                this.countdown.draw(this.froggy_draw_ctx);
                this.intro_ui.draw(this.froggy_draw_ctx);

                // TMP
                //let frame_id = this.client.get_top_frame_id();
                //this.font_controller.text(this.froggy_draw_ctx, frame_id.toString(), 10, 10);
                //this.froggy_draw_ctx.ctx.fillStyle = "black";
                //this.froggy_draw_ctx.ctx.fillText(frame_id.toString(), 10, 10);

                /*
                const local_player_id = this.client.get_local_player_id();
                if (local_player_id >= 0)
                {
                    let local_player = this.players.get(local_player_id);
                    if (local_player)
                    {
                        this.froggy_draw_ctx.ctx.fillStyle = "black";
                        this.froggy_draw_ctx.ctx.fillText(`(${Math.round(local_player.source.x)}, ${Math.round(local_player.source.y)})`, 10, 10);
                    }
                }
                */

                this.froggy_draw_ctx.ctx.fillStyle = "black";
                let est_time_seconds = this.client.estimate_time_from_frame_id();
                this.froggy_draw_ctx.ctx.fillText(`${Math.round(10 * est_time_seconds) / 10}`, 10, 10);
            }
        }
    }

    let listener = key_event_source.add_input_listener();
    listener.on_input_keydown = function(input) {
        view.current_input = input;
        view.audio_manager.webpage_has_inputs = true;
    }

    return view;
}

function sort_depth(a, b) {
    // Would be so nice in rust :(
    // We order by: foreground_depth then dynamic_depth then depth
    if (a.foreground_depth !== undefined && b.foreground_depth !== undefined) {
        return a.foreground_depth - b.foreground_depth
    }
    else if (a.foreground_depth !== undefined) {
        return 1;
    }
    else if (b.foreground_depth !== undefined) {
        return -1;
    }
    else if (a.dynamic_depth !== undefined && b.dynamic_depth !== undefined) {
        return a.dynamic_depth - b.dynamic_depth;
    }
    else if (a.dynamic_depth !== undefined) {
        return 1;
    }
    else if (b.dynamic_depth !== undefined) {
        return -1;
    } 
    else {
        return a.depth - b.depth;
    }
}