#![allow(static_mut_refs)]
#![allow(unused_parens)]
#![allow(non_upper_case_globals)]

mod sprites;
mod console;
mod entities;

use std::{mem::MaybeUninit};

use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, game, map::RowType, player::{PlayerState, PlayerStatePublic}, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use entities::{Entity, EntityContainer, EntityManager, Prop, PropController, Spectator};
use froggy_rand::FroggyRand;
use sprites::draw_with_flip;

static mut c_string_temp_allocator: MaybeUninit<CStringAllocator> = MaybeUninit::uninit();
static mut c_string_leaky_allocator: MaybeUninit<CStringAllocator> = MaybeUninit::uninit();

pub fn c_str_temp(s: &str) -> *const i8 {
    unsafe {
        c_string_temp_allocator.assume_init_mut().alloc(s)
    }
}

pub fn c_str_leaky(s: &str) -> *const i8 {
    unsafe {
        c_string_leaky_allocator.assume_init_mut().alloc(s)
    }
}


const screen_width_f: f32 = 160.0;
const screen_height_f: f32 = 160.0;

fn main() {
    let args : Vec<_> = std::env::args().collect();
    println!("Parsed args: {:?}", args);
    let debug_param = args.iter().any(|x| x.eq_ignore_ascii_case("--debug"));
    if (debug_param) {
        println!("Running in debug mode");
    }

    unsafe {
        c_string_temp_allocator = MaybeUninit::new(CStringAllocator {
            buffers: Vec::new(),
        });
        c_string_leaky_allocator = MaybeUninit::new(CStringAllocator {
            buffers: Vec::new(),
        });

        raylib_sys::SetTraceLogLevel(raylib_sys::TraceLogLevel::LOG_WARNING as i32);
        raylib_sys::SetConfigFlags(raylib_sys::ConfigFlags::FLAG_WINDOW_RESIZABLE as u32);

        raylib_sys::InitWindow(1000, 800, c_str_leaky("Road Taods"));
        raylib_sys::SetTargetFPS(60);

        //raylib_sys::SetExitKey(raylib_sys::KeyboardKey::KEY_NULL as i32);

        console::init_console();
        crossy_multi_core::set_debug_logger(Box::new(console::QuakeConsoleLogger{}));

        sprites::init_sprites();

        raylib_sys::GuiLoadStyle(c_str_leaky("style_dark.rgs"));

        let framebuffer = raylib_sys::LoadRenderTexture(160, 160);

        let mut client = Client::new(debug_param);

        while !raylib_sys::WindowShouldClose() && !client.exit {
            let mapping_info = FrameBufferToScreenInfo::compute(&framebuffer.texture);
            client.tick();

            if key_pressed(raylib_sys::KeyboardKey::KEY_GRAVE) {
                console::toggle_open();
            };

            if key_pressed(raylib_sys::KeyboardKey::KEY_ESCAPE) {
                if (console::eating_input()) {
                    console::toggle_open();
                }
                else {
                    //let mut eaten = false;
                    //if let Some(x) = client.game.editor.as_mut() {
                    //    if let EditorMode::Dragging(s) = &x.mode {
                    //        if s.selected_entity.is_some() || s.selected_world_id.is_some() {
                    //            eaten = true;
                    //            x.mode = EditorMode::Dragging(DraggingState::default());
                    //        }
                    //    }
                    //}

                    //if (!eaten) {
                    //    client.exit = true;
                    //}
                }
            }

            console::tick(&mut client);

            {
                raylib_sys::BeginTextureMode(framebuffer);
                client.draw();
                raylib_sys::EndTextureMode();
            }
            //let texture = raylib_sys::LoadTextureFromImage(image);

            {
                raylib_sys::BeginDrawing();
                raylib_sys::ClearBackground(BLACK);
                raylib_sys::DrawTexturePro(framebuffer.texture, mapping_info.source, mapping_info.destination, raylib_sys::Vector2{ x: 0.0, y: 0.0 }, 0.0, WHITE);

                /*
                if let Some(editor) = client.game.editor.as_mut() {
                    gui_editor::draw_gui(&mut client.game.local_simulation.simulation, editor);

                    raylib_sys::DrawFPS(raylib_sys::GetScreenWidth() - 100, 20);
                }
                */

                console::draw(&client);

                raylib_sys::EndDrawing();
            }

            //raylib_sys::UnloadImage(image);
            //raylib_sys::UnloadTexture(texture);

            c_string_temp_allocator.assume_init_mut().clear();
        }
    }
}

