use std::mem::MaybeUninit;

use crossy_multi_core::math::V2;
use crate::{audio::{self, g_music_volume}, c_str_leaky, c_str_temp, client::{river_col_1, VisualEffects}, gamepad_pressed, key_pressed, lerp_color_rgba, to_vector2, WHITE};

//static mut g_font_roboto: MaybeUninit<raylib_sys::Font> = MaybeUninit::uninit();
static mut g_font_roboto: [(i32, MaybeUninit<raylib_sys::Font>); 7] = [
    (12, MaybeUninit::uninit()),
    (18, MaybeUninit::uninit()),
    (24, MaybeUninit::uninit()),
    (36, MaybeUninit::uninit()),
    (48, MaybeUninit::uninit()),
    (60, MaybeUninit::uninit()),
    (72, MaybeUninit::uninit()),
];

pub fn init_pause_fonts() {
    unsafe {
        //g_font_roboto = MaybeUninit::new(raylib_sys::LoadFont(c_str_leaky("../web-client/static/Roboto-Regular.ttf")));
        for (size, data) in g_font_roboto.iter_mut() {
            *data = MaybeUninit::new(raylib_sys::LoadFontEx(
                c_str_leaky("../web-client/static/Roboto-Bold.ttf"),
                *size,
                std::ptr::null_mut(), // Default characters
                95, // Default character count in raylib, just ascii
                ));
        }
    }
}

#[derive(Default)]
pub struct Pause {
    pub t: i32,
    pub t_since_move: i32,
    pub highlighted: i32,

    pub settings_menu: Option<SettingsMenu>,
}

pub enum PauseResult {
    Nothing,
    Unpause,
    Exit,
    Lobby,
    Feedback,
}

impl Pause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, visual_effects: &mut VisualEffects) -> PauseResult {
        visual_effects.noise = visual_effects.noise.max(0.8);

        if let Some(settings) = self.settings_menu.as_mut() {
            if !settings.tick(visual_effects) {
                self.settings_menu = None;
            }

            return PauseResult::Nothing;
        }

        let input = MenuInput::read();

        self.t += 1;
        self.t_since_move += 1;

        // @Fragile
        let option_count = 5;

        // @TODO controller input / WASD.
        if let MenuInput::Down = input {
            self.highlighted = (self.highlighted + 1) % option_count;
            self.t_since_move = 0;
            audio::play("menu_move");
            //visual_effects.noise = visual_effects.noise.max(5.0);
            //visual_effects.noise();
        }
        if let MenuInput::Up = input {
            self.highlighted = (self.highlighted - 1);
            if (self.highlighted < 0) {
                self.highlighted = option_count - 1;
            }

            self.t_since_move = 0;
            audio::play("menu_move");
            //visual_effects.noise = visual_effects.noise.max(5.0);
            //visual_effects.noise();
        }

        // @TODO controller input / WASD.
        if let MenuInput::Enter = input {
            visual_effects.noise = visual_effects.noise.max(5.0);
            audio::play("menu_click");
            match self.highlighted {
                0 => {
                    // Resume
                    return PauseResult::Unpause;
                }
                1 => {
                    // Lobby
                    return PauseResult::Lobby;
                }
                2 => {
                    // Settings
                    self.settings_menu = Some(SettingsMenu::new());
                }
                3 => {
                    // Feedback
                    return PauseResult::Feedback;
                }
                4 => {
                    // Exit
                    return PauseResult::Exit;
                }
                _ => {
                    // @Unreachable
                    debug_assert!(false);
                    return PauseResult::Unpause;
                }
            }
        }

