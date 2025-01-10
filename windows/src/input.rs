use crossy_multi_core::game;

use crate::{gamepad_pressed, key_pressed, steam::g_steam_client_single};

//static mut g_steam_input: bool = true;
static mut g_steam_input: bool = false;

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

pub fn arrow_game_input() -> game::Input {
    if (!crate::console::eating_input()) {
        if (key_pressed(raylib_sys::KeyboardKey::KEY_LEFT)) {
            return game::Input::Left;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT)) {
            return game::Input::Right;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_UP)) {
            return game::Input::Up;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_DOWN)) {
            return game::Input::Down;
        }
    }

    game::Input::None
}

pub fn wasd_game_input() -> game::Input {
    if (!crate::console::eating_input()) {
        if (key_pressed(raylib_sys::KeyboardKey::KEY_LEFT)) {
            return game::Input::Left;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT)) {
            return game::Input::Right;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_UP)) {
            return game::Input::Up;
        }
        if (key_pressed(raylib_sys::KeyboardKey::KEY_DOWN)) {
            return game::Input::Down;
        }
    }

    game::Input::None
}


pub fn game_input_controller_raylib(gamepad_id: i32) -> game::Input {
    if (unsafe { raylib_sys::IsGamepadAvailable(gamepad_id) })
    {
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
            return game::Input::Left;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
            return game::Input::Right;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
            return game::Input::Up;
        }
        if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
            return game::Input::Down;
        }
    }

    game::Input::None
    /*
    if (unsafe { raylib_sys::IsGamepadAvailable(gamepad_id) })
    {
        {
            let mut input = Input::None;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                input = Input::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                input = Input::Right;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                input = Input::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                input = Input::Down;
            }
            Self::process_input(&mut self.controller_a_players[gamepad_id as usize], input, &mut player_inputs, timeline, players_local, outfit_switchers, &mut new_players, Some(gamepad_id));
        }

        if (false) {
            // Need to rethink this
            // I want this to be possible but will probably need some interaction setup

            let mut input = Input::None;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                input = Input::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                input = Input::Right;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                input = Input::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                input = Input::Down;
            }
            Self::process_input(&mut self.controller_b_players[gamepad_id as usize], input, &mut player_inputs, timeline, players_local, outfit_switchers, &mut new_players, Some(gamepad_id));
        }
    }
    */
}