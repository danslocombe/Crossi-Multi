use std::{mem::MaybeUninit};

use crossy_multi_core::{ring_buffer::RingBuffer, timeline::Timeline, DebugLogger, PlayerId, Pos};

use crate::Client;

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
            name: "shader".to_owned(),
            lambda: Box::new(do_toggle_shader),
        });
        g_console = MaybeUninit::new(Console::new(command_set));
    }
}

pub fn info(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(s.to_owned(), LineType::Info);
    }
}

pub fn big(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(s.to_owned(), LineType::Big);
    }
}

pub fn err(s: &str) {
    unsafe {
        g_console.assume_init_mut().write_with_type(format!("Error: {}", s), LineType::Error);
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
                let size = unsafe { raylib_sys::MeasureText(str, 28)};

                let yy: i32 = screen_height / 3;
                let xx: i32 = screen_width / 2 - size / 2;

                unsafe {
                    raylib_sys::DrawText(str, xx, yy, 28, crate::WHITE);
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

fn do_new(args: &[&str], client: &mut Client) {
    if (args.len() > 1) {
        err(&format!("Expected 0 or 1 argument to new, got {}", args.len()));
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

    big(&format!("New Level Seed '{}'", seed));
    let new_game_id = client.timeline.top_state().rules_state.game_id;
    let mut config = client.timeline.top_state().rules_state.config.clone();
    config.bypass_lobby = true;
    client.timeline = Timeline::from_seed(config, &seed);
    client.timeline.set_game_id(new_game_id);
    client.timeline.add_player(PlayerId(1), Pos::new_coord(7, 7));
    client.timeline.add_player(PlayerId(2), Pos::new_coord(8, 7));
}

fn do_toggle_shader(args: &[&str], client: &mut Client) {
    if (args.len() > 0) {
        err(&format!("Expected no arguments to 'shader' got {}", args.len()));
        return;
    }

    client.screen_shader.enabled = !client.screen_shader.enabled;
}