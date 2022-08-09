
let spr_font = new Image(16 * 26, 22);
spr_font.src = '/sprites/spr_font_blob.png';

export function create_font_controller() {
    return {
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

                x += 16;

                if (frame_id > 0)
                {
                    ctx.ctx.drawImage(spr_font, 16 * frame_id, 0, 16, 22, x, y, 16, 22);
                }
            }

        }
    };
}