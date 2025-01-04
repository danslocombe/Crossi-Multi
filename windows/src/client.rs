use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, map::RowType, math::V2, ring_buffer::RingBuffer, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use serde::{Deserialize, Serialize};
use crate::{audio, c_str_temp, dan_lerp, entities::{self, create_dust, Entity, EntityContainer, EntityManager, OutfitSwitcher, PropController}, gamepad_pressed, hex_color, key_pressed, lerp_color_rgba, player_local::{PlayerInputController, PlayerLocal, Skin}, rope::NodeType, sprites, title_screen::{self, ActorController, TitleScreen}, to_vector2, BLACK, WHITE};
use froggy_rand::FroggyRand;

pub struct Client {
    pub debug: bool,

    // Disable some gui / teaching elements.
    pub trailer_mode: bool,

    pub exit: bool,
    pub seed: String,

    pub pause: Option<Pause>,

    pub title_screen: Option<TitleScreen>,

    pub timeline: Timeline,
    pub camera: Camera,

    pub prop_controller: PropController,
    pub entities: EntityManager,
    pub visual_effects: VisualEffects,

    pub screen_shader: crate::ScreenShader,

    pub big_text_controller: crate::bigtext::BigTextController,
    pub player_input_controller: PlayerInputController,

    prev_rules: Option<CrossyRulesetFST>,

    actor_controller: ActorController,

    bg_music: TitleBGMusic,

    pub frame_ring_buffer: RingBuffer<Option<Vec<u8>>>,
    pub recording_gif: bool,
    pub recording_gif_name: String,
}

pub const grass_col_0: raylib_sys::Color = hex_color("c4e6b5".as_bytes());
pub const grass_col_1: raylib_sys::Color = hex_color("d1bfdb".as_bytes());
pub const river_col_0: raylib_sys::Color = hex_color("6c6ce2".as_bytes());
pub const river_col_1: raylib_sys::Color = hex_color("5b5be7".as_bytes());
pub const road_col_0: raylib_sys::Color = hex_color("646469".as_bytes());
pub const road_col_1: raylib_sys::Color = hex_color("59595d".as_bytes());
pub const icy_col_0: raylib_sys::Color = hex_color("cbdbfc".as_bytes());
pub const icy_col_1: raylib_sys::Color = hex_color("9badb7".as_bytes());

impl Client {
    pub fn new(debug: bool, seed: &str) -> Self {
        println!("Initialising, Seed {}", seed);
        let mut game_config = GameConfig::default();
        //game_config.bypass_lobby = true;
        //game_config.minimum_players = 1;
        let timeline = Timeline::from_seed(game_config, seed);
        let entities = EntityManager::new();

        let mut actor_controller = ActorController::default();
        //actor_controller.spawn_positions_grid.push((V2::new(20.0, 17.0), false));
        actor_controller.spawn_positions_grid.push((V2::new(0.0, 3.0), true));

        Self {
            debug,
            trailer_mode: false,
            seed: seed.to_owned(),
            exit: false,
            timeline,
            camera: Camera::new(),
            entities,
            prop_controller: PropController::new(),
            visual_effects: VisualEffects::default(),
            screen_shader: crate::ScreenShader::new(),
            big_text_controller: Default::default(),
            player_input_controller: PlayerInputController::default(),
            prev_rules: Default::default(),
            pause: None,
            title_screen: Some(TitleScreen::default()),
            actor_controller,
            bg_music: TitleBGMusic::new(),
            frame_ring_buffer: RingBuffer::new_with_value(60 * 60, None),
            recording_gif: false,
            recording_gif_name: String::default(),
        }
    }

    pub fn goto_loby_seed(&mut self, seed: &str, bypass_lobby: Option<bool>) {
        let mut config = self.timeline.top_state().rules_state.config.clone();
        if let Some(bl) =  bypass_lobby {
            config.bypass_lobby = bl;
        }

        if (!seed.is_empty()) {
            self.seed = seed.to_owned();
        }

        self.timeline = Timeline::from_seed(config, &self.seed);

        self.player_input_controller = PlayerInputController::default();
        self.entities.clear_round_entities();
        self.entities.players.inner.clear();

        self.pause = None;

        self.visual_effects.noise();
        self.visual_effects.whiteout();
        self.visual_effects.screenshake();
        audio::play("car");
    }

