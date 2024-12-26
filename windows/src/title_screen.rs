use crossy_multi_core::{math::V2};

use crate::{rope::{self, ConstantForce, RopeWorld}, sprites, to_vector2};

pub struct TitleScreen {
    t: i32,
    text_pos: V2,
    left_curtain: Curtain,
    right_curtain: Curtain,

    pub draw_bg_tiles: bool,
}

impl Default for TitleScreen {
    fn default() -> Self {

        Self {
            t: 0,
            text_pos: V2::new(10.0, 60.0),
            left_curtain: Curtain::new(V2::new(-10.0, 20.0), V2::new(90.0, 20.0)),
            right_curtain: Curtain::new(V2::new(170.0, 20.0), V2::new(80.0, 20.0)),
            draw_bg_tiles: false,
        }
    }
}

impl TitleScreen {
    pub fn tick(&mut self) -> bool {
        self.t += 1;

        self.left_curtain.tick();
        self.right_curtain.tick();

        if self.t > 80 {
            self.text_pos = crate::dan_lerp_v2(self.text_pos, V2::new(10.0, -50.0), 15.);
        }

        self.draw_bg_tiles = self.t > 100;

        self.t < 180
    }

    pub fn draw(&mut self) {
        unsafe {
            let fade_out_t = ((self.t as f32 - 80.0) / 40.0).clamp(0.0, 1.0);
            let mut col = crate::BLACK;
            col.a = (255.0 * (1.0 - fade_out_t)) as u8;
            raylib_sys::DrawRectangle(0, 0, 200, 200, col);
        }
        self.left_curtain.draw();
        self.right_curtain.draw();
        unsafe {
            //raylib_sys::DrawText(crate::c_str_leaky("test"), 10, 10, 26, crate::BLACK);
        }

        sprites::draw("champion", 0, self.text_pos.x, self.text_pos.y);
    }
}

struct Curtain {
    t: i32,
    rope_world: RopeWorld,

    node_top_corner_wall: usize,
    node_top_corner_center: usize,

    on_left: bool,

    grid: Vec<Vec<usize>>,
}

impl Curtain {
    pub fn new(top_corner_wall: V2, top_corner_center: V2) -> Self {
        let mut rope_world = RopeWorld::default();
        rope_world.forces.push(Box::new(ConstantForce {
            force: V2::new(0.0, 0.03),
        }));

        let node_top_corner_wall = rope_world.add_node_p(top_corner_wall);
        rope_world.get_node_mut(node_top_corner_wall).node_type = rope::NodeType::Fixed;

        let node_top_corner_center = rope_world.add_node_p(top_corner_center);
        rope_world.get_node_mut(node_top_corner_center).node_type = rope::NodeType::Fixed;

        /*
        let delta = top_corner_center - top_corner_wall;
        let n = 8;
        let mut prev_id = node_top_corner_wall;
        for i in 0..n {
            let p = top_corner_wall + delta * (((i + 1) as f32) * 1.0/((n+1) as f32));
            let id = rope_world.add_node_p(p);
            rope_world.add_rope(prev_id, id);
            prev_id = id;
        }

        rope_world.add_rope(prev_id, node_top_corner_center);
        */

        let mut grid: Vec<Vec<usize>> = Vec::new();

        let width = 12;
        let height = 12;
        let x_offset = top_corner_center - top_corner_wall;
        let y_offset = V2::new(0.0, 120.);
        for y in 0..height {
            //let top_left = top_corner_center + y_offset * (((y + 1) as f32) * 1.0/((height+1) as f32));
            let top_left = top_corner_wall + y_offset * (((y) as f32) * 1.0/((height) as f32));

            let mut row = Vec::new();
            for x in 0..width {
                let mut created = false;
                if (y == 0) {
                    if (x == 0) {
                        row.push(node_top_corner_wall);
                        created = true;
                    }
                    if (x == width - 1) {
                        row.push(node_top_corner_center);
                        created = true;
                    }
                }

                if !created {
                    //let p = top_left + x_offset * (((x + 1) as f32) * 1.0/((width+1) as f32));
                    let p = top_left + x_offset * (((x) as f32) * 1.0/((width - 1) as f32));
                    //println!("Creating {}", p);
                    let id = rope_world.add_node_p(p);
                    row.push(id);
                }

                let id = *row.last().unwrap();

                if y > 0 {
                    let above = grid[y - 1][x];
                    rope_world.add_rope(above, id);
                }

                if x > 0 {
                    let left = row[row.len() - 2];
                    rope_world.add_rope(left, id);
                }
            }

            grid.push(row);
        }

        for i in 0..width {
            if i % 2 == 0 {
                let id = grid[0][i];
                rope_world.get_node_mut(id).node_type = rope::NodeType::Fixed;
            }
        }

        Self {
            t: 0,
            rope_world,
            node_top_corner_wall,
            node_top_corner_center,

            grid,

            on_left: top_corner_wall.x < top_corner_center.x,
        }
    }

    pub fn tick(&mut self) {
        self.t += 1;

        if self.t > 60 {
            let w = self.grid[0].len();
            for x in 0..w {
                if x % 2 == 0 || x == w-1 {
                    let id = self.grid[0][x];
                    let pos = self.rope_world.get_node(id).pos;
                    let wall = self.rope_world.get_node(self.node_top_corner_wall).pos;
                    let new_pos = crate::dan_lerp_v2(pos, wall, 35.0);
                    self.rope_world.get_node_mut(id).pos = new_pos;
                }
            }
        }

        self.rope_world.tick(1.0);
    }

    pub fn draw(&self) {
        /*
        for rope in &self.rope_world.ropes {
            unsafe {
                let from = self.rope_world.nodes[rope.from].pos;
                let to = self.rope_world.nodes[rope.to].pos;
                raylib_sys::DrawLineV(to_vector2(from), to_vector2(to), crate::PINK);
            }
        }
        */

        /*
        for node in &self.rope_world.nodes {
            unsafe {
                raylib_sys::DrawCircleLinesV(to_vector2(node.pos), 2.0, crate::PINK);
            }
        }
        */

        let h = self.grid.len();
        let w = self.grid[0].len();
        for y in 1..h {
            for x in 1..w {
                let top_left = self.rope_world.get_node(self.grid[y-1][x-1]).pos;
                let top_right = self.rope_world.get_node(self.grid[y-1][x]).pos;
                let bot_left = self.rope_world.get_node(self.grid[y][x-1]).pos;
                let bot_right = self.rope_world.get_node(self.grid[y][x]).pos;

                let col = if x % 2 == 0 {
                    crate::PURPLE
                }
                else {
                    crate::RED
                };

                if (self.on_left) {
                    unsafe {
                        raylib_sys::DrawTriangle(
                            to_vector2(top_left),
                            to_vector2(bot_left),
                            to_vector2(top_right),
                            col);
                        raylib_sys::DrawTriangle(
                            to_vector2(bot_right),
                            to_vector2(top_right),
                            to_vector2(bot_left),
                            col);
                    }
                }
                else {
                    unsafe {
                        raylib_sys::DrawTriangle(
                            to_vector2(top_left),
                            to_vector2(top_right),
                            to_vector2(bot_left),
                            col);
                        raylib_sys::DrawTriangle(
                            to_vector2(bot_right),
                            to_vector2(bot_left),
                            to_vector2(top_right),
                            col);
                    }
                }
            }
        }
    }
}