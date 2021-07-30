import { SCALE} from "./constants.js";

const spr_car_width = 24;
const spr_car_height = 16;
let spr_car = new Image(spr_car_width, spr_car_height);
spr_car.src = '/sprites/spr_car_flipped.png';

let spr_car_flipped = new Image(spr_car_width, spr_car_height);
spr_car_flipped.src = '/sprites/spr_car.png';

export function make_car(car) {
    const x = car[0] * SCALE;
    const y = car[1] * SCALE;
    const flipped = car[2];
    const frame_id = 0;
    let spr = spr_car;
    if (flipped) {
        spr = spr_car_flipped;
    }

    return {
        x : x,
        y : y,
        dynamic_depth : y,
        flipped : flipped,
        frame_id : frame_id,
        spr : spr,
        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.drawImage(this.spr,
                spr_car_width*this.frame_id,
                0,
                spr_car_width,
                spr_car_height,
                this.x - spr_car_width / 2 + froggy_draw_ctx.x_off,
                this.y - spr_car_height / 2 + froggy_draw_ctx.y_off,
                spr_car_width,
                spr_car_height);
        }
    };
}