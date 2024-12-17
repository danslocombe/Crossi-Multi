#![allow(static_mut_refs)]
#![allow(unused_parens)]
#![allow(non_upper_case_globals)]

mod sprites;
mod console;
mod entities;
mod client;
mod bigtext;

use std::{mem::MaybeUninit};

use client::Client;

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

        raylib_sys::InitWindow(1000, 800, c_str_leaky("Road Toads"));
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


                if (client.screen_shader.enabled) {
                    client.screen_shader.iTime += 1;

                    let iTime_ptr: *const i32 = std::ptr::from_ref(&client.screen_shader.iTime);
                    raylib_sys::SetShaderValue(client.screen_shader.shader, client.screen_shader.shader_iTime_loc, iTime_ptr.cast(), raylib_sys::ShaderUniformDataType::SHADER_UNIFORM_INT as i32);
                    let amp = client.visual_effects.screenshake * 2.0;// / 16.0;
                    let amp_ptr: *const f32 = std::ptr::from_ref(&amp);
                    raylib_sys::SetShaderValue(client.screen_shader.shader, client.screen_shader.shader_amp_loc, amp_ptr.cast(), raylib_sys::ShaderUniformDataType::SHADER_UNIFORM_FLOAT as i32);
                    raylib_sys::BeginShaderMode(client.screen_shader.shader);
                }

                raylib_sys::DrawTexturePro(framebuffer.texture, mapping_info.source, mapping_info.destination, raylib_sys::Vector2{ x: 0.0, y: 0.0 }, 0.0, WHITE);

                if (client.screen_shader.enabled) {
                    raylib_sys::EndShaderMode();
                }
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

struct ScreenShader {
    enabled: bool,
    shader: raylib_sys::Shader,
    iTime: i32,
    shader_iTime_loc: i32,
    shader_amp_loc: i32,
}

impl ScreenShader {
    pub fn new() -> Self {
        unsafe {
            let shader = raylib_sys::LoadShader(std::ptr::null(), c_str_leaky("shaders/pause.fs"));
            let shader_iTime_loc = raylib_sys::GetShaderLocation(shader, c_str_leaky("iTime"));
            let shader_amp_loc = raylib_sys::GetShaderLocation(shader, c_str_leaky("amp"));
            Self {
                enabled: true,
                shader,
                iTime: 0,
                shader_iTime_loc,
                shader_amp_loc,
            }
        }
    }
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

    let snap_dir_small = 8.0 * 0.15;
    let snap_dir_large = 8.0 * 3.0;

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

pub fn ease_in_quad(x: f32) -> f32 {
    return 1.0 - (1.0 - x) * (1.0 - x);
}