import { SCALE} from "./constants.js";

export function create_graph(x, y, graph_data) {
    return {
        x : x,
        y : y,
        width : 32,
        height : 32,
        graph_data : graph_data,
        depth : 20000,
        colour : "green",
        draw : function(froggy_draw_ctx) {

            const ctx = froggy_draw_ctx.ctx;
            ctx.strokeStyle = this.colour;
            ctx.beginPath();
            for (let i in this.graph_data.data) {
                const value = this.graph_data.data[i];

                const x = this.x + 0.1 * i; 
                const y = this.y + (1 - value) * this.height;

                if (i == 0)
                {
                    ctx.moveTo(x, y);
                }
                else
                {
                    ctx.lineTo(x, y);
                }

            }
            ctx.stroke()
            ctx.closePath();

            ctx.strokeStyle = "black";
            ctx.strokeText(this.graph_data.y_min.toString(), this.x - 8, this.y + this.height);
            ctx.strokeText(this.graph_data.y_max.toString(), this.x - 8, this.y);
        }
    }
}