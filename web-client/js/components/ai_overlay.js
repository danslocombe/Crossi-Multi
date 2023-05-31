import { SCALE} from "./constants.js";

function create_ai_overlay(overlay_obj) {
    return {
        x : overlay_obj.pos.x,
        y : overlay_obj.pos.y,
        dynamic_depth : overlay_obj.pos.y + 20000,
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
            else if (this.type.Line) {
                ctx.lineWidth = 1
                ctx.beginPath();
                ctx.moveTo(xx, yy);
                const x1 = (this.type.Line.x + 0.5) * SCALE + froggy_draw_ctx.x_off;
                const y1 = (this.type.Line.y + 0.5) * SCALE + froggy_draw_ctx.y_off;
                //console.log("from " + xx + " " + yy + " | to " + x1 + " " + y1);
                ctx.lineTo(x1, y1);
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

    return drawables;
}