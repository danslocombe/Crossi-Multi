import { SCALE} from "./constants.js";
import { dan_lerp, diff} from "./utils.js";
import { create_whiteout, create_dust, create_corpse, create_bubble, create_pinwheel } from "./visual_effects.js";
import { MOVE_T, spr_shadow, sprites_list, colours_list, move_sounds_list, names } from "./character_assets.js";

let spr_crown = new Image(8, 6);
spr_crown.src = '/sprites/spr_crown.png';

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

function move_effects(x, y, simple_entities, player_actor, audio_manager) {
    for (let i = 0; i < 2; i++) {
        const dust_off = Math.random() * (3 / SCALE);
        const dust_dir = Math.random() * 2 * 3.141;
        const dust_x = x + dust_off * Math.cos(dust_dir);
        const dust_y = y + dust_off * Math.sin(dust_dir);
        simple_entities.push(create_dust(dust_x, dust_y));
    }

    audio_manager.play(player_actor.move_sound);
}

export function create_player_remote(client, player_id, audio_manager) {
    let source = {
        client : client,
        player_id : player_id,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        moving : false,
        x_flip : 1,
        frame_id : 0,
        audio_manager : audio_manager,

        tick : function(player_state, simple_entities, player_actor) {
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
                move_effects(x, y, simple_entities, player_actor, this.audio_manager);
            }

            this.moving = moving;

            this.x = x;
            this.y = y;
        }
    };

    return create_player_actor_from_id_and_source(player_id, source, audio_manager)
}

