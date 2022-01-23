export function dan_lerp(x0, x, k) {
    return (x0 * (k-1) + x) / k;
}

export function ease_in_quad(x) {
    return 1 - (1 - x) * (1 - x);
}

export function diff(x, y) {
    return Math.abs(x - y);
}

// These are so ugly
export function get_target_y_from_rule_state(rule_state) {
    if (rule_state.Round) {
        return rule_state.Round.screen_y;
    }
    else if (rule_state.RoundWarmup) {
        return 0;
    }
    else if (rule_state.RoundCooldown) {
        //return rule_state.RoundCooldown.round_state.screen_y;
    }

    return undefined;
}

export function get_round_id_from_rule_state(rule_state) {
    if (rule_state.Round) {
        return rule_state.Round.round_id;
    }
    else if (rule_state.RoundWarmup) {
        return rule_state.RoundWarmup.round_id;
    }
    else if (rule_state.RoundCooldown) {
        return rule_state.RoundCooldown.round_id;
    }

    return -1;
}
