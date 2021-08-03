import { SCALE} from "./constants.js";

let spr_tree_top = new Image(SCALE, SCALE);
spr_tree_top.src = '/sprites/spr_tree_top.png';

let spr_block = new Image(SCALE, SCALE);
spr_block.src = '/sprites/spr_block.png';

let spr_barrier = new Image(SCALE, SCALE);
spr_barrier.src = '/sprites/spr_barrier.png';

function draw_static(froggy_draw_ctx, spr, x, y) {
    const xx = x * SCALE + froggy_draw_ctx.x_off;
    const yy = y * SCALE + froggy_draw_ctx.y_off;
    froggy_draw_ctx.ctx.drawImage(spr, 0, 0, SCALE, SCALE, xx, yy, SCALE, SCALE);
}

function draw_static_inverted(froggy_draw_ctx, spr, x, y) {
    const xx = 152 - (x * SCALE) + froggy_draw_ctx.x_off;
    const yy = y * SCALE + froggy_draw_ctx.y_off;
    froggy_draw_ctx.ctx.drawImage(spr, 0, 0, SCALE, SCALE, xx, yy, SCALE, SCALE);
}

export function draw_background(froggy_draw_ctx, in_lobby, in_warmup, client) {
    let ctx = froggy_draw_ctx.ctx;
    //ctx.fillStyle = "#000000";
    ctx.fillStyle = "#EEFFAA";
    ctx.fillRect(0, 0, 160, 160);

    if (!in_lobby) {
        const rows = JSON.parse(client.get_rows_json());
        for (const row of rows) {
            let y = row[0];

            let col0, col1;

            if (row[1].row_type.River) {
                col0 = "#6c6ce2";
                col1 = "#5b5be7";
            }
            else if (row[1].row_type.Road) {
                col0 = '#646469';
                col1 = '#59595d';
            }
            else {
                col0 = "#c4e6b5";
                col1 = "#d1bfdb";
            }

            for (let i = 0; i < 160 / 8; i++) {
                let x = i * 8;

                if ((i + row[1].row_id) % 2 == 0) {
                    ctx.fillStyle = col0
                }
                else {
                    ctx.fillStyle = col1
                }

                ctx.fillRect(x, SCALE*y + froggy_draw_ctx.y_off, x + 8, SCALE);
            }

            if (row[1].row_type.Path) {
                const wall_width = row[1].row_type.Path.wall_width;
                for (let i = 0; i < wall_width; i++) {
                    draw_static(froggy_draw_ctx, spr_tree_top, i, y);
                    draw_static_inverted(froggy_draw_ctx, spr_tree_top, i, y);
                }
            }

            if (row[1].row_type.Stands) {
                draw_static(froggy_draw_ctx, spr_block, 6, y);
                draw_static_inverted(froggy_draw_ctx, spr_block, 6, y);
            }

            if (row[1].row_type.StartingBarrier) {
                for (let i = 0 ; i <= 6; i ++) {
                    draw_static(froggy_draw_ctx, spr_block, i, y);
                    draw_static_inverted(froggy_draw_ctx, spr_block, i, y);
                }

                if (in_warmup) {
                    for (let i = 7 ; i < 20-7; i ++) {
                        draw_static(froggy_draw_ctx, spr_barrier, i, y);
                    }
                }
            }
        }
    }
}