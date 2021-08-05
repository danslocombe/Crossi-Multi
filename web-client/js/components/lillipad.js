import { SCALE} from "./constants.js";

let spr_log = new Image(SCALE, SCALE);
spr_log.src = '/sprites/spr_log.png';


export function create_lillipad(lillipad) {
    const x = lillipad[0] * SCALE;
    const y = lillipad[1] * SCALE;
    //const flipped = car[2];
    const frame_id = 0;
    let spr = spr_log;

    return {
        x : x,
        y : y,

        // Make sure that we draw under players
        dynamic_depth : y - 1000,

        frame_id : frame_id,
        spr : spr,
        draw : function(froggy_draw_ctx) {
            const xx = this.x + froggy_draw_ctx.x_off;
            const yy = this.y + froggy_draw_ctx.y_off;

            froggy_draw_ctx.ctx.drawImage(this.spr,
                SCALE*this.frame_id,
                0,
                SCALE,
                SCALE,
                xx,
                yy,
                SCALE,
                SCALE);
        }
    };
}