struct CStringAllocator {
    buffers: Vec<(usize, Box<[i8]>)>,
}

impl CStringAllocator {
    pub fn alloc(&mut self, s: &str) -> *const i8 {
        let s_bytes = s.as_bytes();
        for (free_start, buffer) in &mut self.buffers {
            if (*free_start + s_bytes.len() + 1 < buffer.len()) {
                // There is space!
                //println!("Allocated from existing");
                return Self::copy_into_buffer(free_start, buffer, s_bytes);
            }
        } 

        // No space, allocate new
        let size = (s_bytes.len() + 1).max(1024);
        let mut new_buf = Vec::new();
        new_buf.resize(size, 0);
        let mut new_buf = new_buf.into_boxed_slice();

        let mut free_start = 0;
        let ret = Self::copy_into_buffer(&mut free_start, &mut new_buf, s_bytes);

        self.buffers.push((free_start, new_buf));
        ret
    }

    fn copy_into_buffer(free_start: &mut usize, buffer: &mut Box<[i8]>, s_bytes: &[u8]) -> *const i8 {
        // @Perf cast and memcpy
        for (i, b) in s_bytes.iter().enumerate() {
            buffer[*free_start + i] = (*b) as i8;
        }

        buffer[*free_start + s_bytes.len()] = 0;

        let ret = unsafe { buffer.as_ptr().byte_add(*free_start) };
        *free_start += s_bytes.len() + 1;

        ret
    }

    pub fn clear(&mut self) {
        for (free_start, _buffer) in &mut self.buffers {
            *free_start = 0;
        }
    }
}

pub struct FrameBufferToScreenInfo {
    mouse_x: i32,
    mouse_y: i32,
    source: raylib_sys::Rectangle,
    destination: raylib_sys::Rectangle,
}

impl FrameBufferToScreenInfo {
    pub unsafe fn compute(framebuffer: &raylib_sys::Texture) -> Self {
        let rl_screen_width_f = raylib_sys::GetScreenWidth() as f32;
        let rl_screen_height_f = raylib_sys::GetScreenHeight() as f32;
        let screen_scale = (rl_screen_width_f / screen_width_f).min(rl_screen_height_f / screen_height_f);

        let source_width = framebuffer.width as f32;

        // This minus is needed to avoid flipping the rendering (for some reason)
        let source_height = -(framebuffer.height as f32);
        //let source_height = (framebuffer.height as f32);

        let destination = raylib_sys::Rectangle{
            x: (rl_screen_width_f - screen_width_f * screen_scale) * 0.5,
            y: (rl_screen_height_f - screen_height_f * screen_scale) * 0.5,
            width: screen_width_f * screen_scale,
            height: screen_height_f * screen_scale,
        };

        // TODO move this out
        // HAcky we put here as we have the remapping maths
        // Makes the mouse pos a frame out but should be fine right?
        let mouse_screen = raylib_sys::GetMousePosition();

        let source = raylib_sys::Rectangle{ x: 0.0, y: 0.0, width: source_width, height: source_height };

        return Self{
            mouse_x: ((mouse_screen.x - destination.x) / screen_scale) as i32,
            mouse_y: ((mouse_screen.y - destination.y) / screen_scale) as i32,
            source: source,
            destination: destination,
        };
    }
}

pub struct Client {
    exit: bool,
    timeline: Timeline,
    camera: Camera,

    local_players: Vec<PlayerLocal>,
    prop_controller: PropController,
    entities: EntityManager,
}

impl Client {
    pub fn new(debug: bool) -> Self {
        let mut game_config = GameConfig::default();
        game_config.bypass_lobby = true;
        game_config.minimum_players = 1;
        let mut timeline = Timeline::from_seed(game_config, "ac");
        timeline.add_player(PlayerId(1), game::Pos::new_coord(7, 7));

        let mut local_players = Vec::new();
        let top = timeline.top_state();
        let local_player_0 = PlayerLocal::new(&top.player_states.get(PlayerId(1)).unwrap().to_public(top.get_round_id(), top.time_us, &timeline.map));
        local_players.push(local_player_0);

        Self {
            exit: false,
            timeline,
            camera: Camera::new(),
            local_players,
            entities: EntityManager {
                next_id: 1,
                props: EntityContainer::<Prop> {
                    entity_type: entities::EntityType::Prop,
                    inner: Default::default(),
                },
                spectators: EntityContainer::<Spectator> {
                    entity_type: entities::EntityType::Spectator,
                    inner: Default::default(),
                },
            },
            prop_controller: PropController::new(),
        }
    }

