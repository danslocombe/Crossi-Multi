let spr_countdown = new Image(48, 32);
spr_countdown.src = '/sprites/spr_countdown.png';

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