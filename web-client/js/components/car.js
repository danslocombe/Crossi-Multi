import { SCALE} from "./constants.js";

const spr_car_width = 24;
const spr_car_height = 16;
const car_sprite_count = 4;

let spr_car = new Image(spr_car_width, spr_car_height);
spr_car.src = '/sprites/spr_car_flipped.png';

let spr_car_flipped = new Image(spr_car_width, spr_car_height);
spr_car_flipped.src = '/sprites/spr_car.png';


export function create_car(car) {
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
        dynamic_depth : y + spr_car_height / 2,
        flipped : flipped,
        frame_id : frame_id,
        spr : spr,
        draw : function(froggy_draw_ctx) {
            const xx = this.x - spr_car_width / 2 + froggy_draw_ctx.x_off;
            const yy = this.y - spr_car_height / 2 + froggy_draw_ctx.y_off;

            // Add 100 to x to make sure we end up with a postive value
            this.frame_id = Math.floor((100 + this.x) / 8) % car_sprite_count;

            froggy_draw_ctx.ctx.drawImage(this.spr,
                spr_car_width*this.frame_id,
                0,
                spr_car_width,
                spr_car_height,
                xx,
                yy,
                spr_car_width,
                spr_car_height);
        }
    };
}