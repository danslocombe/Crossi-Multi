use std::{io::BufWriter, mem::MaybeUninit, str::FromStr};

use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, EndWinnerState, WINNER_TIME_US}, ring_buffer::RingBuffer, timeline::Timeline, DebugLogger, Input, PlayerId, PlayerInputs, Pos};

use crate::{player_local::{PlayerInputController, Skin}, Client};

pub struct QuakeConsoleLogger {
}

impl DebugLogger for QuakeConsoleLogger {
    fn log(&self, logline: &str) {
        unsafe {
            g_console.assume_init_mut().write_with_type(logline.to_owned(), LineType::Info);
        }
    }
}

pub fn init_console() {
    unsafe {
        let mut command_set = CommandSet::create();
        command_set.commands.push(Command {
            name: "new".to_owned(),
            lambda: Box::new(do_new),
        });
        command_set.commands.push(Command {
            name: "exit".to_owned(),
            lambda: Box::new(do_exit),
        });
        command_set.commands.push(Command {
            name: "debug".to_owned(),
            lambda: Box::new(do_toggle_debug),
        });
        command_set.commands.push(Command {
            name: "skin".to_owned(),
            lambda: Box::new(do_set_skin),
        });
        command_set.commands.push(Command {
            name: "win".to_owned(),
            lambda: Box::new(do_win),
        });
        command_set.commands.push(Command {
            name: "min_players".to_owned(),
            lambda: Box::new(do_set_min_players),
        });
        command_set.commands.push(Command {
            name: "restart".to_owned(),
            lambda: Box::new(do_restart),
        });
        command_set.commands.push(Command {
            name: "lobby".to_owned(),
            lambda: Box::new(do_lobby),
        });
        command_set.commands.push(Command {
            name: "start_recording".to_owned(),
            lambda: Box::new(do_start_recording),
        });
        command_set.commands.push(Command {
            name: "stop_recording".to_owned(),
            lambda: Box::new(do_stop_recording),
        });
        command_set.commands.push(Command {
            name: "sr".to_owned(),
            lambda: Box::new(do_stop_recording),
        });
        command_set.commands.push(Command {
            name: "add_player".to_owned(),
            lambda: Box::new(do_add_player),
        });
        command_set.commands.push(Command {
            name: "trailer_mode".to_owned(),
            lambda: Box::new(do_toggle_trailer_mode),
        });
        command_set.commands.push(Command {
            name: "game_config".to_owned(),
            lambda: Box::new(do_game_config),
        });
        command_set.commands.push(Command {
            name: "seed".to_owned(),
            lambda: Box::new(do_seed),
        });
        command_set.commands.push(Command::new(
            "dump_controllers",
            Box::new(do_dump_controllers),
        ));
        g_console = MaybeUninit::new(Console::new(command_set));
    }
}

pub fn info(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(s.to_owned(), LineType::Info);
    }
}

macro_rules! info {
    ( $( $t:tt )* ) => {
        info(&format!( $( $t )* ));
    }
}

pub fn big(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(s.to_owned(), LineType::Big);
    }
}

macro_rules! big {
    ( $( $t:tt )* ) => {
        big(&format!( $( $t )* ));
    }
}

pub fn err(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(format!("Error: {}", s), LineType::Error);
    }
}

macro_rules! err {
    ( $( $t:tt )* ) => {
        err(&format!( $( $t )* ));
    }
}

pub fn toggle_open() {
    unsafe {
        g_console.assume_init_mut().toggle_open();
    }
}

pub fn eating_input() -> bool {
    unsafe {
        g_console.assume_init_mut().eating_input()
    }
}

pub fn tick(client: &mut Client) {
    unsafe {
        g_console.assume_init_mut().tick(client);
    }
}

pub fn draw(client: &Client) {
    unsafe {
        g_console.assume_init_mut().draw(client);
    }
}

static mut g_console: MaybeUninit<Console> = MaybeUninit::uninit();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineType {
    Empty,
    Big,
    Info,
    Error,
    UserEntered,
}

#[derive(Clone)]
struct ConsoleLine {
    t: i32,
    line_type: LineType,
    line: String,
}

#[derive(Debug, Clone, Copy)]
enum ConsoleMode {
    Quiet,
    Open((f32, bool)),
}

struct Console {
    t: i32,
    lines: RingBuffer<ConsoleLine>,
    prompt: TextInput,
    state: ConsoleMode,
    command_set: CommandSet,
}