        PauseResult::Nothing
    }

    pub fn draw(&self) {
        unsafe {
            //let mut col = crate::SEA;
            let mut col = river_col_1;
            col.a = 240;
            //col.a = 80;
            raylib_sys::DrawRectangle(0, 0, 160, 160, col);
        }
    }

    pub fn draw_gui(&self) {
        if let Some(settings) = self.settings_menu.as_ref() {
            settings.draw();
            return;
        }

        let padding = 16.0;

        let mut draw_info = PauseDrawInfo::create(self.t_since_move);
        let text_size = draw_info.title("Paused");

        draw_info.pos = V2::new(draw_info.dimensions.x * 0.5, draw_info.dimensions.y * 0.4);
        draw_info.pos.y += text_size.y + padding;

        draw_info.text_center_incr_padding("Resume", padding, self.highlighted == 0);
        draw_info.text_center_incr_padding("Lobby", padding, self.highlighted == 1);
        draw_info.text_center_incr_padding("Settings", padding, self.highlighted == 2);
        draw_info.text_center_incr_padding("Submit Feedback", padding, self.highlighted == 3);
        draw_info.text_center_incr_padding("Exit", padding, self.highlighted == 4);
    }
}

#[derive(Default)]
pub struct SettingsMenu {
    pub t: i32,
    pub t_since_move: i32,
    pub highlighted: i32,
}

impl SettingsMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, visual_effects: &mut VisualEffects) -> bool {
        self.t += 1;
        self.t_since_move += 1;

        let input = MenuInput::read();

        // @Fragile
        let option_count = 8;

        // @TODO controller input / WASD.
        // @Dedup
        if let MenuInput::Down = input {
            self.highlighted = (self.highlighted + 1) % option_count;
            self.t_since_move = 0;
            audio::play("menu_move");
        }
        if let MenuInput::Up = input {
            self.highlighted = (self.highlighted - 1);
            if (self.highlighted < 0) {
                self.highlighted = option_count - 1;
            }

            self.t_since_move = 0;
            audio::play("menu_move");
        }