    pub fn tick(&mut self) {
        self.bg_music.tick();
        self.visual_effects.tick();

        if let Some(pause) = self.pause.as_mut() {
            match pause.tick(&mut self.visual_effects) {
                PauseResult::Nothing => {},
                PauseResult::Unpause => {
                    self.pause = None;
                },
                PauseResult::Exit => {
                    self.exit = true;
                }
                PauseResult::Lobby => {
                    self.goto_loby_seed(&crate::shitty_rand_seed(), Some(false));
                },
                PauseResult::Feedback => {
                    // @TODO
                    self.pause = None;
                },
            }

            return;
        }

        let mut just_left_title = false;
        if let Some(title) = self.title_screen.as_mut() {
            if (title.t - title.goto_next_t.unwrap_or(title.t) > 10) {
                self.bg_music.mode = BGMusicMode::FadingOutLowpass;
            }
            else {
                self.bg_music.mode = BGMusicMode::Lowpassed;
            }

            if !title.tick(&mut self.visual_effects, self.bg_music.current_time_in_secs()) {
                self.title_screen = None;
                just_left_title = true;
            }
            else {
                // @Hacky
                self.camera.k = 100.0;
                self.camera.y = -200.0;
                self.camera.y_mod = -200.0;
                self.camera.target_y = -200.0;
                return;
            }
        }
        else {
            if let CrossyRulesetFST::Lobby { .. } = &self.timeline.top_state().rules_state.fst {
                self.bg_music.mode = BGMusicMode::Normal;
            }
            else {
                self.bg_music.mode = BGMusicMode::Paused;
            }
        }

        let (inputs, new_players) = self.player_input_controller.tick(&mut self.timeline, &mut self.entities.players, &self.entities.outfit_switchers);
        self.timeline.tick(Some(inputs), TICK_INTERVAL_US);

        let transitions = {
            let top = self.timeline.top_state();
            StateTransition::new(&top.rules_state.fst, &self.prev_rules)
        };

        //if (transitions.into_round) {
            //self.visual_effects.whiteout();
        //}

        if (transitions.into_lobby && !just_left_title) {
            self.visual_effects.noise();
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();
        }

        if (transitions.leaving_lobby) {
            self.visual_effects.noise();
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();
            audio::play("car");
        }

        if (transitions.into_round_warmup) {
            self.visual_effects.noise();
        }

        if (!new_players.is_empty())
        {
            audio::play("join");
            audio::play("car");
            self.visual_effects.whiteout();
            self.visual_effects.screenshake();

            for new in new_players.iter() {
                if let Some(local) = self.entities.players.inner.iter().find(|x| x.player_id == *new) {
                    if let Some(cid) = local.controller_id {
                        self.visual_effects.set_gamepad_vibration(cid);
                    }
                }
            }
        }

        self.camera.tick(Some(self.timeline.top_state().get_rule_state()), &self.visual_effects, &transitions);

        let top = self.timeline.top_state();
        let mut to_remove = Vec::new();
        for local_player in self.entities.players.inner.iter_mut() {
            if let Some(state) = top.player_states.get(local_player.player_id) {
                let player_state = state.to_public(top.get_round_id(), top.time_us, &self.timeline.map, &top.rules_state.fst);
                let alive_state = top.rules_state.fst.get_player_alive(local_player.player_id);
                local_player.tick(
                    &player_state,
                    alive_state,
                    &self.timeline,
                    &mut self.visual_effects,
                    &mut self.big_text_controller,
                    &mut self.entities.dust,
                    &mut self.entities.bubbles,
                    &mut self.entities.corpses,
                    &mut self.entities.crowns,
                    &mut self.entities.outfit_switchers);
            }
            else {
                // Remove the player
                local_player.kill_animation(&mut self.visual_effects, None, &self.timeline, &mut self.entities.corpses, &mut self.entities.bubbles);
                to_remove.push((local_player.player_id, local_player.entity_id));
            }
        }

        for (remove_player_id, remove_entity_id) in to_remove {
            self.player_input_controller.remove(remove_player_id);
            self.entities.players.delete_entity_id(remove_entity_id);
        }

        if (transitions.into_round_warmup)
        {
            for player in self.entities.players.inner.iter_mut() {
                player.reset();
            }
        }

        if (transitions.into_lobby) {
            for player in self.entities.players.inner.iter_mut() {
                player.reset();
            }
        }

        self.prop_controller.tick(
            &top.rules_state,
            &self.timeline.map,
            &mut self.entities,
            &transitions,
            self.camera.y as i32 / 8);

        if let CrossyRulesetFST::Lobby { .. } = &top.rules_state.fst {
            let rand = FroggyRand::from_hash((self.timeline.map.get_seed(), top.rules_state.fst.get_round_id(), top.rules_state.game_id, self.prop_controller.t));
            create_outfit_switchers(rand, &self.timeline, &self.entities.players, &mut self.entities.outfit_switchers);

            for switcher in self.entities.outfit_switchers.inner.iter() {
                let rand = rand.subrand(switcher.pos);
                if (rand.gen_unit(1) < 0.4) {
                    let dust = create_dust(rand, &mut self.entities.dust, 4.0, 6.0, V2::new(switcher.pos.x as f32 * 8.0 + 4.0, switcher.pos.y as f32 * 8.0 + 4.0));
                    dust.tint = (Skin::from_enum(switcher.skin).color);
                }
            }
        }

        // Handle crowd sounds.
        match &top.rules_state.fst {
            CrossyRulesetFST::RoundWarmup(_) | CrossyRulesetFST::Round(_) | CrossyRulesetFST::RoundCooldown(_)
            => {
                let screen_offset = (self.camera.y.min(0.0)).abs();
                audio::ensure_playing_with_volume("win", 1.0 / (1.0 + 0.1 * screen_offset));
            },
            _ => {
                audio::stop("win");
            },
        }

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

        if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
            let pos = V2::new(*raft_pos, 10.0) * 8.0;

            if self.entities.raft_sails.inner.is_empty() {
                let raft = self.entities.raft_sails.create(Pos::Absolute(pos));
                raft.setup();
            }

            let raft = self.entities.raft_sails.inner.first_mut().unwrap();
            raft.tick(pos);
        }
        else {
            self.entities.raft_sails.inner.clear();
        }

