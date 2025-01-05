use crossy_multi_core::math::V2;
use crate::{audio, c_str_temp, client::{g_music_volume, river_col_1, VisualEffects}, gamepad_pressed, key_pressed, lerp_color_rgba, to_vector2, WHITE};

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
            if !settings.tick() {
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

        unsafe {
            let padding = 16.0;
            let width = raylib_sys::GetScreenWidth();
            let height = raylib_sys::GetScreenHeight();
            let dimensions = V2::new(width as f32, height as f32);
            let text_size = draw_text_center_aligned_ex(self.t_since_move, "Paused", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false, true);
            //let text_size = self.draw_text_center_aligned("Paused", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false);

            let mut p = V2::new(dimensions.x * 0.5, dimensions.y * 0.4);
            p.y += text_size.y + padding;

            let text_size = self.draw_text_center_aligned("Resume", p, self.highlighted == 0);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Lobby", p, self.highlighted == 1);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Settings", p, self.highlighted == 2);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Submit Feedback", p, self.highlighted == 3);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Exit", p, self.highlighted == 4);
        }
    }

    fn draw_text_center_aligned(&self, text: &str, pos: V2, highlighted: bool) -> V2 {
        draw_text_center_aligned_ex(self.t_since_move, text, pos, highlighted, false)
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

    pub fn tick(&mut self) -> bool {
        self.t += 1;
        self.t_since_move += 1;

        let input = MenuInput::read();

        // @Fragile
        let option_count = 6;

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
                    state.music_volume -= 0.1;
                    state.validate();
                    unsafe {
                        g_music_volume = state.music_volume;
                    }
                    crate::settings::set(state);
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.music_volume += 0.1;
                    state.validate();
                    unsafe {
                        g_music_volume = state.music_volume;
                    }
                    crate::settings::set(state);
                }
            }
            1 => {
                if let MenuInput::Left = input {
                    let mut state = crate::settings::get();
                    state.sfx_volume -= 0.1;
                    state.validate();
                    crate::settings::set(state);
                    audio::play("menu_click");
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.sfx_volume += 0.1;
                    state.validate();
                    crate::settings::set(state);
                    audio::play("menu_click");
                }
            }
            2 => {
                // Window mode
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.fullscreen = !state.fullscreen;
                    crate::settings::set(state)
                }
            }
            3 => {
            }
            4 => {
                // CRT
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.crt_shader = !state.crt_shader;
                    crate::settings::set(state)
                }
            }
            5 => {
                if let MenuInput::Enter = input {
                    audio::play("menu_click");
                    return false;
                }
            }
            _ => {
                // @Unreachable
                debug_assert!(false);
            }
        }

        true
    }

    pub fn draw(&self) {
        unsafe {
            let padding = 16.0;
            let width = raylib_sys::GetScreenWidth();
            let height = raylib_sys::GetScreenHeight();
            let dimensions = V2::new(width as f32, height as f32);
            //let text_size = self.draw_text_center_aligned("Settings", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false);
            let text_size = draw_text_center_aligned_ex(self.t_since_move, "Settings", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false, true);

            let left = width as f32 * 0.35;
            let right = width as f32 * 0.65;

            let mut p = V2::new(dimensions.x * 0.5, dimensions.y * 0.4);
            p.y += text_size.y + padding;

            let settings = crate::settings::get();

            let music_percentage = format!("{}%", (settings.music_volume * 100.0).round());
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Music Volume:", &music_percentage, left, right, p, self.highlighted == 0, true);
            p.y += text_size.y + padding;

            let sfx_percentage = format!("{}%", (settings.sfx_volume * 100.0).round());
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Sound Effects Volume:", &sfx_percentage, left, right, p, self.highlighted == 1, true);
            p.y += text_size.y + padding;
            //let fullscreen_text = format!("Window Mode: {}", if settings.fullscreen { "Fullscreen" } else { "Windowed"} );
            //let text_size = self.draw_text_center_aligned(&fullscreen_text, p, self.highlighted == 2);
            let window_mode = if settings.fullscreen { "Fullscreen" } else { "Windowed" };
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Window Mode:", window_mode, left, right, p, self.highlighted == 2, false);
            p.y += text_size.y + padding;
            //let text_size = self.draw_text_center_aligned("Visual Effects: Full", p, self.highlighted == 3);
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Visual Effects:", "Full", left, right, p, self.highlighted == 3, false);
            p.y += text_size.y + padding;
            //let text_size = self.draw_text_center_aligned("CRT Shader: Enabled", p, self.highlighted == 4);
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "CRT Effect:", "Enabled", left, right, p, self.highlighted == 4, false);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Back", p, self.highlighted == 5);
        }
    }

    fn draw_text_center_aligned(&self, text: &str, pos: V2, highlighted: bool) -> V2 {
        draw_text_center_aligned_ex(self.t_since_move, text, pos, highlighted, false)
    }
}


fn draw_text_center_aligned_ex(t_since_move: i32, text: &str, pos: V2, highlighted: bool, big: bool) -> V2 {
    let text_c = c_str_temp(text);
    let spacing = 1.0;

    let mut color = WHITE;
    if (highlighted) {
        // @Dedupe
        // Copypasta from console
        let cursor_col_lerp_t = 0.5 + 0.5 * 
            (t_since_move as f32 / 30.0).cos();
        color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
    }

    unsafe {
        let (font, font_size) = if big {
            (crate::FONT_ROBOTO_BOLD_80.assume_init(), 80.0)
        }
        else {
            (crate::FONT_ROBOTO_BOLD_60.assume_init(), 60.0)
        };
        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        let pos = pos - text_size * 0.5;
        raylib_sys::DrawTextEx(font, text_c, to_vector2(pos), font_size, spacing, color);


        /*
        if (highlighted) {
            let square_size = 16.0;
            let hoz_padding = 32.0;
            raylib_sys::DrawRectangleRec(raylib_sys::Rectangle {
                x: (pos.x - hoz_padding - square_size),
                y: (pos.y + text_size.y * 0.5 - square_size * 0.5),
                width: square_size,
                height: square_size,
            }, color);
        }
        */

        text_size
    }
}

fn draw_text_left_right_aligned_ex(t_since_move: i32, text_left: &str, text_right: &str, left: f32, right: f32, pos: V2, highlighted: bool, arrows: bool) -> V2 {
    let text_c_left = c_str_temp(text_left);
    let text_c_right = c_str_temp(text_right);
    let spacing = 1.0;

    let mut color = WHITE;
    if (highlighted) {
        // @Dedupe
        // Copypasta from console
        let cursor_col_lerp_t = 0.5 + 0.5 * 
            (t_since_move as f32 / 30.0).cos();
        color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
    }

    unsafe {
        let (font, font_size) =
            (crate::FONT_ROBOTO_BOLD_60.assume_init(), 60.0);

        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_left, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        //let pos = pos - text_size * 0.5;
        let pos = V2::new(left, pos.y);
        raylib_sys::DrawTextEx(font, text_c_left, to_vector2(pos), font_size, spacing, color);

        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_right, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        let pos = V2::new(right - text_size.x, pos.y);
        raylib_sys::DrawTextEx(font, text_c_right, to_vector2(pos), font_size, spacing, color);

        //if (arrows && highlighted) {
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