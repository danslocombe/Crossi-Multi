export function dan_lerp(x0, x, k) {
    return (x0 * (k-1) + x) / k;
}

export function ease_in_quad(x) {
    return 1 - (1 - x) * (1 - x);
}

export function diff(x, y) {
    return Math.abs(x - y);
}

export function get_target_y_from_rules_state(rules_state) {
    let target_y = rules_state.fst.screen_y;
    if (target_y == null)
    {
        target_y = 0;
    }

    return target_y;
}

export function get_round_id_from_rules_state(rules_state) {
    let round = rules_state.fst.round_id;
    if (round == null)
    {
        round = -1;
    }

    return round;
}
