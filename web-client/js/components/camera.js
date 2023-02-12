import { dan_lerp, get_target_y_from_rules_state } from "./utils";

export function create_camera() {

    return {
        x : 0,
        y : 0,
        target_y : 0,
        screen_shake_t : 0,

        screen_shake : function() {
            this.screen_shake_t = 6;
        },

        tick : function(rules_state) {
            if (rules_state) {
                const new_target_y = get_target_y_from_rules_state(rules_state);
                if (new_target_y !== undefined) {
                    this.target_y = new_target_y;
                }
            } 
            this.y = dan_lerp(this.y, this.target_y, 3);
            if (this.screen_shake_t > 0) {
                this.screen_shake_t -= 1;
                this.x = (1 / (this.screen_shake_t + 1)) * Math.random() < 0.5 ? -1 : 1;
            }
            else {
                this.x = 0;
            }
        }
    }
}