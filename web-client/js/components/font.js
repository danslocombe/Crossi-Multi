
//let spr_font = new Image(16 * 26, 22);
//spr_font.src = '/sprites/spr_font_blob.png';

let font_width = 10;
let font_height = 12;
let spr_font = new Image(font_width * 26, font_height);
spr_font.src = '/sprites/spr_font_small.png';

let spr_font_2 = new Image(font_width * 26, font_height);
spr_font_2.src = '/sprites/spr_font_small_2.png';

export function create_font_controller() {
    return {
        t : 0,
        text_height : font_height,
        tick : function() {
            this.t += 1;
        },
        text : function(ctx, str, x, y) {
            for (let i = 0; i < str.length; i++)
            {
                let frame_id = -1;
                let char_code = str.charCodeAt(i);
                if (char_code >= 65 && char_code <= 90)
                {
                    // Upper case
                    frame_id = char_code - 65;
                }
                else if (char_code >= 97 && char_code <= 122)
                {
                    // Lower case
                    frame_id = char_code - 97;
                }
                else {
                    // Invalid char
                }

                if (frame_id >= 0)
                {
                    const x_off = 0;
                    const y_off = 0;
                    let sprite = spr_font;
                    if ((Math.floor(this.t / 8) % 2) == 0) {
                        sprite = spr_font_2;
                    }
                    ctx.ctx.drawImage(sprite, font_width * frame_id, 0, font_width, font_height, x + x_off, y + y_off, font_width, font_height);
                }

                x += font_width;
            }

        }
    };
}