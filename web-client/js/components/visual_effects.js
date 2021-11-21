import { SCALE} from "./constants.js";

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

const spr_dust = new Image(SCALE,SCALE);
spr_dust.src = "/sprites/spr_dust.png";

const spr_smoke_count = 4;

export function create_dust(x, y) {
    return {
        frame_id : Math.floor(Math.random() * spr_smoke_count),
        scale : 0.5 + Math.random() * 0.6,
        dynamic_depth : y-32,
        x : x,
        y : y,
        tick : function() {
            this.scale -= 0.025;
        },

        alive: function() {
            return this.scale > 0
        },

        draw : function(froggy_draw_ctx) {
            //const x = this.x + 0 + (1-this.scale)*4 + froggy_draw_ctx.x_off;
            //const y = this.y + 0 + (1-this.scale)*4 + froggy_draw_ctx.y_off;
            const x = SCALE*(this.x + 0.25) + (1-this.scale) + froggy_draw_ctx.x_off;
            const y = SCALE*(this.y + 0.25) + (1-this.scale) + froggy_draw_ctx.y_off;
            froggy_draw_ctx.ctx.drawImage(spr_dust, SCALE*this.frame_id, 0, SCALE, SCALE, x, y, SCALE*this.scale, SCALE*this.scale);
        }
    };
}

const spr_bubble = new Image(SCALE,SCALE);
spr_bubble.src = "/sprites/spr_bubble.png";
const spr_bubble_count = 5

export function create_bubble(x, y) {
    return {
        frame_id : Math.floor(Math.random() * spr_bubble_count),
        scale : 0.5 + Math.random() * 0.6,
        static_depth : 100,
        x : x,
        y : y,
        tick : function() {
            this.scale -= 0.025;
        },

        alive: function() {
            return this.scale > 0
        },

        draw : function(froggy_draw_ctx) {
            this.y -= 0.05;
            const x = SCALE*(this.x + 0.25) + (1-this.scale) + froggy_draw_ctx.x_off;
            const y = SCALE*(this.y + 0.25) + (1-this.scale) + froggy_draw_ctx.y_off;
            froggy_draw_ctx.ctx.drawImage(spr_bubble, SCALE*this.frame_id, 0, SCALE, SCALE, x, y, SCALE*this.scale, SCALE*this.scale);
        }
    }
}

export function create_corpse(x, y, spr_dead) {
    return {
        spr : spr_dead,
        x : x,
        y : y,
        dynamic_depth : y,

        tick : () => {},
        alive : () => true,
        draw : function(froggy_draw_ctx) {
            froggy_draw_ctx.ctx.drawImage(
                this.spr,
                0,
                0,
                SCALE,
                SCALE,
                x + froggy_draw_ctx.x_off,
                y + froggy_draw_ctx.y_off,
                SCALE,
                SCALE);
        }
    }
}