        self.big_text_controller.tick(&self.timeline, &self.entities.players, &transitions, &new_players, self.camera.y);

        let camera_y_max = top.rules_state.fst.get_screen_y() as f32 + 200.0;
        self.entities.bubbles.prune_dead(camera_y_max);
        self.entities.props.prune_dead(camera_y_max);
        self.entities.dust.prune_dead(camera_y_max);
        self.entities.crowns.prune_dead(camera_y_max);
        self.entities.snowflakes.prune_dead(camera_y_max);

        self.prev_rules = Some(top.rules_state.clone().fst);

        if let CrossyRulesetFST::Lobby { .. } = &top.rules_state.fst {
            self.actor_controller.tick(self.bg_music.current_time_in_secs());
        }
        else {
            self.actor_controller.reset();
        }
    }

    pub unsafe fn draw(&mut self) {
        let top = self.timeline.top_state();

        raylib_sys::BeginMode2D(self.camera.to_raylib());

        //const bg_fill_col: raylib_sys::Color = hex_color("3c285d".as_bytes());
        raylib_sys::ClearBackground(BLACK);

        //let draw_bg_tiles = self.title_screen.as_ref().map(|x| x.draw_bg_tiles).unwrap_or(true);
        let draw_bg_tiles = true;

        if (draw_bg_tiles)
        {
            // Draw background
            //let screen_y = top.rules_state.fst.get_screen_y();
            let screen_y = self.camera.y as i32 / 8;
            let round_id = top.get_round_id();
            let rows = self.timeline.map.get_row_view(round_id, screen_y);

            for row_with_y in rows {
                let row = row_with_y.row;
                let y = row_with_y.y;

                let (col_0, col_1) = match row.row_type {
                    RowType::River(_) | RowType::LobbyRiver => {
                        (river_col_0, river_col_1)
                    },
                    RowType::Road(_) => {
                        (road_col_0, road_col_1)
                    },
                    RowType::IcyRow{..} => {
                        (icy_col_0, icy_col_1)
                    },
                    RowType::Lobby => {
                        let t = if y > 0 {
                            //println!("y = {} t = 0", y);
                            0.0
                        }
                        else {
                            let yy = -y as f32;
                            let t = (yy as f32 / 6.0).clamp(0.0, 1.0);
                            //println!("y = {} yy = {} t = {}", y, yy, t);
                            t
                        };

                        //let t = (-(y as f32).min(0.0) / 10.0).clamp(0.0, 1.0);
                        (lerp_color_rgba(grass_col_0, BLACK, t), lerp_color_rgba(grass_col_1, BLACK, t))
                    }
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
                    //let hydrated = bush_descr.hydrate();
                }

                if let RowType::LobbyRiver = &row.row_type {
                    if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
                        for i in 0..4 {
                            sprites::draw("log", 0, (*raft_pos as f32 + i as f32) * 8.0, y as f32 * 8.0);
                        }
                    }
                }

                if let RowType::LobbyRiverBankLower = &row.row_type {
                    for i in 0..20 {
                        sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    }
                }

                if let RowType::LobbyMain = &row.row_type {
                    let i = 1;
                    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    let i = 18;
                    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                }

                if let RowType::IcyRow(state) = &row.row_type {
                    //for i in 0..=state.path_descr.wall_width {
                    //    sprites::draw("tree_top", 1, i as f32 * 8.0, y as f32 * 8.0);
                    //    sprites::draw("tree_top", 1, (19 - i) as f32 * 8.0, y as f32 * 8.0);
                    //}

                    for x in 0..20 {
                        if x <= state.path_descr.wall_width || x >= 19 - state.path_descr.wall_width || state.blocks.get(x as i32) {
                            sprites::draw("tree_top", 1, x as f32 * 8.0, y as f32 * 8.0);
                        }
                    }
                    //for block in hydrated.blocks {
                    //    sprites::draw("tree_top", 1, block as f32 * 8.0, y as f32 * 8.0);
                    //}
                    //for ice in hydrated.ice {
                    //    sprites::draw("tree_top", 0, ice as f32 * 8.0, y as f32 * 8.0);
                    //}
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

        if let CrossyRulesetFST::Lobby { raft_pos, .. } = &top.rules_state.fst {
            let players_in_ready_zone = top.player_states.iter().filter(|(_, x)| crossy_multi_core::crossy_ruleset::player_in_lobby_ready_zone(x)).count();
            let total_player_count = top.player_states.count_populated();

            if (!self.trailer_mode && total_player_count >= top.rules_state.config.minimum_players as usize)
            {
                let pos = V2::new(*raft_pos, 10.0) * 8.0 + V2::new(1.0, 6.0) * 8.0;
                let image_index = players_in_ready_zone + 1;
                if (image_index > 9) {
                    // error aahhhh
                    // @Todo cap number of players
                }
                else {
                    sprites::draw("font_linsenn_m5x7_numbers", image_index, pos.x, pos.y);
                }
                let pos = pos + V2::new(6.0, 0.0);
                sprites::draw("font_linsenn_m5x7_numbers", 0, pos.x, pos.y);
                let pos = pos + V2::new(6.0, 0.0);
                let image_index = total_player_count + 1;
                sprites::draw("font_linsenn_m5x7_numbers", image_index, pos.x, pos.y);
            }
        }

        self.big_text_controller.draw_lower();
        self.actor_controller.draw();

        {
            // @Perf keep some list and insertion sort
            let mut all_entities = Vec::new();
            self.entities.extend_all_depth(&mut all_entities);

            all_entities.sort_by_key(|(_, depth)| *depth);

            for (e, _) in all_entities {
                self.entities.draw_entity(e, self.pause.is_some());
            }
        }

        raylib_sys::EndMode2D();

        if let Some(title) = self.title_screen.as_mut() {
            title.draw();
        }

        self.big_text_controller.draw();

        {
            if (self.visual_effects.whiteout > 0) {
                raylib_sys::DrawRectangle(0, 0, 160, 160, WHITE);
            }
        }

        if let Some(pause) = self.pause.as_mut() {
            pause.draw();
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
    k: f32,
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
            k: 3.0,
        }
    }

    pub fn tick(&mut self, m_rules_state: Option<&RulesState>, visual_effects: &VisualEffects, transitions: &StateTransition) {
        self.t += 1;

        if let Some(rules_state) = m_rules_state {
            self.target_y = match &rules_state.fst {
                CrossyRulesetFST::RoundWarmup(state) => {
                    let remaining_s = state.remaining_us as f32 / 1_000_000.0;
                    let t = ((remaining_s - 3.0) / 3.0).max(0.0);
                    -16.0 * (t * t) * 2.5
                },
                CrossyRulesetFST::Round(round_state) => {
                    round_state.screen_y as f32
                },
                CrossyRulesetFST::RoundCooldown(round_state) => {
                    round_state.round_state.screen_y as f32
                },
                _ => 0.0
            };

            self.k = match &rules_state.fst {
                CrossyRulesetFST::Lobby{ .. } => {
                    // Lerp towards 3
                    dan_lerp(self.k, 3.0, 10.0)
                },
                _ => 3.0
            };
        }

        self.x = 0.0;

        if transitions.into_round {
            self.y = self.target_y * 8.0
        }
        else {
            self.y = dan_lerp(self.y, self.target_y * 8.0, self.k);
        }

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

pub struct VisualEffects {
    pub t: i32,
    pub whiteout: i32,
    pub screenshake: f32,
    pub noise: f32,

    pub controller_vibrations: Vec<f32>,
}

impl Default for VisualEffects {
    fn default() -> Self {
        let mut vibration = Vec::new();
        for i in 0..4 {
            vibration.push(0.0);
        }

        Self {
            t: 0,
            whiteout: 0,
            screenshake: 0.0,
            noise: 0.0,
            controller_vibrations: vibration,
        }
    }
}

impl VisualEffects {
    pub fn whiteout(&mut self) {
        self.whiteout = self.whiteout.max(6);
    }

    pub fn screenshake(&mut self) {
        self.screenshake = self.screenshake.max(15.0);
        self.noise = self.noise.max(15.0);
    }

    pub fn noise(&mut self) {
        self.noise = self.noise.max(15.0);
    }

    pub fn set_gamepad_vibration(&mut self, id: i32) {
        self.controller_vibrations[id as usize] = 15.0;
    }

    pub fn tick(&mut self) {
        self.t += 1;
        self.whiteout = (self.whiteout - 1).max(0);
        self.screenshake *= 0.85;
        self.noise *= 0.85;

        for (i, x) in self.controller_vibrations.iter_mut().enumerate() {
            *x *= 0.65;

            let id = i as i32;
            unsafe {
                if raylib_sys::IsGamepadAvailable(id) {
                    let value = 
                    if *x > 0.01 {
                        (*x * u16::MAX as f32).floor() as u16
                    }
                    else {
                        0 as u16
                    };

                    // Lifted from
                    //https://github.com/machlibs/rumble/blob/main/src/up_rumble.h
                    // Call win32 directly
                    let x = windows_sys::Win32::UI::Input::XboxController::XINPUT_VIBRATION {
                        wLeftMotorSpeed: value,
                        wRightMotorSpeed: value,
                    };
                    windows_sys::Win32::UI::Input::XboxController::XInputSetState(id as u32, std::ptr::from_ref(&x));
                }
            }
        }
    }
}

#[derive(Default)]
pub struct StateTransition {
    pub into_lobby: bool,
    pub into_round_warmup: bool,
    pub into_round: bool,
    pub into_round_cooldown: bool,
    pub into_winner: bool,

    pub leaving_lobby: bool,
}

impl StateTransition {
    pub fn new(current: &CrossyRulesetFST, prev: &Option<CrossyRulesetFST>) -> Self {
        let mut transitions = Self::default();
        transitions.into_lobby = 
            matches!(current, CrossyRulesetFST::Lobby { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Lobby { .. }));
        transitions.leaving_lobby = 
            !matches!(current, CrossyRulesetFST::Lobby { .. })
            && matches!(prev, Some(CrossyRulesetFST::Lobby { .. }));

        transitions.into_round_warmup = 
            matches!(current, CrossyRulesetFST::RoundWarmup { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundWarmup { .. }));
        transitions.into_round = 
            matches!(current, CrossyRulesetFST::Round { .. })
            && !matches!(prev, Some(CrossyRulesetFST::Round { .. }));
        transitions.into_round_cooldown = 
            matches!(current, CrossyRulesetFST::RoundCooldown { .. })
            && !matches!(prev, Some(CrossyRulesetFST::RoundCooldown { .. }));
        transitions.into_winner = 
            matches!(current, CrossyRulesetFST::EndWinner { .. })
            && !matches!(prev, Some(CrossyRulesetFST::EndWinner { .. }));


        transitions
    }
}

fn create_outfit_switchers(rand: FroggyRand, timeline: &Timeline, players: &EntityContainer<PlayerLocal>, outfit_switchers: &mut EntityContainer<OutfitSwitcher>) {
    let to_create = 4 - outfit_switchers.inner.len();

    if to_create == 0 {
        return;
    }

    let mut options = Vec::new();
    // Not very efficient but doesnt need to be.
    for x in 3..16 {
        for y in 5..9 {
            options.push(CoordPos::new(x, y))
        }
    }

    for player in players.inner.iter() {
        // @Buggy
        // Rough conversion to coordpos, may occcaassionally put someone on top of another, but should usually be fine
        if let Some((idx, _)) = options.iter().enumerate().find(|(_, pos)| **pos == CoordPos::new(player.pos.x.round() as i32, player.pos.y.round() as i32)) {
            options.remove(idx);
        }
    }

    rand.shuffle("shuffle", &mut options);

    for (i, pos) in options.iter().take(to_create).enumerate() {
        let skin = Skin::rand_not_overlapping(rand.subrand(i), &players.inner, &outfit_switchers.inner);
        let switcher = outfit_switchers.create(Pos::Coord(*pos));
        switcher.skin = skin.player_skin;
    }
}

#[derive(Debug)]
enum BGMusicMode {
    Lowpassed,
    FadingOutLowpass,
    Normal,
    Paused,
}

struct TitleBGMusic {
    pub music: raylib_sys::Music,
    pub mode: BGMusicMode,

    // Note this should not be relied on, its just to avoid perf issues.
    // repeatedly trying to fetch state.
    pub playing_unsynced: bool,
}

//const g_music_volume: f32 = 0.6;
static mut g_music_volume: f32 = 0.0;

impl TitleBGMusic {
    pub fn new() -> Self {
        let music = unsafe {
            let mut music = raylib_sys::LoadMusicStream(crate::c_str_leaky("../web-client/static/sounds/mus_jump_at_sun_3.mp3"));
            raylib_sys::SetMusicVolume(music, g_music_volume);
            music.looping = true;
            raylib_sys::AttachAudioStreamProcessor(music.stream, Some(rl_low_pass));
            music
        };

        Self {
            music,
            mode: BGMusicMode::Paused,
            playing_unsynced: false,
        }
    }

    pub fn current_time_in_secs(&self) -> f32 {
        unsafe {
            raylib_sys::GetMusicTimePlayed(self.music)
        }
    }

    pub fn tick(&mut self) {
        unsafe {
            // @Perf
            // Cache
            raylib_sys::SetMusicVolume(self.music, g_music_volume);
        }

        match self.mode {
            BGMusicMode::Lowpassed => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 100.0, 10.0);
                }
            },
            BGMusicMode::FadingOutLowpass => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 50_000.0, 500.0);
                }
            },
            BGMusicMode::Normal => {
                unsafe {
                    LP_FREQ = dan_lerp(LP_FREQ, 50_000.0, 10.0);
                }
            },
            _ => {},
        }

        match self.mode {
            BGMusicMode::Paused => {
                unsafe {
                    if self.playing_unsynced {
                        raylib_sys::PauseMusicStream(self.music);
                        self.playing_unsynced = false;
                    }
                }
            },
            _ => {
                unsafe {
                    if !self.playing_unsynced {
                        raylib_sys::PlayMusicStream(self.music);
                        self.playing_unsynced = true;
                    }
                }
            },
        }

        unsafe {
            raylib_sys::UpdateMusicStream(self.music);
        }
    }
}

