

let font_width = 10;
let font_height = 12;
let spr_font = new Image(font_width * 26, font_height);
spr_font.src = '/sprites/spr_font_small.png';

let spr_font_2 = new Image(font_width * 26, font_height);
spr_font_2.src = '/sprites/spr_font_small_2.png';

let small_font = {
    width: 10,
    height: 12,
    sprite_0 : spr_font,
    sprite_1 : spr_font_2,
}

let spr_font_blob = new Image(16 * 22, 22);
spr_font_blob.src = '/sprites/spr_font_blob.png';

let blob_font = {
    width: 16,
    height: 22,
    sprite_0 : spr_font_blob,
    sprite_1 : spr_font_blob,
}

export function create_font_controller() {
    return {
        t : 0,
        font : small_font,

        set_Font_small : function() {
            this.font = small_font;
        },

        set_font_blob: function() {
            this.font = blob_font;
        },

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
                    let sprite = this.font.sprite_0;
                    if ((Math.floor(this.t / 8) % 2) == 0) {
                        sprite = this.font.sprite_1;
                    }
                    ctx.ctx.drawImage(
                        sprite,
                        this.font.width * frame_id, 0,
                        this.font.width,
                        this.font.height,
                        x + x_off,
                        y + y_off,
                        this.font.width,
                        this.font.height);
                }

                x += this.font.width;
            }

        }
    };
}