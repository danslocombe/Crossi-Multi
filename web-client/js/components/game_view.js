import { create_player_local } from "./player_def";
//import "/components/player_def";

const SCALE = 8;

let players = {};

export function create_game_view(ctx, client, ws, key_event_source) {
    let view = {
        client : client,
        ws : ws,
        ctx : ctx,
        key_event_source : key_event_source,
        current_input : "None",
        simple_entities : [],

        tick : function()
        {
            this.ctx.fillStyle = "#BAEAAA";
            this.ctx.fillRect(0, 0, 256, 256);

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

                const rows = JSON.parse(this.client.get_rows_json());
                for (const row of rows) {
                    let y = row.y;
                    if (row.row_type.River) {
                        this.ctx.fillStyle = "#BAEAAA";
                    }
                    else {
                        this.ctx.fillStyle = "#BAE666";
                    }
                    //ctx.fillStyle = "#4060f0";
                    this.ctx.fillRect(0, 256 - SCALE*y, 256, SCALE);
                }

                let simple_entities_new = [];//new Array(simple_entities.length);
                for (let entity of this.simple_entities) {
                    entity.tick(); 
                    entity.draw(this.ctx);
                    if (entity.alive()) {
                        simple_entities_new.push(entity);
                    }
                }

                this.simple_entities = simple_entities_new;

                const players_json = this.client.get_players_json();
                const current_player_states = JSON.parse(players_json);

                // TODO fixme
                let local_player_id = this.client.get_local_player_id();
                if (local_player_id >= 0) {
                    for (const current_player_state of current_player_states) {
                        if (!players[current_player_state.id]) {
                            console.log("creating player");
                            if (current_player_state.id === local_player_id) {
                                console.log("creating local player");
                                // Create local player
                                players[local_player_id] = create_player_local(this.client, this.key_event_source);
                            }
                            else {
                                // Create remote player
                            }
                        }

                        let player = players[current_player_state.id];
                        player.tick(current_player_state, this.simple_entities);
                        player.draw(this.ctx);
                    }
                }

                const rule_state = this.client.get_rule_state_json()
                document.getElementById("state").innerHTML = rule_state;
            }
        }

    }

    let listener = key_event_source.add_listener();
    listener.on_keydown = function(input) {
        view.current_input = input;
    }

    return view;
}