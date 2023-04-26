import { SCALE } from "./constants"
import { rand_push_spectator } from "./spectator";
import { get_target_y_from_rules_state, get_round_id_from_rules_state } from "./utils";

var sprite_map_def = {
    "foliage" : {w: SCALE, y : SCALE, frames : 6, depth : 250},
    "stand" : {w : 40, y : 74, depth : 100},
};

var sprite_map = {}

function create_sprites(name, def) {
    if (!def.srcs || !Array.isArray(def.srcs)) {
        let spr = new Image(def.w, def.h);
        spr.src = "sprites/spr_" + name + ".png";
        return [spr];
    }

    let sprites = [];
    for (src of def.srcs) {
        let spr = new Image(def.w, def.h);
        src = "sprites/" + src + ".png";
        sprites.push(spr);
    }

    return sprites;
}

for (const key in sprite_map_def) {
    const def = sprite_map_def[key];
    sprite_map[key] = create_sprites(key, def);
}

function get_sprite(sprite_name) {
    const lookup_result = sprite_map[sprite_name];
    if (Array.isArray(lookup_result)) {
        const i = Math.floor(Math.random() * lookup_result.length);
        return lookup_result[i];
    }
    
    return lookup_result;
}

export function create_prop(x, y, prop_name) {
    const spr = get_sprite(prop_name);
    const frames = sprite_map_def[prop_name].frames;
    let frame = 0;
    if (frames !== undefined) {
        frame = Math.floor(Math.random() * frames);
    }

    let prop = {
        spr : spr,
        x : x,
        y : y,
        frame : frame,
        w : spr.width,
        h : spr.height,
        flipped : false,

        tick : () => {},
        alive : function(camera_y_max) {
            return this.y < this.h + camera_y_max;
        },
        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.save();
            let x = this.x + froggy_draw_ctx.x_off;
            if (this.flipped) {
                x = -x - this.w;
                froggy_draw_ctx.ctx.scale(-1, 1);
            }
            froggy_draw_ctx.ctx.drawImage(
                this.spr,
                this.frame * this.w,
                0,
                this.w,
                this.h,
                x,
                this.y + froggy_draw_ctx.y_off,
                this.w,
                this.h);
            froggy_draw_ctx.ctx.restore();
        }
    };


    let def_depth_mod = sprite_map_def[prop_name].dynamic_depth;
    if (def_depth_mod !== undefined) {
        prop.dynamic_depth = prop.y + def_depth_mod;
    }

    let def_depth = sprite_map_def[prop_name].depth;
    prop.depth = def_depth;

    return prop;
}

export function create_prop_controller() {
    return {
        last_generated_round : -1,
        last_generated_game : -1,
        gen_to: 20,

        tick : function(rules_state, simple_entities, client) {
            if (!rules_state) {
                return;
            }

            const round_id = get_round_id_from_rules_state(rules_state);
            const game_id = rules_state.game_id;

            //if (round_id >= 0 && this.last_generated_round != round_id) {
            if (this.last_generated_round != round_id || this.last_generated_game != game_id) {
                console.log("Creating props");
                this.last_generated_round = round_id;
                this.last_generated_game = game_id;
                this.gen_to = 20;

                console.log("Round Id " + round_id);
                console.log("Game Id " + game_id);

                const stand_left = create_prop(4, 10*SCALE, "stand");
                const stand_right = create_prop(14* SCALE + 4, 10*SCALE, "stand");
                stand_right.flipped = true;
                simple_entities.push(stand_left);
                simple_entities.push(stand_right);

                const prob_stands = 0.7;
                const ymin = stand_left.y + 8;
                for (let ix = 0; ix < 4; ix++) {
                    for (let iy = 0; iy < 4; iy++) {
                        const x = stand_left.x + ix * SCALE;
                        const y = ymin + x / 2 + 4 + SCALE * iy;
                        rand_push_spectator(x + 4, y, false, prob_stands, simple_entities);
                    }
                }

                for (let ix = 0; ix < 4; ix++) {
                    for (let iy = 0; iy < 4; iy++) {
                        const x = stand_right.x + ix * SCALE;
                        const y = ymin - 4 * ix + 16 + SCALE * iy;
                        rand_push_spectator(x + 4, y, true, prob_stands, simple_entities);
                    }
                }

                const prob_front = 0.35;
                for (let iy = 0; iy < 7; iy++) {
                    // In front of left stand
                    const yy = 13 * SCALE + iy * SCALE;
                    let xx = stand_left.x + 4 * SCALE + 4;
                    rand_push_spectator(xx, yy, false, prob_front, simple_entities);

                    // In front of right stand
                    xx = 14 * SCALE;
                    rand_push_spectator(xx, yy, true, prob_front, simple_entities);
                }

                const prob_below = 0.2;
                for (let ix = 0; ix < 5; ix++) {
                    for (let iy = 0; iy < 2; iy++) {
                        const yy = 18 * SCALE + iy * SCALE;

                        // Below left stand
                        let xx = stand_left.x + ix * SCALE - SCALE + 4;
                        rand_push_spectator(xx, yy, false, prob_below, simple_entities);

                        // Below right stand
                        xx = 15 * SCALE + ix * SCALE;
                        rand_push_spectator(xx, yy, true, prob_below, simple_entities);
                    }
                }
            }

            const gen_to_target = get_target_y_from_rules_state(rules_state);
            if (gen_to_target !== undefined)
            {
                while (this.gen_to > gen_to_target - 4) {
                    if (client.is_path(this.gen_to)) {
                        for (let x = 0; x < 20; x++) {
                            // TODO make a call to the rust rng here so we get a deterministic result across games
                            if (Math.random() < 0.15) {
                                simple_entities.push(create_prop(x*SCALE, this.gen_to*SCALE, "foliage"));
                            }
                        }
                    }

                    this.gen_to -= 1;
                }
            }
        }
    }
}