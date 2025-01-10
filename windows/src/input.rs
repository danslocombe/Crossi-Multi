use crate::{gamepad_pressed, key_pressed, steam::g_steam_client_single};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    None,
    Up,
    Down,
    Left,
    Right,
    Select,
    ReturnToGame,
}

impl MenuInput {
    pub fn is_toggle(self) -> bool {
        match self {
            MenuInput::Left | MenuInput::Right | MenuInput::Select => true,
            _ => false
        }
    }

    pub fn read() -> Self {
        let input = Self::read_raylib_keyboard();
        if (input != Self::None) {
            return input;
        }

        if (using_steam_input()) {
            crate::steam::read_menu_input()
        }
        else {
            Self::read_raylib_controllers()
        }
    }

    pub fn read_raylib_keyboard() -> Self {
        if key_pressed(raylib_sys::KeyboardKey::KEY_UP) {
            return MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_LEFT) {
            return MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_DOWN) {
            return MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT) {
            return MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_SPACE) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_ENTER) {
            return MenuInput::Select;
        }

        if key_pressed(raylib_sys::KeyboardKey::KEY_W) {
            return MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_A) {
            return MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_S) {
            return MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_D) {
            return MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_Z) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_X) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_F) {
            return MenuInput::Select;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_G) {
            return MenuInput::Select;
        }

        Self::None
    }

    pub fn read_raylib_controllers() -> Self {
        for i in 0..4 {
            let gamepad_id = i as i32;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                return MenuInput::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                return MenuInput::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                return MenuInput::Down;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                return MenuInput::Right;
            }

            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                return MenuInput::Select;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                return MenuInput::Select;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                return MenuInput::Select;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                return MenuInput::Select;
            }
        }

        Self::None
    }
}