        match self.highlighted {
            0 => {
                if let MenuInput::Left = input {
                    let mut state = crate::settings::get();
                    state.set_music_volume(state.music_volume - 0.1);
                    crate::settings::set_save(state);
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.set_music_volume(state.music_volume + 0.1);
                    crate::settings::set_save(state);
                }
            }
            1 => {
                if let MenuInput::Left = input {
                    let mut state = crate::settings::get();
                    state.set_sfx_volume(state.sfx_volume - 0.1);
                    audio::play("menu_click");
                    crate::settings::set_save(state);
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.set_sfx_volume(state.sfx_volume + 0.1);
                    audio::play("menu_click");
                    crate::settings::set_save(state);
                }
            }
            2 => {
                // Window mode
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.toggle_fullscreen();
                    crate::settings::set_save(state);
                }
            }
            3 => {
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.screenshake = !state.screenshake;
                    if (state.screenshake) {
                        visual_effects.screenshake = visual_effects.screenshake.max(15.0);
                    }
                    crate::settings::set_save(state)
                }
            }
            4 => {
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.vibration = !state.vibration;
                    if (state.vibration) {
                        for i in 0..4 {
                            visual_effects.set_gamepad_vibration(i);
                        }
                    }
                    crate::settings::set_save(state)
                }
            }
            5 => {
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.flashing = !state.flashing;
                    if (state.flashing) {
                        visual_effects.screenshake();
                        visual_effects.whiteout();
                    }
                    crate::settings::set_save(state)
                }
            }
            6 => {
                // CRT
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.crt_shader = !state.crt_shader;
                    crate::settings::set_save(state)
                }
            },
            7 => {
                if let MenuInput::Enter = input {
                    audio::play("menu_click");
                    return false;
                }
            },
            _ => {
                // @Unreachable
                debug_assert!(false);
            }
        }

        true
    }

    pub fn draw(&self) {
        let padding = 16.0;

        let mut draw_info = PauseDrawInfo::create(self.t_since_move);
        let text_size = draw_info.title("Paused");

        draw_info.pos = V2::new(draw_info.dimensions.x * 0.5, draw_info.dimensions.y * 0.4);
        draw_info.pos.y += text_size.y + padding;

        let settings = crate::settings::get();

        let music_percentage = format!("{}%", (settings.music_volume * 100.0).round());
        draw_info.text_left_right_incr_padding("Music Volume:", &music_percentage, padding, self.highlighted == 0);

        let sfx_percentage = format!("{}%", (settings.sfx_volume * 100.0).round());
        draw_info.text_left_right_incr_padding("Sound Effects Volume:", &sfx_percentage, padding, self.highlighted == 1);

        let window_mode = if settings.fullscreen { "Fullscreen" } else { "Windowed" };
        draw_info.text_left_right_incr_padding("Window Mode:", &window_mode, padding, self.highlighted == 2);

        let screenshake_mode = if settings.screenshake { "On" } else { "Off" };
        draw_info.text_left_right_incr_padding("Screenshake:", screenshake_mode, padding, self.highlighted == 3);
        let vibration_mode = if settings.vibration { "On" } else { "Off" };
        draw_info.text_left_right_incr_padding("Vibration:", vibration_mode, padding, self.highlighted == 4);
        let flashing_mode = if settings.flashing { "On" } else { "Off" };
        draw_info.text_left_right_incr_padding("Flashing:", flashing_mode, padding, self.highlighted == 5);
        let crt_mode = if settings.crt_shader { "On" } else { "Off" };
        draw_info.text_left_right_incr_padding("CRT Effect:", crt_mode, padding, self.highlighted == 6);
        draw_info.text_center("Back", self.highlighted == 7);
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuInput {
    None,
    Up,
    Down,
    Left,
    Right,
    Enter,
}

impl MenuInput {
    pub fn is_toggle(self) -> bool {
        match self {
            MenuInput::Left | MenuInput::Right | MenuInput::Enter => true,
            _ => false
        }
    }

    pub fn read() -> Self {
        let mut input = MenuInput::None;

        if key_pressed(raylib_sys::KeyboardKey::KEY_UP) {
            input = MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_LEFT) {
            input = MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_DOWN) {
            input = MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT) {
            input = MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_SPACE) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_ENTER) {
            input = MenuInput::Enter;
        }

        if key_pressed(raylib_sys::KeyboardKey::KEY_W) {
            input = MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_A) {
            input = MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_S) {
            input = MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_D) {
            input = MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_Z) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_X) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_F) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_G) {
            input = MenuInput::Enter;
        }

        for i in 0..4 {
            let gamepad_id = i as i32;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                input = MenuInput::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                input = MenuInput::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                input = MenuInput::Down;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                input = MenuInput::Right;
            }

            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                input = MenuInput::Enter;
            }
        }

        input
    }
}

#[derive(Debug, Clone)]
struct PauseDrawInfo {
    dimensions: V2,
    title_font: (raylib_sys::Font, f32),
    main_font: (raylib_sys::Font, f32),
    t_for_fade: i32,
    pos: V2,

    left: f32,
    right: f32,
}

impl PauseDrawInfo {
    pub fn create(t_for_fade: i32) -> Self {
        unsafe {
            let width = raylib_sys::GetScreenWidth();
            let height = raylib_sys::GetScreenHeight();
            let dimensions = V2::new(width as f32, height as f32);

            let index_base = if height < 500 {
                0
            }
            else if width < 800 {
                1
            }
            else if width < 1200 {
                1
            }
            else if width < 1500 {
                2
            }
            else {
                3
            };

            let title_font = (g_font_roboto[index_base + 1].1.assume_init(), g_font_roboto[index_base + 1].0 as f32);
            let main_font = (g_font_roboto[index_base].1.assume_init(), g_font_roboto[index_base].0 as f32);

            let test = raylib_sys::MeasureTextEx(main_font.0, c_str_leaky("a"), main_font.1, 1.0);
            let left = dimensions.x * 0.5 - test.x * 22.0;
            let right = dimensions.x * 0.5 + test.x * 22.0;

            Self {
                dimensions,
                title_font,
                main_font,
                t_for_fade,
                pos: V2::default(),

                left,
                right,
            }
        }
    }

