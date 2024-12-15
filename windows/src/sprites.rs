use core::str;
use std::{collections::BTreeMap, mem::MaybeUninit};


static mut SPRITE_FRAMES: MaybeUninit<BTreeMap<String, Vec<raylib_sys::Texture2D>>> = MaybeUninit::uninit();

pub fn init_sprites() {
    unsafe {
        let map = BTreeMap::new();
        SPRITE_FRAMES = MaybeUninit::new(map);
        load_frames("../web-client/static/sprites/spr_frog.png", None);
        load_frames("../web-client/static/sprites/spr_shadow.png", None);
        load_frames("../web-client/static/sprites/spr_block.png", None);
        load_frames("../web-client/static/sprites/spr_barrier.png", None);
    }
}

fn load_frames(filename: &str, p_frame_count: Option<usize>) {
    let frames = unsafe { load_frames_unsafe(filename, p_frame_count) };

    let path = std::path::Path::new(filename);
    let mut name = path.file_stem().unwrap().to_str().unwrap();
    if name.starts_with("spr_") {
        name = &name["spr_".len()..];
    }
    unsafe {
        SPRITE_FRAMES.assume_init_mut().insert(name.to_owned(), frames);
    }
}

unsafe fn load_frames_unsafe(filename: &str, p_frame_count: Option<usize>) -> Vec<raylib_sys::Texture2D> {
    let image = raylib_sys::LoadImage(crate::c_str_temp(filename));

    let mut frame_count: usize = 0;
    if (p_frame_count == None) {
        frame_count = (image.width / image.height) as usize;
    }
    else {
        frame_count = p_frame_count.unwrap();
    }

    let frame_w = image.width / (frame_count as i32);

    let mut frames = Vec::new();
    for iu in 0..frame_count {
        let i = iu as i32;
        let xoff: f32 = (i * frame_w) as f32;
        let frame_image = raylib_sys::ImageFromImage(
            image,
            raylib_sys::Rectangle {
                x: xoff,
                y: 0.0,
                width: frame_w as f32,
                height: image.height as f32 });

        frames.push(raylib_sys::LoadTextureFromImage(frame_image));

        raylib_sys::UnloadImage(frame_image);
    }

    raylib_sys::UnloadImage(image);
    frames
}

pub fn get_sprite(name: &str) -> &[raylib_sys::Texture2D] {
    unsafe { 
        let frames = SPRITE_FRAMES.assume_init_ref();
        let frame_vec = frames.get(name).unwrap_or_else(|| frames.get("unknown").unwrap());
        &frame_vec[..]
    }
}

pub fn draw(name: &str, image_index: usize, x: f32, y: f32) {
    let spr = &get_sprite(name)[image_index];
    unsafe {
        raylib_sys::DrawTexture(
            *spr,
            x as i32,
            y as i32,
            crate::WHITE);
    }
}