    pub fn tick(&mut self) {
        let mut inputs = PlayerInputs::new();

        for player in self.local_players.iter_mut() {
            let mut input = Input::None;
            if (player.player_id.0 == 1) {
                if (key_pressed(raylib_sys::KeyboardKey::KEY_LEFT)) {
                    input = game::Input::Left;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT)) {
                    input = game::Input::Right;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_UP)) {
                    input = game::Input::Up;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_DOWN)) {
                    input = game::Input::Down;
                }
            }

            player.update_inputs(&self.timeline, &mut inputs, input);
        }

        self.timeline.tick(Some(inputs), TICK_INTERVAL_US);
        self.camera.tick(Some(self.timeline.top_state().get_rule_state()));

        let top = self.timeline.top_state();
        for local_player in self.local_players.iter_mut() {
            if let Some(state) = top.player_states.get(local_player.player_id) {
                local_player.tick(&state.to_public(top.get_round_id(), top.time_us, &self.timeline.map));
            }
        }

        self.prop_controller.tick(&top.rules_state, &self.timeline.map, &mut self.entities);
    }

    pub unsafe fn draw(&mut self) {
        let top = self.timeline.top_state();

        raylib_sys::BeginMode2D(self.camera.to_raylib());

        {
            // Draw background
            const bg_fill_col: raylib_sys::Color = hex_color("3c285d".as_bytes());
            raylib_sys::ClearBackground(bg_fill_col);
            const grass_col_0: raylib_sys::Color = hex_color("c4e6b5".as_bytes());
            const grass_col_1: raylib_sys::Color = hex_color("d1bfdb".as_bytes());
            const river_col_0: raylib_sys::Color = hex_color("6c6ce2".as_bytes());
            const river_col_1: raylib_sys::Color = hex_color("5b5be7".as_bytes());
            const road_col_0: raylib_sys::Color = hex_color("646469".as_bytes());
            const road_col_1: raylib_sys::Color = hex_color("59595d".as_bytes());

            let screen_y = top.rules_state.fst.get_screen_y();
            let round_id = top.get_round_id();
            let rows = self.timeline.map.get_row_view(round_id, screen_y);

            for row_with_y in rows {
                let row = row_with_y.row;
                let y = row_with_y.y;

                let (col_0, col_1) = match row.row_type {
                    RowType::River(_) => {
                        (river_col_0, river_col_1)
                    },
                    RowType::Road(_) => {
                        (road_col_0, road_col_1)
                    },
                    _ => {
                        (grass_col_0, grass_col_1)
                    },
                };

                for x in (0..160 / 8) {
                    let col = if (x + y) % 2 == 0 {
                        col_0
                    }
                    else {
                        col_1
                    };

                    raylib_sys::DrawRectangle(x * 8, y * 8, 8, 8, col);
                }

                if let RowType::Stands = row.row_type {
                    sprites::draw("block", 0, 6.0 * 8.0, y as f32 * 8.0);
                    sprites::draw("block", 0, (19.0 - 6.0) * 8.0, y as f32 * 8.0);
                }

                if let RowType::StartingBarrier = row.row_type {
                    for i in 0..=6 {
                        sprites::draw("block", 0, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("block", 0, (19.0 - i as f32) * 8.0, y as f32 * 8.0);
                    }

                    if let CrossyRulesetFST::RoundWarmup(_) = &top.rules_state.fst {
                        for i in 7..(20-7) {
                            sprites::draw("barrier", 0, i as f32 * 8.0, y as f32 * 8.0);
                        }
                    }
                }
            }
        }

        {
            // @Perf keep some list and insertion sort
            let mut all_entities = Vec::new();
            self.entities.extend_all_depth(&mut all_entities);

            all_entities.sort_by_key(|(_, depth)| *depth);

            for (e, _) in all_entities {
                self.entities.draw_entity(e);
            }
        }

        for local_player in &self.local_players {
            let shadow = sprites::get_sprite("shadow");
            raylib_sys::DrawTexture(
                shadow[0],
                (local_player.x * 8.0) as i32,
                (local_player.y * 8.0) as i32,
                WHITE);

            sprites::draw("shadow", 0, local_player.x * 8.0, local_player.y * 8.0);
            sprites::draw_with_flip("frog", local_player.frame_id as usize, local_player.x * 8.0, local_player.y * 8.0 - 2.0, local_player.x_flip);
        }

        raylib_sys::EndMode2D();
    }
}

