import { SCALE } from "./constants.js";

export const spr_bush_back = new Image(SCALE, SCALE);
spr_bush_back.src = '/sprites/spr_bush_back.png';

export function create_bush(x, y)
{
    return {
        x : x,
        y : y,

        dynamic_depth : y,

        tick : function() {
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

    }
}