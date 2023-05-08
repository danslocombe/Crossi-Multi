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

                const top_row_y = this.rows[0][0];
                while (top_row_y < this.generated_to_y) {
                    const index = this.generated_to_y - top_row_y;
                    if (index >= this.rows.length) {
                        // Skip creating entities for this row, thats fine as out of view
                    }
                    else {
                        const row = this.rows[index];
                        const y = row[0];
                        if (row[1].row_type.Bushes) {
                            for (let x = 0; x < 20; x++) {
                                //if (Math.random() < 0.15) {
                                    let bush = create_bush(x*SCALE, y*SCALE, "foliage");
                                    entities.simple_entities.push(bush);
                                    entities.bushes.push(bush);
                                //}
                            }
                        }
                    }

                    this.generated_to_y -= 1;
                }
            }
            else {
                for (let i = 0; i < 160 / 8; i++)
                {
                    this.rows.push([i, {row_type: {}, row_id: (i)}]);
                }
            }
        },

        draw : function(froggy_draw_ctx, client) {
            let ctx = froggy_draw_ctx.ctx;
            ctx.fillStyle = "#3c285d";
            ctx.fillRect(0, 0, 160, 160);

            for (const row of this.rows) {
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
                    for (let i = 0; i <= wall_width; i++) {
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

                if (row[1].row_type.Bushes) {
                    /*
                    const wall_width = row[1].row_type.Bushes.path_descr.wall_width;
                    for (let i = 0; i <= wall_width; i++) {
                        let xx = (i * SCALE) + froggy_draw_ctx.x_off;
                        let yy = y * SCALE + froggy_draw_ctx.y_off - 2;
                        froggy_draw_ctx.ctx.drawImage(spr_tree_top, 0, 0, SCALE, 10, xx, yy, SCALE, 10);
                        xx = 152 - (i * SCALE) + froggy_draw_ctx.x_off;
                        yy = y * SCALE + froggy_draw_ctx.y_off - 2;
                        froggy_draw_ctx.ctx.drawImage(spr_tree_top, 0, 0, SCALE, 10, xx, yy, SCALE, 10);
                    }

                    let bushes_json = client.get_bushes_row_json(row[0]);
                    const bushes = JSON.parse(bushes_json);

                    for (let x of bushes.bushes) {
                        draw_static(froggy_draw_ctx, spr_bush, x, y);
                    }
                    */
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