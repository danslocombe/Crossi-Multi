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
            if (this.in_lobby)
            {
                if (!this.local_player_has_moved)
                {
                    this.font_controller.text(froggy_draw_ctx, "move with", xoff + 8, this.text_y - this.font_controller.text_height / 2 - 8);
                    this.font_controller.text(froggy_draw_ctx, "the arrow keys", xoff - 12, this.text_y - this.font_controller.text_height / 2 + 8);
                }
                else
                {
                    if (!this.multiple_players)
                    {
                        this.font_controller.text(froggy_draw_ctx, "invite friends", xoff - 16, this.text_y - this.font_controller.text_height / 2);
                    }
                    else if (!this.local_player_has_readied)
                    {
                        this.font_controller.text(froggy_draw_ctx, "press space", xoff, this.text_y - this.font_controller.text_height / 2 - 8);
                        this.font_controller.text(froggy_draw_ctx, "to ready up", xoff, this.text_y - this.font_controller.text_height / 2 + 8);
                    }
                }
            }
        },
    }
}