static mut LP_DATA: [f32;2] = [0.0, 0.0];
static mut LP_FREQ: f32 = 100.0;

unsafe extern "C" fn rl_low_pass(buffer_void: *mut ::std::os::raw::c_void, frames: ::std::os::raw::c_uint) {
    let cutoff = LP_FREQ / 44100.0; // 70 Hz lowpass filter
    let k = cutoff / (cutoff + 0.1591549431); // RC filter formula

    // Converts the buffer data before using it
    let buffer_raw : *mut f32 = buffer_void.cast();
    let buffer = std::slice::from_raw_parts_mut(buffer_raw, frames as usize * 2);
    for i in 0..(frames as usize) {
        let index = i * 2;

        let l = buffer[index];
        let r = buffer[index+1];

        LP_DATA[0] += k * (l - LP_DATA[0]);
        LP_DATA[1] += k * (r - LP_DATA[1]);
        buffer[index] = LP_DATA[0];
        buffer[index + 1] = LP_DATA[1];
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuInput {
    None,
    Up,
    Down,
    Left,
    Right,
    Enter,
}

impl MenuInput {
    pub fn is_toggle(self) -> bool {
        match self {
            MenuInput::Left | MenuInput::Right | MenuInput::Enter => true,
            _ => false
        }
    }

    pub fn read() -> Self {
        let mut input = MenuInput::None;

        if key_pressed(raylib_sys::KeyboardKey::KEY_UP) {
            input = MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_LEFT) {
            input = MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_DOWN) {
            input = MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_RIGHT) {
            input = MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_SPACE) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_ENTER) {
            input = MenuInput::Enter;
        }

        if key_pressed(raylib_sys::KeyboardKey::KEY_W) {
            input = MenuInput::Up;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_A) {
            input = MenuInput::Left;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_S) {
            input = MenuInput::Down;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_D) {
            input = MenuInput::Right;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_Z) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_X) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_F) {
            input = MenuInput::Enter;
        }
        if key_pressed(raylib_sys::KeyboardKey::KEY_G) {
            input = MenuInput::Enter;
        }

        for i in 0..4 {
            let gamepad_id = i as i32;
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
                input = MenuInput::Up;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
                input = MenuInput::Left;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
                input = MenuInput::Down;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
                input = MenuInput::Right;
            }

            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_UP) {
                input = MenuInput::Enter;
            }
            if gamepad_pressed(gamepad_id, raylib_sys::GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN) {
                input = MenuInput::Enter;
            }
        }

        input
    }
}

