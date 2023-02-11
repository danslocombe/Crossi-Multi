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
export function get_target_y_from_rules_state(rules_state) {
    if (rules_state.fst.Round) {
        return rules_state.fst.Round.screen_y;
    }
    else if (rules_state.fst.RoundWarmup) {
        return 0;
    }
    else if (rules_state.fst.RoundCooldown) {
        //return rules_state.fst.RoundCooldown.round_state.screen_y;
    }

    return undefined;
}

export function get_round_id_from_rules_state(rules_state) {
    if (rules_state.fst.Round) {
        return rules_state.fst.Round.round_id;
    }
    else if (rules_state.fst.RoundWarmup) {
        return rules_state.fst.RoundWarmup.round_id;
    }
    else if (rules_state.fst.RoundCooldown) {
        return rules_state.fst.RoundCooldown.round_id;
    }
    else if (rules_state.fst.EndWinner || rules_state.fst.EndAllLeft)
    {
        return -2;
    }

    return -1;
}