impl Console {
    pub fn new(command_set: CommandSet) -> Self {
        Self {
            t: 0,
            lines: RingBuffer::new_with_value(64, ConsoleLine {
                t: 0,
                line_type: LineType::Empty,
                line: Default::default(),
            }),

            prompt: TextInput::default(),
            state: ConsoleMode::Quiet,
            command_set,
        }
    }

    pub fn write_with_type(&mut self, line: String, line_type: LineType) {
        println!("{}", line);
        self.lines.push(ConsoleLine {
            t: self.t,
            line_type,
            line,
        })
    }

    pub fn toggle_open(&mut self) {
        match (&self.state) {
            ConsoleMode::Quiet => {
                self.state = ConsoleMode::Open((0.0, true));
            },
            ConsoleMode::Open((t, opening)) => {
                self.state = ConsoleMode::Open((*t, !opening));
            },
        }
    }

    pub fn eating_input(&self) -> bool {
        match (&self.state) {
            ConsoleMode::Quiet => {
                false
            },
            ConsoleMode::Open((t, opening)) => {
                *opening
            },
        }
    }

    pub fn tick(&mut self, client: &mut Client) {
        self.t += 1;

        if let ConsoleMode::Open((mut t, opening)) = self.state {
            let k_open = 0.15;
            let k_close = 0.20;
            self.prompt.tick();
            if (crate::key_pressed(raylib_sys::KeyboardKey::KEY_ENTER)) {
                let command = std::str::from_utf8(&self.prompt.buffer[0..]).unwrap().to_owned();
                self.write_with_type(format!("> {}", &command), LineType::UserEntered);

                if (!command.is_empty()) {
                    self.command_set.run(command, client);
                    self.prompt.buffer.clear();
                }
            }

            if (opening) {
                t += k_open;
                if (t > 1.0) {
                    t = 1.0;
                }

                self.state = ConsoleMode::Open((t, opening));
            } else {
                t -= k_close;
                if (t < 0.0) {
                    self.state = ConsoleMode::Quiet;
                }
                else {
                    self.state = ConsoleMode::Open((t, opening));
                }
            }
        }
        else {
        }
    }

    pub fn draw(&mut self, client: &Client) {
        let size = self.lines.size();
        match &self.state {
            ConsoleMode::Quiet => {
                if !client.debug {
                    return;
                }

                let mut yy: i32 = 20;
                let xx: i32 = 20;
                let cutoff = self.t - 60 * 3;
                for i in 0..size {
                    let message_index = (size - 1) - i;
                    let offset = message_index as i32;
                    let message = self.lines.get(offset);
                    if (message.line_type == LineType::Empty || message.line.is_empty()) {
                        continue;
                    }

                    if (message.t > cutoff) {
                        unsafe {
                            raylib_sys::DrawText(crate::c_str_temp(&message.line), xx, yy, 18, crate::WHITE);
                        }

                        yy += 22;
                    }
                }
            },
            ConsoleMode::Open((open_t, _opening)) => unsafe {
                let tt = open_t * open_t;
                let bot_full = 0.3;

                let rect = raylib_sys::Rectangle{
                    x: 0.0,
                    y: 0.0,
                    width: raylib_sys::GetScreenWidth() as f32,
                    height: tt * bot_full * raylib_sys::GetScreenHeight() as f32,
                };

                let mut color = crate::DARKGREY;
                color.a = 200;

                raylib_sys::DrawRectangleRec(rect, color);

                let font_size = 20;
                let hspace = 8;
                let mut yy = rect.height as i32 - font_size - hspace;
                let xx: i32 = 20;

                self.prompt.draw("> ", xx, yy, font_size);
                yy -= font_size + hspace;

                for i in 0..size {
                    let offset = -(i as i32);
                    let message = self.lines.get(offset);
                    if (message.line_type == LineType::Empty || message.line.is_empty()) {
                        continue;
                    }

                    let col = match (message.line_type) {
                        LineType::Info => crate::YELLOW,
                        LineType::Big => crate::YELLOW,
                        LineType::UserEntered => crate::WHITE,
                        LineType::Error => crate::RED,
                        _ => unreachable!(),
                    };

                    raylib_sys::DrawText(crate::c_str_temp(&message.line), xx, yy, font_size, col);
                    yy -= font_size + hspace;
                }
            },
        }

        let cutoff = self.t - 60 * 1;
        for i in 0..size {
            let offset = -(i as i32);
            let message = self.lines.get(offset);
            if (message.line_type != LineType::Big || message.line.is_empty()) {
                continue;
            }

            if (message.t > cutoff) {
                let screen_width = unsafe { raylib_sys::GetScreenWidth() };
                let screen_height = unsafe { raylib_sys::GetScreenHeight() };
                let str = crate::c_str_temp(&message.line);
                let size = unsafe { raylib_sys::MeasureText(str, 32)};

                let yy: i32 = screen_height / 3;
                let xx: i32 = screen_width / 2 - size / 2;

                unsafe {
                    raylib_sys::DrawText(str, xx, yy, 28, crate::BLACK);
                }
            }
            else {
                // Nothing
            }

            break;
        }
    }
}

