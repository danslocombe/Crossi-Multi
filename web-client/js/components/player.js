import { SCALE} from "./constants.js";
import { dan_lerp, diff} from "./utils.js";
import { create_whiteout, create_dust, create_corpse, create_bubble } from "./visual_effects.js";
import { MOVE_T, spr_shadow, sprites_list, colours_list, move_sounds_list } from "./character_assets.js";

const snd_push = new Audio('/sounds/snd_push.wav');
snd_push.volume = 0.14;

const snd_hit_car = new Audio('/sounds/snd_car.wav');
snd_hit_car.volume = 0.25;

const snd_drown = new Audio('/sounds/snd_drown_bubbles.wav');
snd_drown.volume = 0.75;

// frog has duplicate frame for some reason
//const frame_count = 6;
const player_frame_count = 5;

function lerp_snap(x0, y0, x1, y1)
{
    const k = 4;
    let x = dan_lerp(x0, x1, k);
    let y = dan_lerp(y0, y1, k);

    const dist = Math.sqrt((x - x1) * (x - x1) + (y - y1) * (y - y1));

    const snap_dir_small = 0.15;
    const snap_dir_large = 3;

    if (dist < snap_dir_small || dist > snap_dir_large) {
        x = x1;
        y = y1;
    }

    return {
        x : x,
        y : y,
    }
}

function move_effects(x, y, simple_entities, player_def) {
    for (let i = 0; i < 2; i++) {
        const dust_off = Math.random() * (3 / SCALE);
        const dust_dir = Math.random() * 2 * 3.141;
        const dust_x = x + dust_off * Math.cos(dust_dir);
        const dust_y = y + dust_off * Math.sin(dust_dir);
        simple_entities.push(create_dust(dust_x, dust_y));
    }

    player_def.move_sound.play();
}

export function create_player_remote(client, player_id) {
    let source = {
        client : client,
        player_id : player_id,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        moving : false,
        x_flip : 1,
        frame_id : 0,

        tick : function(player_state, simple_entities, player_def) {
            // dumb implementation
            // basically inverse kinomatics, play back animations to match movement
            // Lerp to current pos
            // TODO if local player is pushing then we should be much tighter on this
            let x1 = player_state.x
            let y1 = player_state.y

            if (player_state.moving) {
                const interp_t = (player_state.remaining_move_dur / MOVE_T);
                x1 = player_state.x * (interp_t) + player_state.t_x * (1-interp_t);
                y1 = player_state.y * (interp_t) + player_state.t_y * (1-interp_t);
            }

            const new_p = lerp_snap(this.x, this.y, x1, y1);
            const x = new_p.x;
            const y = new_p.y;

            const delta = 0.1;
            if (x > this.x + delta) {
                this.x_flip = 1;
            }
            else if (x < this.x - delta){
                this.x_flip = -1;
            }

            let moving = false;

            if (diff(x, this.x) > delta || diff(y, this.y) > delta) {
                moving = true;
                this.frame_id = (this.frame_id + 1) % player_frame_count
            }
            else {
                this.frame_id = 0;
                moving = false;
            }

            if (moving && !this.moving) {
                move_effects(x, y, simple_entities, player_def);
            }

            this.moving = moving;

            this.x = x;
            this.y = y;
        }
    };

    return player_def_from_player_id(player_id, source)
}

export function create_player_local(client, key_event_source) {
    const player_id = client.get_local_player_id();
    let source = {
        client : client,
        player_id : player_id,
        x : 0,
        y : 0,
        moving : false,
        x_flip : 1,
        frame_id : 0,

        tick : function(player_state, simple_entities, player_def) {
            const x0 = player_state.x;
            const y0 = player_state.y;

            let x,y;
            if (player_state.moving) {
                const lerp_t = (1 - player_state.remaining_move_dur / MOVE_T);

                const x1 = player_state.t_x;
                const y1 = player_state.t_y;

                x = x0 + lerp_t * (x1 - x0);
                y = y0 + lerp_t * (y1 - y0);
                this.frame_id = Math.floor(lerp_t * (player_frame_count - 1));

            }
            else {
                const new_p = lerp_snap(this.x, this.y, x0, y0);
                x = new_p.x;
                y = new_p.y;

                const delta = 0.1;
                if (diff(x, this.x) > delta || diff(y, this.y) > delta) {
                    this.frame_id = (this.frame_id + 1) % player_frame_count;
                }
                else {
                    this.frame_id = 0;
                }
            }

            // Started moving
            if (player_state.moving && !this.moving) {

                move_effects(x, y, simple_entities, player_def);

                if (player_state.pushing >= 0) {
                    snd_push.play();
                }
            }

            this.x = x;
            this.y = y;
            this.moving = player_state.moving;
        }
    }

    let listener = key_event_source.add_input_listener();
    listener.on_input_keydown = function(input) {
        if (input === "Left") {
            source.x_flip = -1;
        }
        if (input === "Right") {
            source.x_flip = 1;
        }
    };

    return player_def_from_player_id(player_id, source)
}

