use std::{collections::BTreeMap, mem::{MaybeUninit}};

static mut g_muted: bool = false;
static mut g_sfx_volume: f32 = 0.8;
static mut g_music_volume: f32 = 0.8;

static mut SFX: MaybeUninit<BTreeMap<String, Sound>> = MaybeUninit::uninit();

pub fn init_audio() {
    unsafe {
        let map = BTreeMap::new();
        SFX = MaybeUninit::new(map);

        load_sfx("snd_join.wav", 1.0);
    }
}


fn load_sfx(filename: &str, base_volume: f32) {
    let path_base = "../web-client/static/sounds";
    let path = format!("{}/{}", path_base, filename);
    println!("Loading {}", path);

    unsafe {
        let filename_c = crate::c_str_temp(&path);
        let sound = raylib_sys::LoadSound(filename_c);

        //raylib_sys::SetSoundVolume(sound, base_volume * g_sfx_volume);

        let path = std::path::Path::new(&path);
        let mut name = path.file_stem().unwrap().to_str().unwrap();
        if name.starts_with("snd_") {
            name = &name["snd_".len()..];
        }

        SFX.assume_init_mut().insert(name.to_owned(), Sound {
            sound,
            base_volume,
        });
    }
}

pub fn play(name: &str) {
    unsafe {
        if (g_muted) {
            return;
        }

        if let Some(sound) = SFX.assume_init_ref().get(name) {
            raylib_sys::PlaySound(sound.sound);
        }
        else {
            crate::console::err(&format!("Could not find SFX {}", name));
        }
    }
}

struct Sound {
    sound: raylib_sys::Sound,
    base_volume: f32,
}