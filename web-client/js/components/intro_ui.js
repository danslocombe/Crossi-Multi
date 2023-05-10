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

        t : 0,

        visible : false,

        tick : function(rules_state, players) {
            this.visible = false;

            let local_player_exists = false;

            // @FIXME move this logic somewhere or inject local_player instance.
            const local_player_id = this.client.get_local_player_id();
            let local_player = null; 
            if (local_player_id >= 0)
            {
                local_player = players.get(local_player_id);
            }

            const in_lobby = rules_state && rules_state.fst.type === "Lobby";

            if (in_lobby && local_player != null)
            {
                // Hacky, add a lil delay before showing to allow the dialogue to dissipate
                this.t += 1;
                if (this.t < 80)
                {
                    return;
                }

                this.visible = true;

                // SeizureDome style text movement based on local player position.
                if (!this.local_player_pos_set)
                {
                    this.local_player_pos_set = true;
                    this.local_player_x_0 = local_player.x;
                    this.local_player_y_0 = local_player.y;
                }
                else if (!this.local_player_has_moved && this.t > 160)
                {
                    if (local_player.x != this.local_player_x_0 && local_player.y != this.local_player_y_0)
                    {
                        this.local_player_has_moved = true;
                    }
                }

                this.multiple_players = players.size > 1;
                this.local_player_has_readied |= local_player.lobby_ready;

                let y = local_player.y;
                if (y > 160 * 3/10)
                {
                    this.draw_upper = true;
                }
                if (y < 160 * 2.3/10)
                {
                    this.draw_upper = false;
                }

                let target_y = 160 - 80;
                if (this.draw_upper)
                {
                    target_y = 20;
                }

                this.text_y = dan_lerp(this.text_y, target_y, 15);
            }
        },

        draw : function(froggy_draw_ctx) {
            const xoff = 24; 

            /*
            froggy_draw_ctx.ctx.fillStyle = "#FFFFFF";
            froggy_draw_ctx.ctx.fillRect(0, 0, 256, 256);

                this.font_controller.text(froggy_draw_ctx, "heavy rain", xoff + 8, this.text_y - this.font_controller.font.height / 2 - 8);
                this.font_controller.text(froggy_draw_ctx, "eleven degrees", xoff - 12, this.text_y - this.font_controller.font.height / 2 + 8);
                */
            
            if (this.visible)
            {
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
                   else
                   {
                        this.font_controller.set_Font_small();
                        this.font_controller.text(froggy_draw_ctx, "go", 10 * SCALE - 12, 16 * SCALE - this.font_controller.font.height / 2);
                   }
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