#[derive(Default)]
pub struct Pause {
    pub t: i32,
    pub t_since_move: i32,
    pub highlighted: i32,

    pub settings_menu: Option<SettingsMenu>,
}

pub enum PauseResult {
    Nothing,
    Unpause,
    Exit,
    Lobby,
    Feedback,
}

impl Pause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, visual_effects: &mut VisualEffects) -> PauseResult {
        visual_effects.noise = visual_effects.noise.max(0.8);

        if let Some(settings) = self.settings_menu.as_mut() {
            if !settings.tick() {
                self.settings_menu = None;
            }

            return PauseResult::Nothing;
        }

        let input = MenuInput::read();

        self.t += 1;
        self.t_since_move += 1;

        // @Fragile
        let option_count = 5;

        // @TODO controller input / WASD.
        if let MenuInput::Down = input {
            self.highlighted = (self.highlighted + 1) % option_count;
            self.t_since_move = 0;
            audio::play("menu_move");
            //visual_effects.noise = visual_effects.noise.max(5.0);
            //visual_effects.noise();
        }
        if let MenuInput::Up = input {
            self.highlighted = (self.highlighted - 1);
            if (self.highlighted < 0) {
                self.highlighted = option_count - 1;
            }

            self.t_since_move = 0;
            audio::play("menu_move");
            //visual_effects.noise = visual_effects.noise.max(5.0);
            //visual_effects.noise();
        }

