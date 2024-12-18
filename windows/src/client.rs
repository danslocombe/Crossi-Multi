use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, game, map::RowType, math::V2, player::{PlayerState, PlayerStatePublic}, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use crate::{dan_lerp, entities::{self, Entity, EntityContainer, EntityManager, Prop, PropController, Spectator}, hex_color, key_pressed, sprites, WHITE};
use froggy_rand::FroggyRand;

pub struct Client {
    pub exit: bool,
    pub timeline: Timeline,
    pub camera: Camera,

    pub prop_controller: PropController,
    pub entities: EntityManager,
    pub visual_effects: VisualEffects,

    pub screen_shader: crate::ScreenShader,

    pub big_text_controller: crate::bigtext::BigTextController,
}

impl Client {
    pub fn new(debug: bool) -> Self {
        let mut game_config = GameConfig::default();
        game_config.bypass_lobby = true;
        //game_config.minimum_players = 1;
        let mut timeline = Timeline::from_seed(game_config, "ac");
        timeline.add_player(PlayerId(1), game::Pos::new_coord(7, 7));
        timeline.add_player(PlayerId(2), game::Pos::new_coord(8, 7));

        let top = timeline.top_state();

        let mut entities = EntityManager::new();
        {
            let player_state = top.player_states.get(PlayerId(1)).unwrap().to_public(top.get_round_id(), top.time_us, &timeline.map);
            let eid = entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Player,
                pos: Pos::Absolute(V2::default())
            });
            let player_local = entities.players.get_mut(eid).unwrap();
            player_local.set_from(&player_state);
            player_local.sprite = "frog";
        }
        {
            let player_state = top.player_states.get(PlayerId(2)).unwrap().to_public(top.get_round_id(), top.time_us, &timeline.map);
            let eid = entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Player,
                pos: Pos::Absolute(V2::default())
            });
            let player_local = entities.players.get_mut(eid).unwrap();
            player_local.set_from(&player_state);
            player_local.sprite = "snake";
        }

        Self {
            exit: false,
            timeline,
            camera: Camera::new(),
            entities,
            prop_controller: PropController::new(),
            visual_effects: VisualEffects::default(),
            screen_shader: crate::ScreenShader::new(),
            big_text_controller: Default::default(),
        }
    }

    pub fn tick(&mut self) {
        let mut inputs = PlayerInputs::new();

        for player in self.entities.players.inner.iter_mut() {
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
            if (player.player_id.0 == 2) {
                if (key_pressed(raylib_sys::KeyboardKey::KEY_A)) {
                    input = game::Input::Left;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_D)) {
                    input = game::Input::Right;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_W)) {
                    input = game::Input::Up;
                }
                if (key_pressed(raylib_sys::KeyboardKey::KEY_S)) {
                    input = game::Input::Down;
                }
            }

            player.update_inputs(&self.timeline, &mut inputs, input);
        }

        self.timeline.tick(Some(inputs), TICK_INTERVAL_US);
        self.camera.tick(Some(self.timeline.top_state().get_rule_state()), &self.visual_effects);
        self.visual_effects.tick();

        let top = self.timeline.top_state();
        for local_player in self.entities.players.inner.iter_mut() {
            if let Some(state) = top.player_states.get(local_player.player_id) {
                let player_state = state.to_public(top.get_round_id(), top.time_us, &self.timeline.map);
                let alive_state = top.rules_state.fst.get_player_alive(local_player.player_id);
                local_player.tick(
                    &player_state,
                    alive_state,
                    &self.timeline,
                    &mut self.visual_effects,
                    &mut self.entities.dust,
                    &mut self.entities.bubbles,
                    &mut self.entities.corpses);
            }
        }

        self.prop_controller.tick(&top.rules_state, &self.timeline.map, &mut self.entities);

        // @TODO how do we model this?
        // Should cars be ephemeral actors?
        self.entities.cars.inner.clear();
        self.entities.lillipads.inner.clear();
        //let rows = self.timeline.map.get_row_view(top.get_round_id(), top.rules_state.fst.get_screen_y());
        let pub_cars = self.timeline.map.get_cars(top.get_round_id(), top.time_us);
        for pub_car in pub_cars {
            let car_id = self.entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Car,
                pos: Pos::Absolute(V2::new(pub_car.0 as f32 * 8.0, pub_car.1 as f32 * 8.0)),
            });
            let car = self.entities.cars.get_mut(car_id).unwrap();
            car.flipped = pub_car.2;
        }

        let pub_lillies = self.timeline.map.get_lillipads(top.get_round_id(), top.time_us);
        for pub_lilly in pub_lillies {
            let lilly_id = self.entities.create_entity(Entity {
                id: 0,
                entity_type: entities::EntityType::Lillipad,
                pos: Pos::Absolute(V2::new(pub_lilly.0 as f32 * 8.0, pub_lilly.1 as f32 * 8.0)),
            });
            let lilly = self.entities.lillipads.get_mut(lilly_id).unwrap();
        }

        self.big_text_controller.tick(&self.timeline);

        let camera_y_max = top.rules_state.fst.get_screen_y() as f32 + 200.0;
        self.entities.bubbles.prune_dead(camera_y_max);
        self.entities.props.prune_dead(camera_y_max);
        self.entities.dust.prune_dead(camera_y_max);
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

                if let RowType::Bushes(bush_descr) = &row.row_type {
                    for i in 0..=bush_descr.path_descr.wall_width {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    }
                    let hydrated = bush_descr.hydrate();
                }

                if let RowType::Path { wall_width } = row.row_type {
                    for i in 0..=wall_width {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                        sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    }
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

        raylib_sys::EndMode2D();

        self.big_text_controller.draw();

        {
            if (self.visual_effects.whiteout > 0) {
                raylib_sys::DrawRectangle(0, 0, 160, 160, WHITE);
            }
        }
    }
}

