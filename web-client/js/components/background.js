import { SCALE } from "./constants.js";
import { create_bush} from "./bush.js";

let spr_tree_top = new Image(SCALE, 10);
spr_tree_top.src = '/sprites/spr_tree_top.png';

let spr_bush = new Image(SCALE, SCALE);
spr_bush.src = '/sprites/spr_bush.png';

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

export function create_background_controller() {
    return {
        // Bottom of the screen
        generated_to_y : 160/8,
        in_lobby : true,
        in_warmup : false,
        rows : [],

        reset : function() {
            this.generated_to_y = 160/8;
        },

        tick : function(in_lobby, in_warmup, entities, client) {
            this.in_lobby = in_lobby;
            this.in_warmup = in_warmup;

            this.rows = []
            if (!in_lobby) {
                this.rows = JSON.parse(client.get_rows_json());

                const top_row_y = this.rows[0].y;
                while (top_row_y < this.generated_to_y) {
                    const index = this.generated_to_y - top_row_y;
                    if (index >= this.rows.length) {
                        // Skip creating entities for this row, thats fine as out of view
                    }
                    else {
                        const row = this.rows[index];
                        const y = row.y;
                        if (row.type === "Bushes") {
                            for (let x = 0; x < 20; x++) {
                                //if (Math.random() < 0.15) {
                                    let bush = create_bush(x*SCALE, y*SCALE);
                                    entities.simple_entities.push(bush);
                                    entities.simple_entities.push(bush.foreground);
                                    entities.bushes.push(bush);
                                //}
                            }
                        }
                    }

                    this.generated_to_y -= 1;
                }
            }
            else {
                // @TODO hack to render lobby correctly
                for (let i = 0; i < 160 / 8; i++)
                {
                    this.rows.push({
                        y : i,
                        row_id : i,
                        type: "Lobby",
                    });
                }
            }
        },

        draw : function(froggy_draw_ctx, client) {
            let ctx = froggy_draw_ctx.ctx;
            ctx.fillStyle = "#3c285d";
            ctx.fillRect(0, 0, 160, 160);

            for (const row of this.rows) {
                let y = row.y;

                let col0, col1;

                if (row.type === "River") {
                    col0 = "#6c6ce2";
                    col1 = "#5b5be7";
                }
                else if (row.type === "Road") {
                    col0 = '#646469';
                    col1 = '#59595d';
                }
                else {
                    col0 = "#c4e6b5";
                    col1 = "#d1bfdb";
                }

                for (let i = 0; i < 160 / 8; i++) {
                    let x = i * 8;

                    if ((i + row.row_id) % 2 == 0) {
                        ctx.fillStyle = col0
                    }
                    else {
                        ctx.fillStyle = col1
                    }

                    ctx.fillRect(x, SCALE*y + froggy_draw_ctx.y_off, x + 8, SCALE);
                }

                if (row.type === "Path") {
                    for (let i = 0; i <= row.wall_width; i++) {
                        //draw_static(froggy_draw_ctx, spr_tree_top, i, y);
                        //draw_static_inverted(froggy_draw_ctx, spr_tree_top, i, y);
                        let xx = (i * SCALE) + froggy_draw_ctx.x_off;
                        let yy = y * SCALE + froggy_draw_ctx.y_off - 2;
                        froggy_draw_ctx.ctx.drawImage(spr_tree_top, 0, 0, SCALE, 10, xx, yy, SCALE, 10);
                        xx = 152 - (i * SCALE) + froggy_draw_ctx.x_off;
                        yy = y * SCALE + froggy_draw_ctx.y_off - 2;
                        froggy_draw_ctx.ctx.drawImage(spr_tree_top, 0, 0, SCALE, 10, xx, yy, SCALE, 10);
                    }
                }

                if (row.type === "Stands") {
                    draw_static(froggy_draw_ctx, spr_block, 6, y);
                    draw_static_inverted(froggy_draw_ctx, spr_block, 6, y);
                }

                if (row.type === "StartingBarrier") {
                    for (let i = 0 ; i <= 6; i ++) {
                        draw_static(froggy_draw_ctx, spr_block, i, y);
                        draw_static_inverted(froggy_draw_ctx, spr_block, i, y);
                    }

                    if (this.in_warmup) {
                        for (let i = 7 ; i < 20-7; i ++) {
                            draw_static(froggy_draw_ctx, spr_barrier, i, y);
                        }
                    }
                }
            }
        }
    }
}