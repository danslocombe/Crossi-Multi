import { SCALE} from "./constants.js";

function load_sprites(name) {
    let spr = new Image(SCALE, SCALE);
    spr.src = '/sprites/spr_' + name + ".png";
    let spr_flipped = new Image(SCALE, SCALE);
    spr_flipped.src = '/sprites/spr_' + name + "_flipped.png";
    let spr_dead = new Image(SCALE, SCALE);
    spr_dead.src = '/sprites/spr_' + name + "_dead.png";

    return {
        spr : spr,
        spr_flipped : spr_flipped,
        spr_dead : spr_dead,
    }
}

let spr_shadow = new Image(SCALE, SCALE);
spr_shadow.src = '/sprites/spr_shadow.png';

let sprites_list = [
    load_sprites('frog'),
    load_sprites('mouse'),
    load_sprites('bird'),
    load_sprites('snake'),
]

let sounds_list = [
    new Audio('/sounds/snd_move1.wav'),
    new Audio('/sounds/snd_move2.wav'),
    new Audio('/sounds/snd_move3.wav'),
    new Audio('/sounds/snd_move4.wav'),
]

for (let sound of sounds_list) {
    sound.volume = 0.15;
}

let spr_dust = new Image(SCALE,SCALE);
spr_dust.src = "/sprites/spr_dust.png";
const spr_smoke_count = 4;

function create_dust(x, y) {
    return {
        frame_id : Math.floor(Math.random() * spr_smoke_count),
        scale : 0.5 + Math.random() * 0.6,
        static_depth : 100,
        x : x,
        y : y,
        tick : function() {
            this.scale -= 0.025;
        },

        alive: function() {
            return this.scale > 0;
        },

        draw : function(froggy_draw_ctx) {
            //const x = this.x + 0 + (1-this.scale)*4 + froggy_draw_ctx.x_off;
            //const y = this.y + 0 + (1-this.scale)*4 + froggy_draw_ctx.y_off;
            const x = SCALE*(this.x + 0.25) + (1-this.scale) + froggy_draw_ctx.x_off;
            const y = SCALE*(this.y + 0.25) + (1-this.scale) + froggy_draw_ctx.y_off;
            froggy_draw_ctx.ctx.drawImage(spr_dust, SCALE*this.frame_id, 0, SCALE, SCALE, x, y, SCALE*this.scale, SCALE*this.scale);
        }
    }
}

function create_corpse(x, y, spr_dead) {
    return {
        spr : spr_dead,
        x : x,
        y : y,
        dynamic_depth : y,

        tick : () => {},
        alive : () => true,
        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.drawImage(
                this.spr,
                0,
                0,
                SCALE,
                SCALE,
                x + froggy_draw_ctx.x_off,
                y + froggy_draw_ctx.y_off,
                SCALE,
                SCALE);
        }
    }
}

function dan_lerp(x0, x, k) {
    return (x0 * (k-1) + x) / k;
}

function diff(x, y) {
    return Math.abs(x - y);
}

// frog has duplicate frame for some reason
//const frame_count = 6;
const player_frame_count = 5;