pub struct Camera {
    x: f32,
    y: f32,
    x_mod: f32,
    y_mod: f32,
    target_y: f32,
    t: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            x_mod: 0.0,
            y_mod: 0.0,
            target_y: 0.0,
            t: 0,
        }
    }

    pub fn tick(&mut self, m_rules_state: Option<&RulesState>, visual_effects: &VisualEffects) {
        self.t += 1;

        if let Some(rules_state) = m_rules_state {
            self.target_y = match &rules_state.fst {
                CrossyRulesetFST::Round(round_state) => {
                    round_state.screen_y as f32
                },
                CrossyRulesetFST::RoundCooldown(round_state) => {
                    round_state.round_state.screen_y as f32
                },
                _ => 0.0
            }
        }

        self.x = 0.0;
        self.y = dan_lerp(self.y, self.target_y * 8.0, 3.0);

        self.x_mod = self.x;
        self.y_mod = self.y;

        if (visual_effects.screenshake > 0.01) {
            //self.screen_shake_t -= 1.0;
            //let dir = *FroggyRand::new(self.t as u64).choose((), &[-1.0, 1.0]) as f32;
            //self.x = 1.0 / (visual_effects.screenshake + 1.0) * dir;

            let dir = (FroggyRand::new(self.t as u64).gen_unit(0) * 3.141 * 2.0) as f32;
            let mag = visual_effects.screenshake * 0.4;
            let offset = V2::norm_from_angle(dir) * mag;
            self.x_mod = self.x + offset.x;
            self.y_mod = self.y + offset.y;
        }
    }

    pub fn to_raylib(&self) -> raylib_sys::Camera2D {
        raylib_sys::Camera2D {
            offset: raylib_sys::Vector2::zero(),
            target: raylib_sys::Vector2 { x: self.x_mod, y: self.y_mod },
            rotation: 0.0,
            zoom: 1.0,
            
        }
    }
}

#[derive(Default)]
pub struct VisualEffects {
    pub whiteout: i32,
    pub screenshake: f32,
}

impl VisualEffects {
    pub fn whiteout(&mut self) {
        self.whiteout = self.whiteout.max(6);
    }

    pub fn screenshake(&mut self) {
        self.screenshake = self.screenshake.max(15.0);
    }

    pub fn tick(&mut self) {
        self.whiteout = (self.whiteout - 1).max(0);
        self.screenshake *= 0.85;
    }
}