    pub fn title(&self, text: &str) -> V2 {
        let pos = V2::new(self.dimensions.x * 0.5, self.dimensions.y * 0.3);
        let text_c = c_str_temp(text);
        let spacing = 1.0;
        let color = WHITE;

        unsafe {
            let text_size_vector2 = raylib_sys::MeasureTextEx(self.title_font.0, text_c, self.title_font.1, spacing);
            let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
            let pos = pos - text_size * 0.5;
            raylib_sys::DrawTextEx(self.title_font.0, text_c, to_vector2(pos), self.title_font.1, spacing, color);

            text_size
        }
    }

    pub fn text_center(&self, text: &str, highlighted: bool) -> V2 {
        let text_c = c_str_temp(text);
        let spacing = 1.0;

        let mut color = WHITE;
        if (highlighted) {
            // @Dedupe
            // Copypasta from console
            let cursor_col_lerp_t = 0.5 + 0.5 * 
                (self.t_for_fade as f32 / 30.0).cos();
            color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
        }

        unsafe {
            let text_size_vector2 = raylib_sys::MeasureTextEx(self.main_font.0, text_c, self.main_font.1, spacing);
            let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
            let pos = self.pos - text_size * 0.5;
            raylib_sys::DrawTextEx(self.main_font.0, text_c, to_vector2(pos), self.main_font.1, spacing, color);

            text_size
        }
    }

    pub fn text_center_incr_padding(&mut self, text: &str, padding: f32, highlighted: bool) {
        self.pos.y += self.text_center(text, highlighted).y;
        self.pos.y += padding;
    }

    pub fn text_left_right(&self, text_left: &str, text_right: &str, highlighted: bool) -> V2 {
        let text_c_left = c_str_temp(text_left);
        let text_c_right = c_str_temp(text_right);

        let font = self.main_font.0;
        let font_size = self.main_font.1;

        let spacing = 1.0;

        let mut color = WHITE;
        if (highlighted) {
            // @Dedupe
            // Copypasta from console
            let cursor_col_lerp_t = 0.5 + 0.5 * 
                (self.t_for_fade as f32 / 30.0).cos();
            color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
        }

        unsafe {
            let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_left, font_size, spacing);
            let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
            //let pos = pos - text_size * 0.5;
            let pos = V2::new(self.left, self.pos.y);
            raylib_sys::DrawTextEx(font, text_c_left, to_vector2(pos), font_size, spacing, color);

            let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_right, font_size, spacing);
            let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
            let pos = V2::new(self.right - text_size.x, pos.y);
            raylib_sys::DrawTextEx(font, text_c_right, to_vector2(pos), font_size, spacing, color);

            if (highlighted) {
                let hoz_padding = 12.0;
                let triangle_mid = pos + V2::new(-hoz_padding, text_size.y * 0.5);
                let p0 = triangle_mid - V2::new(0.0, text_size.y * 0.2);
                let p1 = triangle_mid + V2::new(0.0, text_size.y * 0.2);
                let p2 = triangle_mid - V2::new(text_size.y * 0.3, 0.0);
                raylib_sys::DrawTriangle(
                    to_vector2(p0),
                    to_vector2(p2),
                    to_vector2(p1),
                    color,
                );

                let triangle_mid = pos + V2::new(text_size.x + hoz_padding, text_size.y * 0.5);
                let p0 = triangle_mid - V2::new(0.0, text_size.y * 0.2);
                let p1 = triangle_mid + V2::new(0.0, text_size.y * 0.2);
                let p2 = triangle_mid + V2::new(text_size.y * 0.3, 0.0);
                raylib_sys::DrawTriangle(
                    to_vector2(p0),
                    to_vector2(p1),
                    to_vector2(p2),
                    color,
                );
            }

            text_size
        }
    }

    pub fn text_left_right_incr_padding(&mut self, text_left: &str, text_right: &str, padding: f32, highlighted: bool) {
        self.pos.y += self.text_left_right(text_left, text_right, highlighted).y;
        self.pos.y += padding;
    }
}