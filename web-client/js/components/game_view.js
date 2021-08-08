import { SCALE} from "./constants"
import { create_player_remote, create_player_local } from "./player";
import { draw_background } from "./background";
import { create_car } from "./car";
import { create_camera } from "./camera";
import { create_countdown } from "./countdown";
import { create_dialogue_controller } from "./dialogue";
import { create_lillipad } from "./lillipad";
import { create_prop_controller } from "./props";

export function create_game_view(ctx, client, ws, key_event_source) {
    let view = {
        client : client,
        ws : ws,
        ctx : ctx,
        froggy_draw_ctx : {ctx: ctx, x_off: 0, y_off: 0},
        key_event_source : key_event_source,
        current_input : "None",
        simple_entities : [],
        rule_state : undefined,
        players : [],
        camera : create_camera(),
        countdown : create_countdown(),
        dialogue : create_dialogue_controller(),
        prop_controller : create_prop_controller(),

        tick : function()
        {
            if (this.client)
            {
                this.client.buffer_input_json('"' + this.current_input + '"');
                this.current_input = "None";

                this.client.tick();

                // Check if ws in ready state
                // https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState
                const ws_ready = this.ws.readyState == 1

                if (ws_ready)
                {
                    const client_tick = this.client.get_client_message();
                    this.ws.send(client_tick);
                }

                const rule_state_json = this.client.get_rule_state_json()
                let moving_into_warmup = false;
                if (rule_state_json) {
                    const rule_state = JSON.parse(rule_state_json);

                    if (this.rule_state && rule_state.RoundWarmup && !this.rule_state.RoundWarmup) {
                        moving_into_warmup = true;
                    }

                    this.rule_state = rule_state;
                }

                document.getElementById("state").innerHTML = rule_state_json;

                const players_json = this.client.get_players_json();
                const current_player_states = JSON.parse(players_json);

                if (moving_into_warmup) {
                    this.simple_entities = [];
                    for (const player of this.players) {
                        if (player) {
                            player.new_round();
                        }
                    }
                }

                const local_player_id = this.client.get_local_player_id();
                if (local_player_id >= 0) {
                    for (const current_player_state of current_player_states) {
                        if (!this.players[current_player_state.id]) {
                            console.log("creating player");
                            if (current_player_state.id === local_player_id) {
                                console.log("creating local player");
                                // Create local player
                                this.players[current_player_state.id] = create_player_local(this.client, this.key_event_source);
                            }
                            else {
                                // Create remote player
                                this.players[current_player_state.id] = create_player_remote(this.client, current_player_state.id);
                            }
                        }

                        let player = this.players[current_player_state.id];
                        player.tick(current_player_state, this.simple_entities, this.rule_state);
                    }
                }

                let simple_entities_new = [];
                const camera_y_max = (this.camera.y + 20) * SCALE;
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

                this.prop_controller.tick(this.rule_state, this.simple_entities, this.client);
            }
        },

        draw : function() {
            const in_lobby = !this.rule_state || this.rule_state.Lobby;
            const in_warmup = this.rule_state && this.rule_state.RoundWarmup;
            draw_background(this.froggy_draw_ctx, in_lobby, in_warmup, this.client)

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
                }

                for (const player of this.players) {
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
            }
        }
    }

    let listener = key_event_source.add_input_listener();
    listener.on_input_keydown = function(input) {
        view.current_input = input;
    }

    let activate_listener = key_event_source.add_activate_listener();
    activate_listener.on_activate_keydown = function() {
        view.client.set_ready_state(!view.client.get_ready_state());
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
        return b.depth - a.depth;
    }
}