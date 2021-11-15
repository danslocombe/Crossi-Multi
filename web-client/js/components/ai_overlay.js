import { SCALE} from "./constants.js";

function create_ai_overlay(overlay_obj) {
    return {
        x : overlay_obj.precise_pos.x,
        y : overlay_obj.precise_pos.y,
        dynamic_depth : overlay_obj.precise_pos.y - 2000,
        type : overlay_obj.draw_type,
        colour : overlay_obj.colour,
        draw : function(froggy_draw_ctx) {
            //console.log("Drawing overlay elem");
            const xx = (this.x + 0.5) * SCALE + froggy_draw_ctx.x_off;
            const yy = (this.y + 0.5) * SCALE + froggy_draw_ctx.y_off;

            const ctx = froggy_draw_ctx.ctx;
            ctx.strokeStyle = this.colour;
            ctx.lineWidth = 1

            if (this.type === "Cross") {
                const cross_off = SCALE / 2
                ctx.beginPath();
                ctx.moveTo(xx - cross_off, yy - cross_off);
                ctx.lineTo(xx + cross_off, yy + cross_off);
                ctx.stroke()
                ctx.closePath();

                ctx.beginPath();
                ctx.moveTo(xx + cross_off, yy - cross_off);
                ctx.lineTo(xx - cross_off, yy + cross_off);
                ctx.stroke()
                ctx.closePath();
            }
            else if (this.type === "Tick") {
                ctx.lineWidth = 1
                const ll = SCALE / 5;
                ctx.beginPath();
                ctx.moveTo(xx, yy);
                ctx.lineTo(xx - ll, yy - ll);
                ctx.stroke()
                ctx.closePath();

                const rr = SCALE / 2;
                ctx.beginPath();
                ctx.moveTo(xx, yy);
                ctx.lineTo(xx + rr, yy - rr);
                ctx.stroke()
                ctx.closePath();
            }
            //else if (this.type === "Circle") {
            else {
                ctx.beginPath();
                ctx.arc(xx, yy, SCALE, 0, 2 * Math.PI);
                ctx.stroke();
                ctx.closePath();

            }
        }
    }
}
export function create_from_ai_overlay(overlay) {
    let drawables = [];

    for (const overlay_obj of overlay.draw_objs) {
        drawables.push(create_ai_overlay(overlay_obj));
    }

    //console.log(drawables);
    return drawables;

    /*
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
    */
}