export function create_player_remote(client, player_id) {
    let source = {
        client : client,
        player_id : player_id,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        moving : false,
        states : [],
        x_flip : 1,
        frame_id : 0,

        tick : function(player_state, simple_entities, player_def) {
            this.states.push(player_state);

            // dumb implementation
            // basically inverse kinomatics, play back animations to match movement
            // Lerp to current pos
            // TODO if local player is pushing then we should be much tighter on this
            const k = 4;

            let x1 = player_state.pos.Coord.x;
            let y1 = player_state.pos.Coord.y;

            if (player_state.move_state != "Stationary") {
                const moving_state = player_state.move_state.Moving;
                if (moving_state.target.Coord) {
                    x1 = moving_state.target.Coord.x;
                    y1 = moving_state.target.Coord.y;
                }
            }

            let x = dan_lerp(this.x, player_state.pos.Coord.x, k);
            let y = dan_lerp(this.y, player_state.pos.Coord.y, k);

            const kk = 0.45;
            if (diff(x, player_state.pos.Coord.x) < kk) {
                x = player_state.pos.Coord.x;
            }
            if (diff(y, player_state.pos.Coord.y) < kk) {
                y = player_state.pos.Coord.y;
            }

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
                // Make dust
                for (let i = 0; i < 2; i++) {
                    const dust_off = Math.random() * (3 / SCALE);
                    const dust_dir = Math.random() * 2 * 3.141;
                    const dust_x = x + dust_off * Math.cos(dust_dir);
                    const dust_y = y + dust_off * Math.sin(dust_dir);
                    simple_entities.push(create_dust(dust_x, dust_y));
                }

                player_def.move_sound.play();
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
            if (player_state.pos.Coord)
            {
                let x,y;
                const x0 = player_state.pos.Coord.x;
                const y0 = player_state.pos.Coord.y;

                const moving = player_state.move_state != "Stationary";
                const pushed = false;
                if (moving) {
                    const moving_state = player_state.move_state.Moving;
                    // TODO don't replicate this constant
                    const MOVE_T = 7 * (1000 * 1000 / 60);
                    const lerp_t = (1 - moving_state.remaining_us / MOVE_T);

                    let x1 = x0;
                    let y1 = y0;
                    if (moving_state.target.Coord) {
                        x1 = moving_state.target.Coord.x;
                        y1 = moving_state.target.Coord.y;
                    }

                    x = x0 + lerp_t * (x1 - x0);
                    y = y0 + lerp_t * (y1 - y0);
                    this.frame_id = Math.floor(lerp_t * (player_frame_count - 1));

                }
                else {
                    x = x0;
                    y = y0;
                    this.frame_id = 0;
                }

                // Started moving
                if (moving && !this.moving) {

                    // Make dust
                    for (let i = 0; i < 2; i++) {
                        const dust_off = Math.random() * (3 / SCALE);
                        const dust_dir = Math.random() * 2 * 3.141;
                        const dust_x = x + dust_off * Math.cos(dust_dir);
                        const dust_y = y + dust_off * Math.sin(dust_dir);
                        simple_entities.push(create_dust(dust_x, dust_y));
                    }

                    player_def.move_sound.play();
                }
                this.x = x;
                this.y = y;
                this.moving = moving;
            }
        }
    }

    let listener = key_event_source.add_listener();
    listener.on_keydown = function(input) {
        if (input == "Left") {
            source.x_flip = -1;
        }
        if (input == "Right") {
            source.x_flip = 1;
        }
    };

    return player_def_from_player_id(player_id, source)
}

function player_def_from_player_id(id, source) {
    // player ids start from 1
    let sprites = sprites_list[id - 1];
    let move_sound = sounds_list[id - 1];
    return create_player_def(sprites, move_sound, source)
}

function create_player_def(sprites, move_sound, source) {
    return {
        sprite : sprites.spr,
        sprite_flipped : sprites.spr_flipped,
        sprite_dead : sprites.spr_dead,
        move_sound : move_sound,
        source : source,
        x : 0,
        y : 0,
        dynamic_depth : 0,
        created_corpse : false,

        tick : function(state, simple_entities) {
            if (!this.source.client.player_alive(this.source.player_id)) {
                if (!this.created_corpse) {
                    this.created_corpse = true;
                    const corpse = create_corpse(this.x, this.y, this.sprite_dead);
                    simple_entities.push(corpse);
                }

                return;
            }
            this.source.tick(state, simple_entities, this);

            this.x = this.source.x * SCALE;
            this.y = this.source.y * SCALE;
            this.dynamic_depth = this.y;
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

            // TODO make transparent
            // do in sprite
            crossy_draw_ctx.ctx.drawImage(spr_shadow, x, y + 2);

            crossy_draw_ctx.ctx.drawImage(sprite, SCALE*frame_id, 0, SCALE, SCALE, x, y, SCALE, SCALE);
        },
    }
}