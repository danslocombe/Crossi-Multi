import { SCALE} from "./constants.js";

export function draw_background(froggy_draw_ctx, in_lobby, client) {
    let ctx = froggy_draw_ctx.ctx;
    ctx.fillStyle = "#000000";
    ctx.fillRect(0, 0, 256, 256);

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
        }
    }
}