import { SCALE} from "./constants.js";

// TODO don't replicate this constant
export const MOVE_T = 7 * (1000 * 1000 / 60);

function load_sprites(name) {
    let spr = new Image(SCALE, SCALE);
    spr.src = '/sprites/spr_' + name + ".png";
    let spr_dead = new Image(SCALE, SCALE);
    spr_dead.src = '/sprites/spr_' + name + "_dead.png";

    return {
        spr : spr,
        spr_dead : spr_dead,
        spr_name : name,
    }
}

export const spr_shadow = new Image(SCALE, SCALE);
spr_shadow.src = '/sprites/spr_shadow.png';

export const sprites_list = [
    load_sprites('frog'),
    load_sprites('duck'),
    load_sprites('mouse'),
    load_sprites('bird'),
    load_sprites('snake'),
    load_sprites('frog_alt'),
    load_sprites('frog_3'),
]

export const colours_list = [
    "#4aef5c",
    "#d9a066",
    "#884835",
    "#fb3c3c",
    "#80ffff",
    "#819ecf",
    "#cab56a",
]

export const move_sounds_list = [
    new Audio('/sounds/snd_move1.wav'),
    new Audio('/sounds/snd_move2.wav'),
    new Audio('/sounds/snd_move3.wav'),
    new Audio('/sounds/snd_move4.wav'),
    new Audio('/sounds/snd_move_alt.wav'),
    new Audio('/sounds/snd_move1.wav'),
]

for (let sound of move_sounds_list) {
    sound.volume = 0.15;
}

export const names = [
    "frog",
    "duck",
    "mouse",
    "bird",
    "snake",
    "yellow frog",
    "blue frog",
]