export function create_player_local(client, key_event_source, audio_manager) {
    const player_id = client.get_local_player_id();
    let source = {
        client : client,
        player_id : player_id,
        x : 0,
        y : 0,
        moving : false,
        x_flip : 1,
        frame_id : 0,
        audio_manager : audio_manager,

        tick : function(player_state, simple_entities, player_actor) {
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

                move_effects(x, y, simple_entities, player_actor, this.audio_manager);

                if (player_state.pushing >= 0) {
                    this.audio_manager.play(snd_push);
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

    return create_player_actor_from_id_and_source(player_id, source, audio_manager)
}

function create_player_actor_from_id_and_source(id, source, audio_manager) {
    // player ids start from 1
    const sprites = sprites_list[id - 1];
    const move_sound = move_sounds_list[id - 1];
    const colour = colours_list[id - 1];
    const name = names[id - 1];
    return create_player_actor(sprites, move_sound, colour, name, source, audio_manager)
}

function create_crown(owning_player, i) {
    return {
        x: 0,
        y: 0,
        foreground_depth: 1,
        visible : false,
        t : 0,
        is_alive : true,
        crown_i : i,
        owning_player : owning_player,

        alive : function(max_y) {
            return this.is_alive;
        },

        tick : function() {
            this.x = this.owning_player.x + 1;
            this.y = this.owning_player.y - (5*this.crown_i + 5);
            if (this.t > this.crown_i * 2) {
                this.visible = true;
            }
            this.t += 1;
            if (this.t > 240 + this.crown_i * 3) {
                this.is_alive = false;
            }
        },

        draw : function(froggy_draw_ctx) {
            const xx = this.x + froggy_draw_ctx.x_off + Math.round(0.7*Math.sin(1.1235 * this.crown_i + this.t / 13));
            const yy = this.y + froggy_draw_ctx.y_off;

            froggy_draw_ctx.ctx.drawImage(
                spr_crown,
                0,
                0,
                spr_crown.width,
                spr_crown.height,
                xx,
                yy,
                spr_crown.width,
                spr_crown.height);
        }
    }
}

function create_player_actor(sprites, move_sound, colour, name, source, audio_manager) {
    return {
        sprite : sprites.spr,
        sprite_dead : sprites.spr_dead,
        sprite_name : sprites.spr_name,
        audio_manager : audio_manager,
        colour : colour,
        move_sound : move_sound,
        source : source,
        name: name,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        created_corpse : false,
        t : 0,
        //lobby_ready : false,
        pinwheel : null,
        in_bush : false,

        tick : function(state, entities, rules_state) {
            this.t += 1;
            const alive_state = this.source.client.player_alive_state_json(this.source.player_id);
            //console.log("PlayerId: " + this.source.player_id + "  alive_state: " + alive_state);
            if (alive_state === '"NotInGame"') {
                return;
            }
            if (alive_state === '"Dead"') {
                if (!this.created_corpse) {
                    this.created_corpse = true;
                    const is_river = this.source.client.is_river(state.y);
                    if (!is_river)
                    {
                        const corpse = create_corpse(this.x, this.y, this.sprite_dead);
                        entities.simple_entities.push(corpse);
                        this.audio_manager.play(snd_hit_car);
                    }
                    else {
                        for (let i = 0; i < 2; i++) {
                            // @CLEANUP we use source.x which is in game-coordinates not screen coords and multiplied in bubble draw
                            const bubble_off = Math.random() * (3 / SCALE);
                            const bubble_dir = Math.random() * 2 * 3.141;
                            const bubble_x = this.source.x + bubble_off * Math.cos(bubble_dir);
                            const bubble_y = this.source.y + bubble_off * Math.sin(bubble_dir);
                            entities.simple_entities.push(create_bubble(bubble_x, bubble_y));
                        }

                        this.audio_manager.play(snd_drown);
                    }

                    const whiteout = create_whiteout()
                    entities.simple_entities.push(whiteout);

                }

                return;
            }
            this.source.tick(state, entities.simple_entities, this, rules_state);

            this.x = this.source.x * SCALE;
            this.y = this.source.y * SCALE;

            this.dynamic_depth = this.y;

            const bush = get_colliding_bush(this.x, this.y, entities.bushes);
            if (bush) {
                bush.mark_in();
                this.in_bush = true;
            }
            else {
                this.in_bush = false;
            }

            /*
            if (rules_state && rules_state.fst.Lobby) {
                this.lobby_ready = rules_state.fst.Lobby.ready_states.inner[this.source.player_id];
            }
            else {
                this.lobby_ready = false;
            }
            */

            if (this.t == 1)
            {
                /*
                this.pinwheel = create_pinwheel(this.x + 4, this.y + 4, {depth: 1000});
                simple_entities.push(this.pinwheel);
                */
            }

            if (this.pinwheel)
            {
                this.pinwheel.set_pos(this.x + 4, this.y + 4);
                this.pinwheel.tick();
                this.pinwheel.set_vel(Math.max(0, 1 - (this.pinwheel.t / 120)));
            }
        },
        new_round : function(warmup_state, simple_entities) {
            this.created_corpse = false;

            if (warmup_state.win_counts && warmup_state.win_counts.inner) {
                const win_count = warmup_state.win_counts.inner[this.source.player_id];
                if (win_count && win_count > 0) {
                    for (let i = 0; i < win_count; i++) {
                        simple_entities.push(create_crown(this, i));
                    }
                }
            } 
        },
        draw : function(froggy_draw_ctx) {
            // hackyyy
            const alive_state = this.source.client.player_alive_state_json(this.source.player_id);
            if (alive_state !== '"Alive"') {
                return;
            }

            froggy_draw_ctx.ctx.save();

            let sprite = this.sprite;

            let x = this.x + froggy_draw_ctx.x_off;
            const y = this.y + froggy_draw_ctx.y_off - 1;
            const frame_id = this.source.frame_id;

            if (this.source.x_flip == -1) {
                x = -x - 8;
                froggy_draw_ctx.ctx.scale(-1, 1);
            }

            /*
            // Exclamation mark on head
            if (this.lobby_ready) {
                froggy_draw_ctx.ctx.strokeStyle = this.colour;
                froggy_draw_ctx.ctx.beginPath();
                const tt = this.t + 100 * this.source.player_id;
                const xx = x + 4 + Math.round(Math.sin(tt / 13));
                const yy = y - 1 + Math.round(Math.sin(tt / 7));
                froggy_draw_ctx.ctx.moveTo(xx, yy - 1);
                froggy_draw_ctx.ctx.lineTo(xx, yy - 3);
                froggy_draw_ctx.ctx.stroke();
                froggy_draw_ctx.ctx.moveTo(xx, yy - 4);
                froggy_draw_ctx.ctx.lineTo(xx, yy - 9);
                froggy_draw_ctx.ctx.stroke();
            }
            */

            // TODO make transparent
            // do in sprite
            if (!this.in_bush)
            {
                froggy_draw_ctx.ctx.drawImage(spr_shadow, x, y + 2);
            }

            froggy_draw_ctx.ctx.drawImage(sprite, SCALE*frame_id, 0, SCALE, SCALE, x, y, SCALE, SCALE);

            froggy_draw_ctx.ctx.restore();
        },
    }
}

function get_colliding_bush(x, y, bushes)
{
    const xx = x + SCALE / 2;
    const yy = y + SCALE / 2;
    for (let bush of bushes)
    {
        if (xx >= bush.x && xx < bush.x + SCALE &&
            yy >= bush.y && yy < bush.y + SCALE)
        {
            return bush;
        }
    }

    return null;
}