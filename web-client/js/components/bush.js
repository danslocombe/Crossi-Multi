import { SCALE } from "./constants.js";
import { dan_lerp } from "./utils";

export const spr_bush_back = new Image(SCALE, SCALE);
spr_bush_back.src = '/sprites/spr_bush_back.png';

export const spr_bush_foreground = new Image(4*SCALE, SCALE);
spr_bush_foreground.src = '/sprites/spr_bush_foreground.png';

export function create_bush(x, y)
{
    let bush = {
        x : x,
        y : y,

        dynamic_depth : y - 1,

        tick : function() {
        },

        mark_in : function() {
            this.foreground.mark_in();
        },

        alive : function(camera_y_max) {
            return this.y <= camera_y_max;
        },
        draw : function(froggy_draw_ctx) {
            let x = this.x + froggy_draw_ctx.x_off;
            let y = this.y + froggy_draw_ctx.y_off;
            froggy_draw_ctx.ctx.drawImage(
                spr_bush_back,
                0,
                0,
                SCALE,
                SCALE,
                x,
                y, 
                SCALE,
                SCALE);
        }

    };

    bush.foreground = create_bush_foreground(bush);
    return bush;
}

function create_bush_foreground(bush)
{
    const in_time_max = 8;
    return {
        x : bush.x,
        y : bush.y,
        dynamic_depth : bush.dynamic_depth + 2,

        in_time : 0,
        bush : bush,

        tick : function() {
            this.in_time *= 0.95;
        },

        mark_in : function() {
            this.in_time += 1;
            this.in_time = Math.min(this.in_time, in_time_max);
        },

        alive : function(camera_y_max) {
            return this.bush.alive(camera_y_max);
        },

        draw : function(froggy_draw_ctx) {
            if (this.in_time > 0.001) {
                let x = this.bush.x + froggy_draw_ctx.x_off;
                let y = this.bush.y + froggy_draw_ctx.y_off;
                const frame = Math.floor((this.in_time / in_time_max) * 3.99);
                froggy_draw_ctx.ctx.drawImage(
                    spr_bush_foreground,
                    frame * SCALE,
                    0,
                    SCALE,
                    SCALE,
                    x,
                    y, 
                    SCALE,
                    SCALE);
            }

        }

    }
}