pub struct Camera {
    x: f32,
    y: f32,
    target_y: f32,
    screen_shake_t: f32,
    t: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            target_y: 0.0,
            screen_shake_t: 0.0,
            t: 0,
        }
    }

    pub fn tick(&mut self, m_rules_state: Option<&RulesState>) {
        self.t += 1;

        if let Some(rules_state) = m_rules_state {
            self.target_y = match &rules_state.fst {
                CrossyRulesetFST::Round(round_state) => {
                    round_state.screen_y as f32
                },
                _ => 0.0
            }
        }

        self.y = dan_lerp(self.y, self.target_y * 8.0, 3.0);
        if (self.screen_shake_t > 0.0) {
            self.screen_shake_t -= 1.0;
            let dir = *FroggyRand::new(self.t as u64).choose((), &[-1.0, 1.0]) as f32;
            self.x = 1.0 / (self.screen_shake_t + 1.0) * dir;
        }
        else {
            self.x = 0.0;
        }
    }

    pub fn to_raylib(&self) -> raylib_sys::Camera2D {
        raylib_sys::Camera2D {
            offset: raylib_sys::Vector2::zero(),
            target: raylib_sys::Vector2 { x: self.x, y: self.y },
            rotation: 0.0,
            zoom: 1.0,
            
        }
    }
}

pub struct PlayerLocalSource {

}

#[derive(Debug)]
pub struct PlayerLocal {
    player_id: PlayerId,
    x: f32,
    y: f32,
    moving: bool,
    x_flip: bool,
    frame_id: i32,
    buffered_input: Input,
}

const MOVE_T : i32 = 7 * (1000 * 1000 / 60);
const PLAYER_FRAME_COUNT: i32 = 5;

impl PlayerLocal {
    pub fn new(state: &PlayerStatePublic) -> Self {
        Self {
            player_id: PlayerId(state.id),
            x: state.x as f32,
            y: state.y as f32,
            moving: false,
            x_flip: false,
            frame_id: 0,
            buffered_input: Input::None,
        }
    }

    pub fn update_inputs(&mut self, timeline: &Timeline, player_inputs: &mut PlayerInputs, input: Input) {
        if (input != Input::None) {
            self.buffered_input = input;

        }

        if (input == Input::Left) {
            self.x_flip = true;
        }

        if (input == Input::Right) {
            self.x_flip = false;
        }

        let top = timeline.top_state();
        if (top.player_states.get(self.player_id).unwrap().can_move()) {
            player_inputs.set(self.player_id, self.buffered_input);
            self.buffered_input = Input::None;
        }
    }

    pub fn tick(&mut self, player_state: &PlayerStatePublic) {
        let x0 = player_state.x as f32;
        let y0 = player_state.y as f32;

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        if (player_state.moving) {
            let lerp_t = 1.0 - (player_state.remaining_move_dur as f32 / MOVE_T as f32);

            let x1 = player_state.t_x as f32;
            let y1 = player_state.t_y as f32;

            self.frame_id = (self.frame_id + 1);
            if (self.frame_id >= PLAYER_FRAME_COUNT) {
                self.frame_id = PLAYER_FRAME_COUNT - 1;
            }

            x = x0 + lerp_t * (x1 - x0);
            y = y0 + lerp_t * (y1 - y0);
        }
        else {
            //let new_p = lerp_snap(self.x, self.y, x0, y0);
            x = x0;
            y = y0;

            let delta = 0.01;
            if (diff(x, self.x) > delta || diff(y, self.y) > delta) {
                self.frame_id = (self.frame_id + 1) % PLAYER_FRAME_COUNT;
            }
            else {
                self.frame_id = 0;
            }
        }

        self.x = x;
        self.y = y;
        self.moving = player_state.moving;
    }
}

pub struct PlayerSkin {
}

fn dan_lerp(x0 : f32, x : f32, k : f32) -> f32 {
    (x0 * (k-1.0) + x) / k
}

fn diff(x: f32, y: f32) -> f32 {
    (x - y).abs()
}