#[derive(Default)]
struct TextInput {
    buffer: Vec<u8>,
    t_since_last_keypress: i32,
}

impl TextInput {
    pub fn tick(&mut self) {
        self.t_since_last_keypress += 1;

        loop {
            let c = unsafe {
                raylib_sys::GetCharPressed()
            };

            if (c <= 0) {
                break;
            }

            if (c >= 32 && c <= 125) {
                // @Hack, @TODO better way to filter these on console open.
                if (c == b'`' as i32) {
                    continue;
                }

                if self.buffer.len() < 255 {
                    self.buffer.push(c as u8);
                }

                self.t_since_last_keypress = 0;
            }
        }

        if (crate::key_pressed(raylib_sys::KeyboardKey::KEY_BACKSPACE)) {
            if (crate::key_down(raylib_sys::KeyboardKey::KEY_LEFT_CONTROL)) {
                // No utf8 here!
                while let Some(c) = self.buffer.pop() {
                    if c == b' ' {
                        break;
                    }
                }
            } else {
                _ = self.buffer.pop();
            }

            self.t_since_last_keypress = 0;
        }
    }

    pub fn draw(&mut self, prefix: &str, x: i32, y: i32, font_size: i32) {
        unsafe {
            // @Perf could prevent all these copies by pointing directly to the buffer
            // and doing the accounting to make sure theres always a null terminator, but its not worth it.
            let buffer_utf8 = std::str::from_utf8(&self.buffer).unwrap();
            let str = format!("{}{}", prefix, buffer_utf8);
            let c_str = crate::c_str_temp(&str);
            raylib_sys::DrawText(c_str, x, y, font_size, crate::WHITE);
            let text_len = raylib_sys::MeasureText(c_str, font_size);

            let cursor_col_lerp_t = 0.5 + 0.5 * 
                (self.t_since_last_keypress as f32 / 30.0).cos();
            let cursor_col = crate::lerp_color_rgba(crate::SEA, crate::WHITE, cursor_col_lerp_t);

            raylib_sys::DrawRectangleRec(raylib_sys::Rectangle {
                x: (x + text_len + 1) as f32,
                y: (y - 1) as f32,
                width: 4.,
                height: font_size as f32 + 2.0,
            }, cursor_col);
        }
    }
}

#[derive(Default)]
struct CommandSet {
    commands: Vec<Command>,
}

impl CommandSet {
    pub fn create() -> Self {
        // Dumber implementation than in bounce
        // We arent going to do any fancy registering
        //
        // Commands will just be a lambda that takes a reference to the game
        // and any args.

        let commands = vec![
        ];

        Self {
            commands,
        }
    }

    pub fn run(&self, s: String, client: &mut Client) {
        let split: Vec<&str> = s.split(' ').collect();
        if let Some(command_name) = split.first() {

            let mut found = false;
            for command in &self.commands {
                if (command.name.eq_ignore_ascii_case(&command_name)) {
                    let rest = &split[1..];
                    (command.lambda)(rest, client);

                    found = true;
                    break;
                }
            }

            if (!found) {
                err(&format!("Could not find command '{}'", command_name));
                //for command in &self.commands {
                //}
            }
        }
    }
}

struct Command {
    name: String,
    lambda: Box<dyn Fn(&[&str], &mut Client)>,
}

impl Command {
    pub fn new(name: &str, lambda: Box<dyn Fn(&[&str], &mut Client)>) -> Self {
        Self {
            name: name.to_owned(),
            lambda,
        }
    }
}

