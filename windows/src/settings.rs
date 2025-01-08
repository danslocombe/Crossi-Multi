use std::{io::Write, mem::MaybeUninit};

use serde::{Deserialize, Serialize};

use crate::{audio::{g_sfx_volume, g_music_volume}};

pub static mut g_settings: MaybeUninit<GlobalSettingsState> = MaybeUninit::uninit();

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct GlobalSettingsState {
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub screenshake: bool,
    pub flashing: bool,
    pub vibration: bool,
    pub crt: bool,
    pub vignette: bool,
    pub fullscreen: bool,
}

impl GlobalSettingsState {
    pub fn validate(&mut self) {
        self.music_volume = self.music_volume.clamp(0.0, 1.0);
        self.sfx_volume = self.sfx_volume.clamp(0.0, 1.0);
    }

    pub fn sync(&self) {
        // @TODO
    }

    pub fn set_music_volume(&mut self, music_volume: f32) {
        self.music_volume = music_volume;
        self.validate();
        unsafe {
            g_music_volume = self.music_volume;
        }
    }

    pub fn set_sfx_volume(&mut self, sfx_volume: f32) {
        self.sfx_volume = sfx_volume;
        self.validate();
        unsafe {
            g_sfx_volume = self.sfx_volume;
        }

        crate::audio::update_volumes();
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;

        unsafe {
            raylib_sys::ToggleBorderlessWindowed();
        }
    }
}

impl Default for GlobalSettingsState {
    fn default() -> Self {
        Self {
            sfx_volume: 0.8,
            music_volume: 0.6,
            flashing: true,
            screenshake: true,
            vibration: true,
            crt: true,
            vignette: true,
            fullscreen: true,
        }
    }
}


impl GlobalSettingsState {
    fn load() -> Self {
        let appdata = std::env::var("APPDATA").unwrap();
        let path = format!("{}\\crossy\\save_state.json", appdata);

        println!("Loading settings state from {}", path);
        if let Ok(contents) = std::fs::read_to_string(&path) {
            let load_res: Result<Self, serde_json::Error> = serde_json::from_str(&contents);
            if let Ok(mut state) = load_res {
                println!("Read settings state: \n{:#?}", state);
                state.validate();
                return state;
            }
            else {
                println!("Failed to load {:?}", load_res);
            }
        }

        println!("Creating new settings state");
        let state = GlobalSettingsState::default();
        if let Err(e) = state.save() {
            println!("Failed to save state {}", e)
        }
        state
    }

    fn save(&self) -> std::io::Result<()> {
        let appdata = std::env::var("APPDATA").unwrap();
        let crunda_path = format!("{}\\crossy", appdata);
        let path = format!("{}\\crossy\\save_state.json", appdata);
        println!("Saving settings state to {}", path);

        std::fs::create_dir_all(&crunda_path)?;
        let mut file = std::fs::File::create(&path)?;

        let data = serde_json::to_string_pretty(self)?;
        file.write(data.as_bytes())?;

        Ok(())
    }
}

pub fn init() {
    unsafe {
        g_settings = MaybeUninit::new(GlobalSettingsState::load());
    }
}

pub fn get() -> GlobalSettingsState {
    unsafe {
        g_settings.assume_init()
    }
}

pub fn set_save(new : GlobalSettingsState) {
    unsafe {
        g_settings = MaybeUninit::new(new);
    }

    save();
}

pub fn save() {
    unsafe {
        if let Err(e) = g_settings.assume_init_ref().save() {
            println!("Failed to save settings {}", e);
        }
    }
}