fn lerp_snap(x0 : f32, y0 : f32, x1 : f32, y1 : f32) -> raylib_sys::Vector2
{
    let k = 4.0;
    let mut x = dan_lerp(x0, x1, k);
    let mut y = dan_lerp(y0, y1, k);

    let dist = ((x - x1) * (x - x1) + (y - y1) * (y - y1)).sqrt();

    let snap_dir_small = 0.15;
    let snap_dir_large = 3.0;

    if (dist < snap_dir_small || dist > snap_dir_large) {
        x = x1;
        y = y1;
    }

    raylib_sys::Vector2 {
        x : x,
        y : y,
    }
}

fn key_down(k: raylib_sys::KeyboardKey) -> bool {
    unsafe {
        raylib_sys::IsKeyDown(k as i32)
    }
}

fn key_pressed(k: raylib_sys::KeyboardKey) -> bool {
    unsafe {
        raylib_sys::IsKeyPressed(k as i32)
    }
}

fn mouse_button_down(mb: raylib_sys::MouseButton) -> bool {
    unsafe {
        raylib_sys::IsMouseButtonDown(mb as i32)
    }
}

fn mouse_button_pressed(mb: raylib_sys::MouseButton) -> bool {
    unsafe {
        raylib_sys::IsMouseButtonPressed(mb as i32)
    }
}

pub const WHITE: raylib_sys::Color = hex_color("fff1e8".as_bytes());
pub const BLACK: raylib_sys::Color = hex_color("000000".as_bytes());

pub const BLUE: raylib_sys::Color = hex_color("1d2b53".as_bytes());
pub const PURPLE: raylib_sys::Color = hex_color("7e2553".as_bytes());
pub const GREEN: raylib_sys::Color = hex_color("00e436".as_bytes());
pub const GREEN_TREE: raylib_sys::Color = hex_color("80ff80".as_bytes());

pub const BROWN: raylib_sys::Color = hex_color("ab5236".as_bytes());
pub const DARKGREY: raylib_sys::Color = hex_color("5f574f".as_bytes());
pub const GREY: raylib_sys::Color = hex_color("c2c3c7".as_bytes());

pub const RED: raylib_sys::Color = hex_color("ff004d".as_bytes());
pub const ORANGE: raylib_sys::Color = hex_color("ffa300".as_bytes());
pub const YELLOW: raylib_sys::Color = hex_color("ffec27".as_bytes());

pub const SEA: raylib_sys::Color = hex_color("29adff".as_bytes());
pub const LILAC: raylib_sys::Color = hex_color("83769c".as_bytes());
pub const PINK: raylib_sys::Color = hex_color("ff77a8".as_bytes());
pub const BEIGE: raylib_sys::Color = hex_color("ffccaa".as_bytes());
pub const BEIGE_TREE: raylib_sys::Color = hex_color("ffeed5".as_bytes());

const fn hex_color(s: &[u8]) -> raylib_sys::Color {
    //assert_eq!(s.len(), 6);
    let r = parse_u8_hex([s[0], s[1]]);
    let g = parse_u8_hex([s[2], s[3]]);
    let b = parse_u8_hex([s[4], s[5]]);

    raylib_sys::Color {
        r,
        g,
        b,
        a: 255,
    }
}

const fn parse_u8_hex(s: [u8;2]) -> u8 {
    //assert_eq!(s.len(), 2);
    parse_char_hex(s[0]) * 16 + parse_char_hex(s[1])
}

const fn parse_char_hex(c : u8) -> u8 {
    if (c >= b'0' && c <= b'9') {
        return c - b'0';
    }

    if (c >= b'a' && c <= b'f') {
        return c - b'a' + 10;
    }

    if (c >= b'A' && c <= b'F') {
        return c - b'A' + 10;
    }

    panic!("Unexpected char in hex number")
}

pub fn lerp_color_rgba(c0: raylib_sys::Color, c1: raylib_sys::Color, t: f32) -> raylib_sys::Color {
    let rr = (1.0 - t) * c0.r as f32 + t * c1.r as f32;
    let gg = (1.0 - t) * c0.g as f32 + t * c1.g as f32;
    let bb = (1.0 - t) * c0.b as f32 + t * c1.b as f32;
    let aa = (1.0 - t) * c0.a as f32 + t * c1.a as f32;
    raylib_sys::Color {
        r: rr as u8,
        g: gg as u8,
        b: bb as u8,
        a: aa as u8,
    }
}
