import { SCALE} from "./constants.js";
import { spr_shadow, sprites_list } from "./character_assets.js";

export function rand_push_spectator(x, y, flipped, prob, simple_entities, client) {
    const rand = client.rand_for_prop_unit(x, y, "create_spectator");
    if (rand < prob)
    {
        simple_entities.push(create_spectator(x, y, flipped, client));
    }
}

export function create_spectator(x, y, flipped, client) {
    const rand = client.rand_for_prop_unit(x, y, "spectator_sprite");
    const i = Math.floor(rand * sprites_list.length);

    return {
        i : i,
        x0 : x,
        y0 : y,
        x : x,
        y : y,

        dynamic_depth : y,
        frame : 0,
        flipped: flipped,

        jump_t : 0,
        jump_t_max : 10,

        tick : function() {
            if (this.jump_t <=0 && Math.random() < 0.016) {
                this.jump_t = this.jump_t_max;
            }

            if (this.jump_t > 0) {
                this.jump_t -= 1;
                this.y = this.y0 - Math.sin(3.141 * this.jump_t / this.jump_t_max) * 4;
                this.frame = Math.floor(5 * (this.jump_t / this.jump_t_max));
            }
            else {
                this.y = this.y0;
                this.frame = 0;
            }
        },
        alive : function(camera_y_max) {
            return this.y <= camera_y_max;
        },
        draw : function(froggy_draw_ctx) {
            let x = this.x + froggy_draw_ctx.x_off;
            let y0 = this.y0 + froggy_draw_ctx.y_off;
            let y = this.y + froggy_draw_ctx.y_off;
            let spr = sprites_list[i].spr;
            froggy_draw_ctx.ctx.save();
            if (this.flipped) {
                x = -x - 8;
                froggy_draw_ctx.ctx.scale(-1, 1);
            }

            froggy_draw_ctx.ctx.drawImage(spr_shadow, x, y0 + 2);

            froggy_draw_ctx.ctx.drawImage(
                spr,
                this.frame * SCALE,
                0,
                SCALE,
                SCALE,
                x,
                y, 
                SCALE,
                SCALE);

            froggy_draw_ctx.ctx.restore();
        }
    }
}