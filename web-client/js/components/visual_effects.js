
export function create_whiteout() {
    const t_fade_max = 6;

    return {
        foreground_depth : 10,
        t_full : 2,
        t_fade : t_fade_max,
        alpha : 1,
        tick : function() {
            if (this.t_full > 0) {
                this.t_full -= 1;
            }
            else {
                this.t_fade -= 1;
            }
        },

        alive: function() {
            return this.t_fade > 0;
        },

        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.fillStyle = "#FFFFFF";
            froggy_draw_ctx.ctx.fillRect(0, 0, 256, 256);
        }
    }
}