export function dan_lerp(x0, x, k) {
    return (x0 * (k-1) + x) / k;
}

export function diff(x, y) {
    return Math.abs(x - y);
}