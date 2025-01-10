use crate::{gamepad_pressed, key_pressed};

static mut g_steam_input: bool = true;

pub fn using_steam_input() -> bool {
    unsafe { g_steam_input }
}

pub fn init()
{
    if (using_steam_input()) {
        unsafe {
            if let Some(x) = crate::steam::g_steam_client.as_ref() {
                //let input = x.input();
                //let connected_controllers = input.get_connected_controllers();
                //println!("connected controllers {:?}", connected_controllers);
                ////let xx = input.get_digital_action_handle("MenuControls");
                //let actionset = input.get_action_set_handle("MenuControls");
                //println!("actionset {}", actionset);
                //let actionset = input.get_action_set_handle("InGameControls");
                //println!("actionset {}", actionset);

                //g_steam_up_handle = input.get_digital_action_handle("menu_up");
                ////println!("up handle {}", g_steam_up_handle);
                //g_steam_down_handle = input.get_digital_action_handle("menu_down");
                //g_steam_left_handle = input.get_digital_action_handle("menu_left");
                //g_steam_right_handle = input.get_digital_action_handle("menu_right");
            }
        }
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
        if (using_steam_input()) {
            Self::read_steam()
        }
        else {
            Self::read_raylib()
        }
    }

    pub fn read_steam() -> Self {
        unimplemented!()
    }

    pub fn read_raylib() -> Self {
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