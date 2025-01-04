use std::{io::Write, mem::MaybeUninit};

use serde::{Deserialize, Serialize};

pub static mut g_settings: MaybeUninit<GlobalSettingsState> = MaybeUninit::uninit();

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum VisualEffectsLevel {
    Full,
    Reduced,
    None,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct GlobalSettingsState {
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub visual_effects: VisualEffectsLevel,
    pub crt_shader: bool,
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
}

impl Default for GlobalSettingsState {
    fn default() -> Self {
        Self {
            sfx_volume: 0.8,
            music_volume: 0.6,
            visual_effects: VisualEffectsLevel::Full,
            crt_shader: true,
            fullscreen: true,
        }
    }
}


impl GlobalSettingsState {
    fn load() -> Self {
        let appdata = std::env::var("APPDATA").unwrap();
        let path = format!("{}\\crunda\\save_state.json", appdata);

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
        state.save();
        state
    }

    fn save(&self) -> std::io::Result<()> {
        let appdata = std::env::var("APPDATA").unwrap();
        let crunda_path = format!("{}\\crunda", appdata);
        let path = format!("{}\\crunda\\save_state.json", appdata);
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

pub fn set(new : GlobalSettingsState) {
    unsafe {
        g_settings = MaybeUninit::new(new);
    }
}

pub fn save() {
    unsafe {
        if let Err(e) = g_settings.assume_init_ref().save() {
            println!("Failed to save settings {}", e);
        }
    }
}