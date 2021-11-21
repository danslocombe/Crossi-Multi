import { SCALE} from "./constants.js";

let spr_move_tiles = new Image(SCALE, SCALE);
spr_move_tiles.src = "/sprites/spr_move_tiles.png";

function create_overlay(overlay_obj) {
    let frame_id = 0;
    if (overlay_obj.input === "Up")
    {
        frame_id = 0;
    }
    else if (overlay_obj.input === "Down")
    {
        frame_id = 1;
    }
    else if (overlay_obj.input === "Left")
    {
        frame_id = 2;
    }
    else if (overlay_obj.input === "Right")
    {
        frame_id = 3;
    }
    return {
        x : overlay_obj.precise_coords.x,
        y : overlay_obj.precise_coords.y,
        dynamic_depth : overlay_obj.precise_coords.y + 16,
        frame_id : frame_id,
        draw : function(froggy_draw_ctx) {
            const xx = (this.x) * SCALE + froggy_draw_ctx.x_off;
            const yy = (this.y) * SCALE + froggy_draw_ctx.y_off;

            froggy_draw_ctx.ctx.drawImage(spr_move_tiles, SCALE*frame_id, 0, SCALE, SCALE, xx, yy, SCALE, SCALE);
        }
    }
}
export function create_from_lilly_overlay(overlay) {
    let drawables = [];

    for (const overlay_obj of overlay) {
        drawables.push(create_overlay(overlay_obj));
    }

    return drawables;
}