fn do_new(args: &[&str], client: &mut Client) {
    if (args.len() > 1) {
        err!("Expected 0 or 1 argument to new, got {}", args.len());
        info("Usage: new");
        info("Usage: new some_seed");
        return;
    }

    let mut seed = String::default();
    if args.len() == 1 {
        seed = args[0].to_owned();
    }
    else {
        seed = format!("seed_{}", 10);
    }

    big!("New Level Seed '{}'", seed);
    let mut config = client.timeline.top_state().rules_state.config.clone();
    config.bypass_lobby = true;
    client.timeline = Timeline::from_seed(config, &seed);
    client.seed = seed;

    client.player_input_controller = PlayerInputController::default();
    client.entities.clear_round_entities();
    client.entities.players.inner.clear();
}

fn do_toggle_debug(args: &[&str], client: &mut Client) {
    if (args.len() > 0) {
        err!("Expected no arguments to 'debug' got {}", args.len());
        return;
    }

    client.debug = !client.debug;

    if client.debug {
        big("Debug mode");
    }
    else {
        big("Disabling debug mode");
    }
}

fn do_set_skin(args: &[&str], client: &mut Client) {
    if (args.len() != 1 && args.len() != 2) {
        err!("Expected one or two arguments to 'skin' got {}", args.len());
        return;
    }

    let mut player_id = PlayerId(1);

    if (args.len() == 2) {
        if let Ok(id) = args[0].parse() {
            player_id = PlayerId(id);
        }
        else {
            err!("Could not parse {} as a PlayerId (u8)", args[0]);
            return;
        }
    }

    let mut skin = Skin::default();
    if let Ok(s) = crate::player_local::PlayerSkin::from_str(args.last().unwrap()) {
        skin = Skin::from_enum(s);
    }
    else {
        err!("Could not parse {} as a Skin", args.last().unwrap());
        return;
    }

    if let Some(player) = client.entities.players.inner.iter_mut().find(|x| x.player_id == player_id) {
        player.skin = skin;
    }
    else {
        err!("Could not find player with PlayerId {}", player_id.0);
    }
}

fn do_win(args: &[&str], client: &mut Client) {
    if (args.len() > 1) {
        err!("Expected zero or one arguments to 'win' got {}", args.len());
        return;
    }

    let mut player_id = PlayerId(1);

    if (args.len() == 1) {
        if let Ok(id) = args[0].parse() {
            player_id = PlayerId(id);
        }
        else {
            err!("Could not parse {} as a PlayerId (u8)", args[0]);
            return;
        }
    }

    let state = CrossyRulesetFST::EndWinner(EndWinnerState {
        winner_id: player_id,
        remaining_us: WINNER_TIME_US,
    });

    client.timeline.states.front_mut().unwrap().rules_state.fst = state;
}

fn do_set_min_players(args: &[&str], client: &mut Client) {
    if (args.len() != 1) {
        err!("Expected one arguments to 'min_players' got {}", args.len());
        return;
    }

    if let Ok(min_count) = args[0].parse() {
        info!("Setting min_count to {}", min_count);
        client.timeline.top_state_mut_unsafe().rules_state.config.minimum_players = min_count;
    }
    else {
        err!("Could not parse {} as a number", args[0]);
    }
}

fn do_restart(args: &[&str], client: &mut Client) {
    if (args.len() > 0) {
        err!("Expected zero arguments to 'restart' got {}", args.len());
        return;
    }

    let seed = client.seed.clone();
    big!("Restarting, preserving seed '{}'", seed);
    client.goto_loby_seed(&seed, None);
}

fn do_lobby(args: &[&str], client: &mut Client) {
    if (args.len() > 1) {
        err!("Expected 0 or 1 argument to lobby, got {}", args.len());
        return;
    }

    let mut seed = String::default();
    if args.len() == 1 {
        seed = args[0].to_owned();
    }
    else {
        seed = crate::shitty_rand_seed();
    }

    big!("Lobby with Seed '{}'", seed);
    client.goto_loby_seed(&seed, Some(false));
}

fn do_exit(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected zero 'exit' got {}", args.len());
        return;
    }

    big!("Shutting down..");

    client.exit = true;
}

fn do_start_recording(args: &[&str], client: &mut Client) {
    if (args.len() != 1) {
        err!("Expected one arguments to start_recording, got {}", args.len());
        return;
    }

    client.recording_gif_name = args[0].to_owned();
    client.recording_gif = true;

    info!("Recording to {}", args[0]);
    info!("Use 'sr' or 'stop_recording' to stop");
}

