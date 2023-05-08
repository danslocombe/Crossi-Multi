import { SCALE } from "./constants";
import { dan_lerp } from "./utils";

export function create_intro_ui(font_controller, client) {
    return {
        in_lobby : false,
        font_controller : font_controller,
        client : client,
        draw_upper : false,
        text_y : 0,

        local_player_pos_set : false,
        local_player_x_0 : 0,
        local_player_y_0 : 0,
        local_player_has_moved : false,
        local_player_has_readied : false,

        multiple_players : false,

        set_in_lobby : function() {
            this.in_lobby = true;
        },

        set_in_game : function() {
            this.in_lobby = false;
        },

        tick : function(players) {
            if (this.in_lobby)
            {
                // SeizureDome style text movement based on local player position.
                const local_player_id = client.get_local_player_id();
                if (local_player_id >= 0)
                {
                    let local_player = players.get(local_player_id);
                    if (local_player)
                    {
                        if (!this.local_player_pos_set)
                        {
                            this.local_player_pos_set = true;
                            this.local_player_x_0 = local_player.x;
                            this.local_player_y_0 = local_player.y;
                        }
                        else if (!this.local_player_has_moved)
                        {
                            if (local_player.x != this.local_player_x_0 && local_player.y != this.local_player_y_0)
                            {
                                this.local_player_has_moved = true;
                            }
                        }

                        this.multiple_players = players.size > 1;
                        this.local_player_has_readied |= local_player.lobby_ready;

                        let y = local_player.y;
                        if (y > 160 * 2/3)
                        {
                            this.draw_upper = true;
                        }
                        if (y < 160 * 1/3)
                        {
                            this.draw_upper = false;
                        }
                    }
                }

                // DAN @TMP figure out how to layer intro text with new background
                // for now just keep at the top
                this.draw_upper = true;

                let target_y = 160 - 30;
                if (this.draw_upper)
                {
                    target_y = 30;
                }

                this.text_y = dan_lerp(this.text_y, target_y, 15);
            }
        },

        draw : function(froggy_draw_ctx) {
            const xoff = 24; 
            
            // TODO @dan
            // Disabling for now
            return;

            /*
            froggy_draw_ctx.ctx.fillStyle = "#FFFFFF";
            froggy_draw_ctx.ctx.fillRect(0, 0, 256, 256);

                this.font_controller.text(froggy_draw_ctx, "heavy rain", xoff + 8, this.text_y - this.font_controller.font.height / 2 - 8);
                this.font_controller.text(froggy_draw_ctx, "eleven degrees", xoff - 12, this.text_y - this.font_controller.font.height / 2 + 8);
                */
            
            if (this.in_lobby)
            {
                //return;

                if (!this.local_player_has_moved)
                {
                    this.font_controller.set_Font_small();
                    this.font_controller.text(froggy_draw_ctx, "move with", xoff + 8, this.text_y - this.font_controller.font.height / 2 - 8);
                    this.font_controller.text(froggy_draw_ctx, "the arrow keys", xoff - 12, this.text_y - this.font_controller.font.height / 2 + 8);
                }
                else
                {
                    if (!this.multiple_players)
                    {
                        this.font_controller.set_Font_small();
                        this.font_controller.text(froggy_draw_ctx, "invite friends", xoff - 16, this.text_y - this.font_controller.font.height / 2);
                    }
                    /*
                    else if (!this.local_player_has_readied)
                    {
                        this.font_controller.text(froggy_draw_ctx, "press space", xoff, this.text_y - this.font_controller.font.height / 2 - 8);
                        this.font_controller.text(froggy_draw_ctx, "to ready up", xoff, this.text_y - this.font_controller.font.height / 2 + 8);
                    }
                    */
                }
            }
        },
    }
}

export function create_intro_ui_bg() {
    return {
        visible : false,
        proportion : 0,
        tick : function(rules_state) {
            this.visible = false;
            this.proportion = 0;
            if (rules_state && rules_state.fst.type === "Lobby")
            {
                this.visible = true;
                this.proportion = rules_state.fst.time_with_all_players_in_ready_zone / 120;
            }
        },
        draw : function(froggy_draw_ctx) {
            if (this.visible) {
                const x0 = 7 * SCALE;
                const y0 = 14 * SCALE;
                const width_base = 6 * SCALE;
                const height = 4 * SCALE;

                froggy_draw_ctx.ctx.fillStyle = "white";
                froggy_draw_ctx.ctx.fillRect(x0, y0, width_base * this.proportion, height);

                froggy_draw_ctx.ctx.strokeStyle = "black";
                froggy_draw_ctx.ctx.strokeRect(x0, y0, width_base, height);
            }
        },
    }
}