        // @TODO controller input / WASD.
        if let MenuInput::Enter = input {
            visual_effects.noise = visual_effects.noise.max(5.0);
            audio::play("menu_click");
            match self.highlighted {
                0 => {
                    // Resume
                    return PauseResult::Unpause;
                }
                1 => {
                    // Lobby
                    return PauseResult::Lobby;
                }
                2 => {
                    // Settings
                    self.settings_menu = Some(SettingsMenu::new());
                }
                3 => {
                    // Feedback
                    return PauseResult::Feedback;
                }
                4 => {
                    // Exit
                    return PauseResult::Exit;
                }
                _ => {
                    // @Unreachable
                    debug_assert!(false);
                    return PauseResult::Unpause;
                }
            }
        }

        PauseResult::Nothing
    }

    pub fn draw(&self) {
        unsafe {
            //let mut col = crate::SEA;
            let mut col = river_col_1;
            col.a = 240;
            //col.a = 80;
            raylib_sys::DrawRectangle(0, 0, 160, 160, col);
        }
    }

    pub fn draw_gui(&self) {
        if let Some(settings) = self.settings_menu.as_ref() {
            settings.draw();
            return;
        }

        unsafe {
            let padding = 16.0;
            let width = raylib_sys::GetScreenWidth();
            let height = raylib_sys::GetScreenHeight();
            let dimensions = V2::new(width as f32, height as f32);
            let text_size = draw_text_center_aligned_ex(self.t_since_move, "Paused", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false, true);
            //let text_size = self.draw_text_center_aligned("Paused", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false);

            let mut p = V2::new(dimensions.x * 0.5, dimensions.y * 0.4);
            p.y += text_size.y + padding;

            let text_size = self.draw_text_center_aligned("Resume", p, self.highlighted == 0);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Lobby", p, self.highlighted == 1);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Settings", p, self.highlighted == 2);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Submit Feedback", p, self.highlighted == 3);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Exit", p, self.highlighted == 4);
        }
    }

    fn draw_text_center_aligned(&self, text: &str, pos: V2, highlighted: bool) -> V2 {
        draw_text_center_aligned_ex(self.t_since_move, text, pos, highlighted, false)
    }
}