fn do_stop_recording(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to stop_recording, got {}", args.len());
        return;
    }

    std::fs::create_dir_all(&format!("gifs/{}", &client.recording_gif_name)).unwrap();

    // Find first frame populated
    let mut first_frame_index = -(client.frame_ring_buffer.size() as i32 - 1);
    while (client.frame_ring_buffer.get(first_frame_index).is_none()) {
        first_frame_index += 1;
        if (first_frame_index >= 0) {
            break;
        }
    }

    let mut buffer = Vec::new();

    let mut i : i32 = first_frame_index;
    let mut frame_id_to_write: i32 = 0;
    while let Some(frame) = client.frame_ring_buffer.get(i) {
        if (i.abs() as usize >= client.frame_ring_buffer.size()) {
            break;
        }

        let name = format!("gifs/{}/frame_{:04}.png", &client.recording_gif_name, frame_id_to_write.abs());

        {
            buffer.clear();
            image_data_to_png(frame, 160, 160, &mut buffer);
            info!("Writing {}", name);
            std::fs::write(name, &buffer).unwrap();
            //zip_maker.write_all(&buffer[..]).unwrap();
        }

        i += 1;
        frame_id_to_write += 1;

        if (i >= 0) {
            break;
        }
    }

    big!("Done writing {}! Clearing buffer", &client.recording_gif_name);
    client.frame_ring_buffer = RingBuffer::new_with_value(60 * 60, None);
    client.recording_gif = false;
}

fn image_data_to_png(raw_data: &[u8], width: u32, height: u32, data: &mut Vec<u8>) {
    //let mut data = Vec::new();

    let mut processed_data: Vec<u8> = Vec::with_capacity(raw_data.len());

    // Hack around upside down data.
    for y in 0..(height as usize) {
        let y = height as usize - y - 1;
        let start = y * width as usize * 4;
        let end = (y + 1) * width as usize * 4;
        processed_data.extend_from_slice(&raw_data[start..end]);
    }

    {
        //let writer = BufWriter::new(&mut data);
        let writer = BufWriter::new(data);
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
        encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
        let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
            (0.31270, 0.32900),
            (0.64000, 0.33000),
            (0.30000, 0.60000),
            (0.15000, 0.06000)
        );

        encoder.set_source_chromaticities(source_chromaticities);
        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(&processed_data).unwrap();
    }

    //data
}

fn do_add_player(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to add_player, got {}", args.len());
        return;
    }

    let mut registration = None;
    let mut dummy_player_inputs = PlayerInputs::default();
    let mut new_players = Vec::new();
    PlayerInputController::create_player(
        &mut registration,
        Input::None,
        &mut dummy_player_inputs,
        &mut client.timeline,
        &mut client.entities.players,
        &client.entities.outfit_switchers,
        &mut new_players,
        None,
        None);
}

fn do_toggle_trailer_mode(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to trailer_mode, got {}", args.len());
        return;
    }

    client.trailer_mode = !client.trailer_mode;
    if (client.trailer_mode) {
        big!("Enabling trailer mode");
    }
    else {
        big!("Disabling trailer mode");
    }
}

fn do_game_config(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to game_config, got {}", args.len());
        return;
    }

    println!("{:#?}", client.timeline.top_state().rules_state.config);
    info!("{:?}", client.timeline.top_state().rules_state.config);
}

fn do_seed(args: &[&str], client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to seed, got {}", args.len());
        return;
    }

    println!("Seed: {}", client.seed);
    info!("Seed: {}", client.seed);
}

fn do_dump_controllers(args: &[&str], _client: &mut Client) {
    if (args.len() != 0) {
        err!("Expected no arguments to dump_controllers, got {}", args.len());
    }

    if (crate::input::using_steam_input()) {
        info!("Using Steam Input. Listing Connected Controllers:");
        unsafe {
            for i in 0..crate::steam::g_controller_count {
                let controller_id = crate::steam::g_connected_controllers[i];
                let input = steamworks::sys::SteamAPI_SteamInput_v006();
                let input_type = steamworks::sys::SteamAPI_ISteamInput_GetInputTypeForHandle(input, controller_id);
                info!("Controller Id {} - {:?}", controller_id, input_type);
            }
        }
    }

}