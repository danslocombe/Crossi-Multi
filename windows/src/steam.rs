pub static mut g_steam_client: Option<steamworks::Client> = None;
pub static mut g_steam_client_single: Option<steamworks::SingleClient> = None;

pub fn init() -> bool {
    unsafe {
        match steamworks::Client::init_app(3429480) {
            Ok((client, single_client)) => {
                client.input().init(false);

                //let language = client.utils().ui_language();
                //println!("Language: {}", language);
                //let app_id = client.utils().app_id();
                //println!("AppId: {}", app_id.0);
                //let country = client.utils().ip_country();
                //println!("Country: {}", country);
                //let country = client.input().get_digital_action_handle("blah")
                //println!("Country: {}", country);
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
        if let Some(client) = g_steam_client_single.as_ref() {
            client.run_callbacks();
        }

        if let Some(client) = g_steam_client.as_ref() {
            let input = client.input();

            let mut controller_slice: [u64;16] = [0;16];
            let controller_count = input.get_connected_controllers_slice(&mut controller_slice);

            if controller_count > 0 && g_input_handles.is_none() {
                let actionset_ingame = input.get_action_set_handle("InGameControls");
                if (actionset_ingame != 0) {
                    let actionset_menucontrols = input.get_action_set_handle("MenuControls");

                    let game_up = input.get_digital_action_handle("up");
                    let menu_up = input.get_digital_action_handle("menu_up");
                    let menu_return_to_game = input.get_digital_action_handle("menu_return_to_game");

                    g_input_handles = Some(SteamInputHandles {
                        actionset_ingame,
                        actionset_menucontrols,

                        menu_up,
                        menu_return_to_game,

                        game_up,
                    });

                    println!("Setup handles: {:#?}", g_input_handles);
                }
                else {
                    //println!("Not Setting!");
                }
            }
        }
    }
}

#[derive(Debug)]
struct SteamInputHandles {
    actionset_ingame: u64,
    actionset_menucontrols: u64,

    menu_up: u64,
    //menu_down: u64,
    //menu_left: u64,
    //menu_right: u64,
    //menu_select: u64,
    menu_return_to_game: u64,

    game_up: u64,
    //game_down: u64,
    //game_left: u64,
    //game_right: u64,
    //game_pause_menu: u64,
}

static mut g_input_handles: Option<SteamInputHandles> = None;

//const INVALID_HANDLE: u64 = 0;
//static mut g_steam_up_handle: u64 = INVALID_HANDLE;
//static mut g_steam_down_handle: u64 = INVALID_HANDLE;
//static mut g_steam_left_handle: u64 = INVALID_HANDLE;
//static mut g_steam_right_handle: u64 = INVALID_HANDLE;