#[derive(Default)]
pub struct SettingsMenu {
    pub t: i32,
    pub t_since_move: i32,
    pub highlighted: i32,
}

impl SettingsMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) -> bool {
        self.t += 1;
        self.t_since_move += 1;

        let input = MenuInput::read();

        // @Fragile
        let option_count = 6;

        // @TODO controller input / WASD.
        // @Dedup
        if let MenuInput::Down = input {
            self.highlighted = (self.highlighted + 1) % option_count;
            self.t_since_move = 0;
            audio::play("menu_move");
        }
        if let MenuInput::Up = input {
            self.highlighted = (self.highlighted - 1);
            if (self.highlighted < 0) {
                self.highlighted = option_count - 1;
            }

            self.t_since_move = 0;
            audio::play("menu_move");
        }

        match self.highlighted {
            0 => {
                if let MenuInput::Left = input {
                    let mut state = crate::settings::get();
                    state.music_volume -= 0.1;
                    state.validate();
                    unsafe {
                        g_music_volume = state.music_volume;
                    }
                    crate::settings::set(state);
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.music_volume += 0.1;
                    state.validate();
                    unsafe {
                        g_music_volume = state.music_volume;
                    }
                    crate::settings::set(state);
                }
            }
            1 => {
                if let MenuInput::Left = input {
                    let mut state = crate::settings::get();
                    state.sfx_volume -= 0.1;
                    state.validate();
                    crate::settings::set(state);
                    audio::play("menu_click");
                }
                if let MenuInput::Right = input {
                    let mut state = crate::settings::get();
                    state.sfx_volume += 0.1;
                    state.validate();
                    crate::settings::set(state);
                    audio::play("menu_click");
                }
            }
            2 => {
                // Window mode
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.fullscreen = !state.fullscreen;
                    crate::settings::set(state)
                }
            }
            3 => {
            }
            4 => {
                // CRT
                if input.is_toggle() {
                    audio::play("menu_click");
                    let mut state = crate::settings::get();
                    state.crt_shader = !state.crt_shader;
                    crate::settings::set(state)
                }
            }
            5 => {
                if let MenuInput::Enter = input {
                    audio::play("menu_click");
                    return false;
                }
            }
            _ => {
                // @Unreachable
                debug_assert!(false);
            }
        }

        true
    }

    pub fn draw(&self) {
        unsafe {
            let padding = 16.0;
            let width = raylib_sys::GetScreenWidth();
            let height = raylib_sys::GetScreenHeight();
            let dimensions = V2::new(width as f32, height as f32);
            //let text_size = self.draw_text_center_aligned("Settings", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false);
            let text_size = draw_text_center_aligned_ex(self.t_since_move, "Settings", V2::new(dimensions.x * 0.5, dimensions.y * 0.3), false, true);

            let left = width as f32 * 0.35;
            let right = width as f32 * 0.65;

            let mut p = V2::new(dimensions.x * 0.5, dimensions.y * 0.4);
            p.y += text_size.y + padding;

            let settings = crate::settings::get();

            let music_percentage = format!("{}%", (settings.music_volume * 100.0).round());
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Music Volume:", &music_percentage, left, right, p, self.highlighted == 0, true);
            p.y += text_size.y + padding;

            let sfx_percentage = format!("{}%", (settings.sfx_volume * 100.0).round());
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Sound Effects Volume:", &sfx_percentage, left, right, p, self.highlighted == 1, true);
            p.y += text_size.y + padding;
            //let fullscreen_text = format!("Window Mode: {}", if settings.fullscreen { "Fullscreen" } else { "Windowed"} );
            //let text_size = self.draw_text_center_aligned(&fullscreen_text, p, self.highlighted == 2);
            let window_mode = if settings.fullscreen { "Fullscreen" } else { "Windowed" };
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Window Mode:", window_mode, left, right, p, self.highlighted == 2, false);
            p.y += text_size.y + padding;
            //let text_size = self.draw_text_center_aligned("Visual Effects: Full", p, self.highlighted == 3);
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "Visual Effects:", "Full", left, right, p, self.highlighted == 3, false);
            p.y += text_size.y + padding;
            //let text_size = self.draw_text_center_aligned("CRT Shader: Enabled", p, self.highlighted == 4);
            let text_size = draw_text_left_right_aligned_ex(self.t_since_move, "CRT Effect:", "Enabled", left, right, p, self.highlighted == 4, false);
            p.y += text_size.y + padding;
            let text_size = self.draw_text_center_aligned("Back", p, self.highlighted == 5);
        }
    }

    fn draw_text_center_aligned(&self, text: &str, pos: V2, highlighted: bool) -> V2 {
        draw_text_center_aligned_ex(self.t_since_move, text, pos, highlighted, false)
    }
}


