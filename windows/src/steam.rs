pub static mut g_steam_client: Option<steamworks::Client> = None;
pub static mut g_steam_client_single: Option<steamworks::SingleClient> = None;

pub fn init() -> bool {
    unsafe {
        match steamworks::Client::init_app(3429480) {
            Ok((client, single_client)) => {
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
    }
}