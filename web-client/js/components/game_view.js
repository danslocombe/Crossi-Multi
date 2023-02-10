import { SCALE} from "./constants"
import { create_player_remote, create_player_local } from "./player";
import { draw_background } from "./background";
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


const audio_crowd = new Audio('/sounds/snd_win.wav');
const audio_crowd_max = 0.325;
audio_crowd.addEventListener('timeupdate', function(){
    const buffer = .44
    if (this.currentTime > this.duration - buffer) {
        this.currentTime = 0;
        this.play();
    }
});

export function create_game_view(ctx, client, ws, key_event_source) {
    let font_controller = create_font_controller();
    let audio_manager = create_audio_manager();

    let view = {
        client : client,
        ws : ws,
        ctx : ctx,
        froggy_draw_ctx : {ctx: ctx, x_off: 0, y_off: 0},
        key_event_source : key_event_source,
        current_input : "None",
        simple_entities : [],
        rule_state : undefined,
        players : new Map(),
        camera : create_camera(),
        countdown : create_countdown_font(audio_manager, font_controller),
        dialogue : create_dialogue_controller(audio_manager, font_controller),
        prop_controller : create_prop_controller(),
        intro_ui : create_intro_ui(font_controller, client),
        intro_ui_bg : create_intro_ui_bg(),
        font_controller : font_controller,
        audio_manager : audio_manager,

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
                    //html_elem.innerHTML = (this.client.get_rule_state_json());
                }

                // Check if ws in ready state
                // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
                const ws_ready = this.ws.readyState == 1

                if (ws_ready)
                {
                    const client_tick = this.client.get_client_message();
                    this.ws.send(client_tick);

                    if (this.client.should_get_time_request())
                    {
                        const time_request = this.client.get_time_request();
                        this.ws.send(time_request);
                    }
                }

                const rule_state_json = this.client.get_rule_state_json()
                let moving_into_lobby = false;
                let moving_into_warmup = false;
                let moving_into_end = false;
                if (rule_state_json) {
                    const rule_state = JSON.parse(rule_state_json);

                    if (this.rule_state && rule_state.Lobby)
                    {
                        this.intro_ui.set_in_lobby();
                    }
                    else
                    {
                        this.intro_ui.set_in_game();
                    }

                    if (this.rule_state && rule_state.Lobby && !this.rule_state.Lobby) {
                        console.log("Moving into lobby state...");
                        moving_into_lobby = true;
                    }

                    if (this.rule_state && rule_state.RoundWarmup && !this.rule_state.RoundWarmup) {
                        console.log("State is 'RoundWarmup' movnig into warmup state...");
                        moving_into_warmup = true;
                    }

                    if (this.rule_state && rule_state.EndWinner && !this.rule_state.EndWinner) {
                        console.log("State is 'EndWinner' movnig into end state...");
                        moving_into_end = true;
                    }

                    if (this.rule_state && rule_state.EndAllLeft && !this.rule_state.EndAllLeft) {
                        console.log("State is 'EndAllLeft' movnig into end state...");
                        moving_into_end = true;
                    }

                    this.rule_state = rule_state;
                }

                const players_json = this.client.get_players_json();
                const current_player_states = JSON.parse(players_json);

                if (moving_into_lobby) {
                    this.simple_entities = [];
                }
                else if (moving_into_warmup) {
                    audio_crowd.play();
                    this.simple_entities = [];
                    for (const [_, player] of this.players) {
                        if (player) {
                            player.new_round(this.rule_state.RoundWarmup, this.simple_entities);
                        }
                    }
                }
                else if (moving_into_end)
                {
                    this.simple_entities = [];
                    let winner_name = "";
                    if (this.rule_state && this.rule_state.EndWinner) {
                        winner_name = this.players.get(this.rule_state.EndWinner.winner_id).name;
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

                    this.simple_entities.push(create_game_winner_ui(this.font_controller, winner_name));
                }

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
                        player.tick(current_player_state, this.simple_entities, this.rule_state);

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

                let simple_entities_new = [];

                // TODO Fix the issue on respawn when camera is moving down
                // before pruning by camera_y_max
                const CAMERA_Y_MAX_BUGFIX = 160;
                let camera_y_max = CAMERA_Y_MAX_BUGFIX;
                if (this.rule_state && this.rule_state.Round) {
                    // Do proper pruning if we are in a round where camera cant go down
                    camera_y_max = (this.camera.y + 20) * SCALE;
                }

                for (let entity of this.simple_entities) {
                    entity.tick(); 

                    if (entity.alive(camera_y_max)) {
                        simple_entities_new.push(entity);
                    }
                }

                this.simple_entities = simple_entities_new;

                this.camera.tick(this.rule_state);
                this.froggy_draw_ctx.x_off = Math.round(-SCALE * this.camera.x);
                this.froggy_draw_ctx.y_off = Math.round(-SCALE * this.camera.y);

                this.countdown.tick(this.rule_state);
                this.dialogue.tick(this.rule_state, this.players, this.simple_entities);
                this.intro_ui.tick(this.players);
                this.intro_ui_bg.tick(this.rule_state);

                this.prop_controller.tick(this.rule_state, this.simple_entities, this.client);
                this.font_controller.tick();
            }
        },

        draw : function() {
            const in_lobby = !this.rule_state || this.rule_state.Lobby || this.rule_state.EndWinner || this.rule_state.EndAllLeft;
            const in_warmup = this.rule_state && this.rule_state.RoundWarmup;
            draw_background(this.froggy_draw_ctx, in_lobby, in_warmup, this.client)
            this.intro_ui_bg.draw(this.froggy_draw_ctx);

            if (this.client) {
                let draw_with_depth = [];

                for (let entity of this.simple_entities) {
                    draw_with_depth.push(entity);
                }

                if (!in_lobby)
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