fn draw_text_center_aligned_ex(t_since_move: i32, text: &str, pos: V2, highlighted: bool, big: bool) -> V2 {
    let text_c = c_str_temp(text);
    let spacing = 1.0;

    let mut color = WHITE;
    if (highlighted) {
        // @Dedupe
        // Copypasta from console
        let cursor_col_lerp_t = 0.5 + 0.5 * 
            (t_since_move as f32 / 30.0).cos();
        color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
    }

    unsafe {
        let (font, font_size) = if big {
            (crate::FONT_ROBOTO_BOLD_80.assume_init(), 80.0)
        }
        else {
            (crate::FONT_ROBOTO_BOLD_60.assume_init(), 60.0)
        };
        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        let pos = pos - text_size * 0.5;
        raylib_sys::DrawTextEx(font, text_c, to_vector2(pos), font_size, spacing, color);


        /*
        if (highlighted) {
            let square_size = 16.0;
            let hoz_padding = 32.0;
            raylib_sys::DrawRectangleRec(raylib_sys::Rectangle {
                x: (pos.x - hoz_padding - square_size),
                y: (pos.y + text_size.y * 0.5 - square_size * 0.5),
                width: square_size,
                height: square_size,
            }, color);
        }
        */

        text_size
    }
}

fn draw_text_left_right_aligned_ex(t_since_move: i32, text_left: &str, text_right: &str, left: f32, right: f32, pos: V2, highlighted: bool, arrows: bool) -> V2 {
    let text_c_left = c_str_temp(text_left);
    let text_c_right = c_str_temp(text_right);
    let spacing = 1.0;

    let mut color = WHITE;
    if (highlighted) {
        // @Dedupe
        // Copypasta from console
        let cursor_col_lerp_t = 0.5 + 0.5 * 
            (t_since_move as f32 / 30.0).cos();
        color = crate::lerp_color_rgba(crate::PINK, crate::ORANGE, cursor_col_lerp_t);
    }

    unsafe {
        let (font, font_size) =
            (crate::FONT_ROBOTO_BOLD_60.assume_init(), 60.0);

        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_left, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        //let pos = pos - text_size * 0.5;
        let pos = V2::new(left, pos.y);
        raylib_sys::DrawTextEx(font, text_c_left, to_vector2(pos), font_size, spacing, color);

        let text_size_vector2 = raylib_sys::MeasureTextEx(font, text_c_right, font_size, spacing);
        let text_size = V2::new(text_size_vector2.x, text_size_vector2.y);
        let pos = V2::new(right - text_size.x, pos.y);
        raylib_sys::DrawTextEx(font, text_c_right, to_vector2(pos), font_size, spacing, color);

        //if (arrows && highlighted) {
        if (highlighted) {
            let hoz_padding = 12.0;
            let triangle_mid = pos + V2::new(-hoz_padding, text_size.y * 0.5);
            let p0 = triangle_mid - V2::new(0.0, text_size.y * 0.2);
            let p1 = triangle_mid + V2::new(0.0, text_size.y * 0.2);
            let p2 = triangle_mid - V2::new(text_size.y * 0.3, 0.0);
            raylib_sys::DrawTriangle(
                to_vector2(p0),
                to_vector2(p2),
                to_vector2(p1),
                color,
            );

            let triangle_mid = pos + V2::new(text_size.x + hoz_padding, text_size.y * 0.5);
            let p0 = triangle_mid - V2::new(0.0, text_size.y * 0.2);
            let p1 = triangle_mid + V2::new(0.0, text_size.y * 0.2);
            let p2 = triangle_mid + V2::new(text_size.y * 0.3, 0.0);
            raylib_sys::DrawTriangle(
                to_vector2(p0),
                to_vector2(p1),
                to_vector2(p2),
                color,
            );
        }

        text_size
    }
}