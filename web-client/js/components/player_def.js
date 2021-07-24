const SCALE = 8;

let spr_frog = new Image(SCALE,SCALE);
spr_frog.src = "/sprites/spr_frog.png";

let spr_frog_flipped = new Image(SCALE,SCALE);
spr_frog_flipped.src = "/sprites/spr_frog_flipped.png";

var snd_frog_move = new Audio('/sounds/snd_move1.wav');
snd_frog_move.volume = 0.15;

let spr_dust = new Image(SCALE,SCALE);
spr_dust.src = "/sprites/spr_dust.png";
const spr_smoke_count = 4;

function create_dust(x, y) {
    return {
        frame_id : Math.floor(Math.random() * spr_smoke_count),
        scale : 0.5 + Math.random() * 0.6,
        x : x,
        y : y,
        tick : function() {
            this.scale -= 0.025;
        },

        alive: function() {
            return this.scale > 0;
        },

        draw : function(ctx) {
            const x = SCALE*(this.x + 0.25) + (1-this.scale);
            const y = SCALE*(this.y + 0.25) + (1-this.scale);
            ctx.drawImage(spr_dust, SCALE*this.frame_id, 0, SCALE, SCALE, x, y, SCALE*this.scale, SCALE*this.scale);
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

export function create_player_remote(player_id) {
    let source = {
        player_id : player_id,
        x : 0,
        y : 0,
        moving : false,
        states : [],
        x_flip : 1,
        frame_id : 0,

        tick : function(player_state, simple_entities) {
            this.states.push(player_state);

            // dumb implementation
            // basically inverse kinomatics, play back animations to match movement
            // Lerp to current pos
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

                // Make function
                        // Make dust
                        for (let i = 0; i < 2; i++) {
                            const dust_off = Math.random() * (3 / SCALE);
                            const dust_dir = Math.random() * 2 * 3.141;
                            const dust_x = x + dust_off * Math.cos(dust_dir);
                            const dust_y = y + dust_off * Math.sin(dust_dir);
                            simple_entities.push(create_dust(dust_x, dust_y));
                        }

                        snd_frog_move.play();
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

        tick : function(player_state, simple_entities) {
            if (player_state.pos.Coord)
            {
                let x,y;
                const x0 = player_state.pos.Coord.x;
                const y0 = player_state.pos.Coord.y;

                const moving = player_state.move_state != "Stationary";
                if (moving) {
                    console.log("Local Lerping " + player_state.id);
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

                    snd_frog_move.play();
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
    return create_player_def(spr_frog, spr_frog_flipped, snd_frog_move, source)
}

function create_player_def(spr, spr_mirrored, move_sound, source) {
    return {
        sprite : spr,
        sprite_mirrored : spr_mirrored,
        move_sound : move_sound,
        source : source,
        tick : function(state, simple_entities) {
            this.source.tick(state, simple_entities);
        },
        draw : function(ctx) {
            let sprite = this.sprite;
            if (this.source.x_flip == -1) {
                sprite = this.sprite_mirrored;
            }
            const x = this.source.x;
            const y = this.source.y;
            const frame_id = this.source.frame_id;
            ctx.drawImage(sprite, SCALE*frame_id, 0, SCALE, SCALE, SCALE*x, SCALE*y, SCALE, SCALE);
        },
    }
}