function player_def_from_player_id(id, source) {
    // player ids start from 1
    const sprites = sprites_list[id - 1];
    const move_sound = move_sounds_list[id - 1];
    const colour = colours_list[id - 1];
    return create_player_def(sprites, move_sound, colour, source)
}

function create_player_def(sprites, move_sound, colour, source) {
    return {
        sprite : sprites.spr,
        sprite_flipped : sprites.spr_flipped,
        sprite_dead : sprites.spr_dead,
        sprite_name : sprites.spr_name,
        colour : colour,
        move_sound : move_sound,
        source : source,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        created_corpse : false,
        t : 0,
        lobby_ready : false,

        tick : function(state, simple_entities, rule_state) {
            this.t += 1;
            if (!this.source.client.player_alive(this.source.player_id)) {
                if (!this.created_corpse) {
                    this.created_corpse = true;
                    const is_river = source.client.is_river(state.y);
                    if (!is_river)
                    {
                        const corpse = create_corpse(this.x, this.y, this.sprite_dead);
                        simple_entities.push(corpse);
                        snd_hit_car.play();
                    }
                    else {
                        for (let i = 0; i < 2; i++) {
                            const bubble_off = Math.random() * (3 / SCALE);
                            const bubble_dir = Math.random() * 2 * 3.141;
                            const bubble_x = this.source.x + bubble_off * Math.cos(bubble_dir);
                            const bubble_y = this.source.y + bubble_off * Math.sin(bubble_dir);
                            simple_entities.push(create_bubble(bubble_x, bubble_y));
                        }

                        snd_drown.play();
                    }

                    const whiteout = create_whiteout()
                    simple_entities.push(whiteout);

                }

                return;
            }
            this.source.tick(state, simple_entities, this, rule_state);

            this.x = this.source.x * SCALE;
            this.y = this.source.y * SCALE;
            this.dynamic_depth = this.source.y;

            if (rule_state && rule_state.Lobby) {
                this.lobby_ready = rule_state.Lobby.ready_states.inner[source.player_id];
            }
            else {
                this.lobby_ready = false;
            }
        },
        new_round : function() {
            this.created_corpse = false;
        },
        draw : function(crossy_draw_ctx) {
            // hackyyy
            if (!this.source.client.player_alive(this.source.player_id)) {
                return;
            }

            let sprite = this.sprite;
            if (this.source.x_flip == -1) {
                sprite = this.sprite_flipped;
            }

            const x = this.x + crossy_draw_ctx.x_off;
            const y = this.y + crossy_draw_ctx.y_off - 1;
            const frame_id = this.source.frame_id;

            if (this.lobby_ready) {
                crossy_draw_ctx.ctx.strokeStyle = this.colour;
                crossy_draw_ctx.ctx.beginPath();
                const tt = this.t + 100 * this.source.player_id;
                const xx = x + 4 + Math.round(Math.sin(tt / 13));
                const yy = y - 1 + Math.round(Math.sin(tt / 7));
                crossy_draw_ctx.ctx.moveTo(xx, yy - 1);
                crossy_draw_ctx.ctx.lineTo(xx, yy - 3);
                crossy_draw_ctx.ctx.stroke();
                crossy_draw_ctx.ctx.moveTo(xx, yy - 4);
                crossy_draw_ctx.ctx.lineTo(xx, yy - 9);
                crossy_draw_ctx.ctx.stroke();
            }

            // TODO make transparent
            // do in sprite
            crossy_draw_ctx.ctx.drawImage(spr_shadow, x, y + 2);

            crossy_draw_ctx.ctx.drawImage(sprite, SCALE*frame_id, 0, SCALE, SCALE, x, y, SCALE, SCALE);
        },
    }
}