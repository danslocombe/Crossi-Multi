import { ease_in_quad } from "./utils"

let spr_countdown = new Image(48, 32);
spr_countdown.src = '/sprites/spr_countdown.png';

let spr_winner = new Image(113, 32);
spr_winner.src = '/sprites/spr_winner.png';

let spr_no_winner = new Image(113, 64);
spr_no_winner.src = '/sprites/spr_no_winner.png';

let snd_countdown = new Audio('/sounds/snd_countdown.wav');
snd_countdown.volume = 0.25;
let snd_countdown_go = new Audio('/sounds/snd_countdown_go.wav');
snd_countdown_go.volume = 0.25;

export function create_countdown() {
    return {
        enabled : false,
        time : 0,
        go_time : 0,

        tick : function (rule_state) {
            if (!rule_state) {
                return;
            }

            if (rule_state.RoundWarmup) {
                const time = Math.ceil(rule_state.RoundWarmup.remaining_us / 1000000);
                //console.log(rule_state.RoundWarmup);
                if (time != this.time) {
                    snd_countdown.play();
                }
                this.time = time;
                this.enabled = true;
                this.go_time = 60;
            }
            else if (rule_state.Round) {
                if (this.go_time > 0) {
                    if (this.time == 1) {
                        // First tick of "go"
                        snd_countdown_go.play();
                        this.time = 0;
                    }
                    this.go_time -= 1;
                    this.enabled = true;
                }
                else {
                    this.enabled = false;
                }
            }
            else {
                this.enabled = false;
            }
        },

        draw : function(crossy_draw_ctx) {
            if (this.enabled) {
                const frame_id = 3 - this.time;
                const x = 80 - 24;
                const y = 80 - 16;
                crossy_draw_ctx.ctx.drawImage(spr_countdown, 48*frame_id, 0, 48, 32, x, y, 48, 32);
            }
        }
    }
}

export function create_winner_ui() {
    return {
        foreground_depth : 5,
        is_alive : true,
        fade_in_time : 16,
        fade_out_time : 24,
        target_scale : 1,
        scale : 0,
        scale_factor : 0,
        t : 0,
        t_end : 180,
        spr : spr_winner,
        no_winner : false,

        alive : function(x) {
            return this.is_alive;
        },

        trigger_no_winner : function() {
            this.spr = spr_no_winner;
            this.no_winner = true;
            //this.is_alive = false;
        },

        tick : function (rule_state) {
            this.t += 1;

            if (this.t < this.fade_in_time) {
                // Easing in
                this.scale_factor = ease_in_quad(this.t / this.fade_in_time);
            }
            else {
                this.scale_factor = 1;
            }

            this.scale = this.scale_factor * this.target_scale;
        },

        draw : function(crossy_draw_ctx) {

            const w = this.spr.width;
            const h = this.spr.height;
            const interval = 50;
            const spin_interval = -105;

            let w_draw = w;
            let h_draw = h;

            if (!this.no_winner) {
                w_draw = w * this.scale * (1 + 0.12 * Math.sin(this.t / interval));
                h_draw = h * this.scale * (1 + 0.12 * Math.sin(this.t / interval));
            }

            let ctx = crossy_draw_ctx.ctx;
            ctx.save();
            const x = 80;
            const y = 80;
            ctx.translate(x, y);

            if (!this.no_winner) {
                ctx.rotate(0.3 * Math.sin(this.t / spin_interval));
            }

            ctx.drawImage(
                this.spr,
                0,
                0,
                w,
                h,
                -w_draw / 2,
                -h_draw / 2,
                w_draw,
                h_draw);

            ctx.restore();
        }
    }
}