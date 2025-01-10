use crate::input::MenuInput;

pub static mut g_steam_client: Option<steamworks::Client> = None;
pub static mut g_steam_client_single: Option<steamworks::SingleClient> = None;

pub static mut g_connected_controllers: [u64;16] = [0;16];
pub static mut g_controller_count: usize = 0;

pub static mut g_input_handles: Option<SteamInputHandles> = None;
pub static mut g_controller_input_states: Vec<InputState> = Vec::new();

pub static mut g_t: i32 = 0;

const STEAM_INPUT_HANDLE_ALL_CONTROLLERS: u64 = u64::MAX; 

pub fn init() -> bool {
    unsafe {
        match steamworks::Client::init_app(3429480) {
            Ok((client, single_client)) => {
                client.input().init(false);

                g_steam_client = Some(client);
                g_steam_client_single = Some(single_client);

                true
            },
            Err(e) => {
                eprintln!("Failed to init steam client {}", e);
                false
            }
        }
    }
}

pub fn tick() {
    unsafe {
        g_t += 1;

        if let Some(client) = g_steam_client_single.as_ref() {
            client.run_callbacks();
        }

        if let Some(client) = g_steam_client.as_ref() {
            let input = client.input();

            g_controller_count = input.get_connected_controllers_slice(&mut g_connected_controllers);

            if g_controller_count > 0 && g_input_handles.is_none() {
                let actionset_ingame = input.get_action_set_handle("InGameControls");
                if (actionset_ingame != 0) {
                    let actionset_menucontrols = input.get_action_set_handle("MenuControls");

                    let game_up = input.get_digital_action_handle("up");
                    let game_down = input.get_digital_action_handle("down");
                    let game_left = input.get_digital_action_handle("left");
                    let game_right = input.get_digital_action_handle("right");
                    let game_pause_menu = input.get_digital_action_handle("pause_menu");

                    let menu_up = input.get_digital_action_handle("menu_up");
                    let menu_down = input.get_digital_action_handle("menu_down");
                    let menu_left = input.get_digital_action_handle("menu_left");
                    let menu_right = input.get_digital_action_handle("menu_right");
                    let menu_select = input.get_digital_action_handle("menu_select");
                    let menu_return_to_game = input.get_digital_action_handle("menu_return_to_game");

                    g_input_handles = Some(SteamInputHandles {
                        actionset_ingame,
                        actionset_menucontrols,

                        game_up,
                        game_down,
                        game_left,
                        game_right,
                        game_pause_menu,

                        menu_up,
                        menu_down,
                        menu_left,
                        menu_right,
                        menu_select,
                        menu_return_to_game,
                    });

                    println!("Setup handles: {:#?}", g_input_handles);
                }
                else {
                    //println!("Not Setting!");
                }
            }

            if let Some(handles) = g_input_handles.as_ref() {
                // @Nocheckin
                input.activate_action_set_handle(STEAM_INPUT_HANDLE_ALL_CONTROLLERS, handles.actionset_menucontrols);

                for i in 0..g_controller_count {
                    let controller_id = g_connected_controllers[i];

                    if let Some(state) = g_controller_input_states.iter_mut().find(|x| x.controller_id == controller_id) {
                        state.tick(&input, handles);
                    }
                    else {
                        // @Fixme ignoring inputs on first frame of connection.
                        let mut state = InputState::default();
                        state.controller_id = controller_id;
                        g_controller_input_states.push(state);
                    }

                    // @Incomplete
                    // Prune this maybe?
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SteamInputHandles {
    pub actionset_ingame: u64,
    pub actionset_menucontrols: u64,

    pub game_up: u64,
    pub game_down: u64,
    pub game_left: u64,
    pub game_right: u64,
    pub game_pause_menu: u64,

    pub menu_up: u64,
    pub menu_down: u64,
    pub menu_left: u64,
    pub menu_right: u64,
    pub menu_select: u64,
    pub menu_return_to_game: u64,
}

#[derive(Default, Clone)]
pub struct InputState
{
    controller_id: u64,

    // Takes over a year to overflow. 
    last_update: i32,

    prev: ButtonStates,
    current: ButtonStates,
}

#[derive(Default, Clone)]
pub struct ButtonStates {
    game_up: bool,
    game_down: bool,
    game_left: bool,
    game_right: bool,
    game_select: bool,
    game_pause_menu: bool,

    menu_up: bool,
    menu_down: bool,
    menu_left: bool,
    menu_right: bool,
    menu_select: bool,
    menu_return_to_game: bool,
}

impl InputState {
    pub fn tick(&mut self, steam_input: &steamworks::Input<steamworks::ClientManager>, handles: &SteamInputHandles) {
        self.last_update = unsafe { g_t };

        self.prev = self.current.clone();

        self.current.menu_up = steam_input.get_digital_action_data(self.controller_id, handles.menu_up).bState;
        self.current.menu_down = steam_input.get_digital_action_data(self.controller_id, handles.menu_down).bState;
        self.current.menu_left = steam_input.get_digital_action_data(self.controller_id, handles.menu_left).bState;
        self.current.menu_right = steam_input.get_digital_action_data(self.controller_id, handles.menu_right).bState;
        self.current.menu_select = steam_input.get_digital_action_data(self.controller_id, handles.menu_select).bState;
        self.current.menu_return_to_game = steam_input.get_digital_action_data(self.controller_id, handles.menu_return_to_game).bState;
    }
}

pub fn read_menu_input() -> MenuInput {
    unsafe {
        for state in g_controller_input_states.iter() {
            if (state.last_update != g_t) {
                continue;
            }

            if state.current.menu_up && !state.prev.menu_up {
                return MenuInput::Up;
            }
            if state.current.menu_down && !state.prev.menu_down {
                return MenuInput::Down;
            }
            if state.current.menu_left && !state.prev.menu_left {
                return MenuInput::Left;
            }
            if state.current.menu_right && !state.prev.menu_right {
                return MenuInput::Right;
            }
            if state.current.menu_select && !state.prev.menu_select {
                return MenuInput::Select;
            }
            if state.current.menu_return_to_game && !state.prev.menu_return_to_game {
                return MenuInput::ReturnToGame;
            }
        }
    }

    MenuInput::None
}
