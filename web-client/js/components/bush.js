
export const spr_bush_back = new Image(SCALE, SCALE);
spr_bush_back.src = '/sprites/spr_bush_back.png';

export function create_bush(x, y)
{
    return {
        x : x,
        y : y,

        dynamic_depth : y,
        frame : 0,
        flipped: flipped,


        tick : function() {
        },
        alive : function(camera_y_max) {
            return this.y <= camera_y_max;
        },
        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.drawImage(
                spr_bush_back,
                SCALE,
                0,
                SCALE,
                SCALE,
                this.x,
                this.y, 
                SCALE,
                SCALE);

        }

    }
}