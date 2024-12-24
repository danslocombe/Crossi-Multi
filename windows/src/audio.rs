use std::{collections::BTreeMap, mem::{MaybeUninit}};

static mut g_muted: bool = false;
static mut g_sfx_volume: f32 = 0.2;
static mut g_music_volume: f32 = 0.3;

static mut SFX: MaybeUninit<BTreeMap<String, Sound>> = MaybeUninit::uninit();

pub fn init_audio() {
    unsafe {
        let map = BTreeMap::new();
        SFX = MaybeUninit::new(map);

        load_sfx("snd_join.wav", 1.0);
        load_sfx("snd_car.wav", 1.0);
        load_sfx("snd_countdown.wav", 1.0);
        load_sfx("snd_countdown_go.wav", 1.0);
        load_sfx("snd_drown.wav", 1.0);
        load_sfx("snd_drown_bubbles.wav", 1.0);

        load_sfx("snd_move_alt.wav", 1.0);
        load_sfx("snd_move1.wav", 1.0);
        load_sfx("snd_move2.wav", 1.0);
        load_sfx("snd_move3.wav", 1.0);
        load_sfx("snd_move4.wav", 1.0);

        load_sfx("snd_push.wav", 1.0);

        load_sfx("snd_frog_win.wav", 1.0);
        load_sfx("snd_frog_win_2.wav", 1.0);
        load_sfx("snd_bird_win.wav", 1.0);
        load_sfx("snd_mouse_win.wav", 1.0);
        load_sfx("snd_mouse_win_2.wav", 1.0);
        load_sfx("snd_win.wav", 1.0);
        load_sfx("snd_viper.mp3", 1.0);
    }
}


fn load_sfx(filename: &str, base_volume: f32) {
    let path_base = "../web-client/static/sounds";
    let path = format!("{}/{}", path_base, filename);
    println!("Loading {}", path);

    unsafe {
        let filename_c = crate::c_str_temp(&path);
        let sound = raylib_sys::LoadSound(filename_c);

        raylib_sys::SetSoundVolume(sound, base_volume * g_sfx_volume);

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

pub fn ensure_playing_with_volume(name: &str, volume_mult: f32) {
    unsafe {
        if (g_muted) {
            return;
        }

        if let Some(sound) = SFX.assume_init_ref().get(name) {
            raylib_sys::SetSoundVolume(sound.sound, sound.base_volume * g_sfx_volume * volume_mult);
            if (!raylib_sys::IsSoundPlaying(sound.sound)) {
                raylib_sys::PlaySound(sound.sound);
            }
        }
        else {
            crate::console::err(&format!("Could not find SFX {}", name));
        }
    }
}

pub fn stop(name: &str) {
    unsafe {
        if let Some(sound) = SFX.assume_init_ref().get(name) {
            raylib_sys::StopSound(sound.sound);
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