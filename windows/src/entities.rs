use raylib_sys::Color;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, RulesState}, map::{Map, RowType}, math::V2, CoordPos, PlayerId, Pos};
use froggy_rand::FroggyRand;

use crate::{client::StateTransition, hex_color, player_local::{PlayerLocal, PlayerSkin, Skin}, rope::{NodeType, RopeWorld}, sprites, to_vector2};

pub struct PropController {
    gen_to : i32,
    last_generated_round: i32,
    last_generated_game: i32,
    pub t: i32,
}

impl PropController {
    pub fn new() -> Self {
        Self {
            gen_to: 20,
            last_generated_game: -1,
            last_generated_round: -1,
            t: 0,
        }
    }

    pub fn create_stands(entities: &mut EntityManager) -> (CoordPos, CoordPos) {
        let stand_left_id = entities.create_entity(Entity {
            id: 0,
            entity_type: EntityType::Prop,
            pos: Pos::new_coord(0, 10)
        });
        let stand_left_pos = {
            let stand_left = entities.props.get_mut(stand_left_id).unwrap();
            stand_left.depth = Some(100);
            stand_left.sprite = "stand";
            stand_left.draw_offset = V2::new(4.0, 0.0);
            stand_left.pos
        };

        let stand_right_id = entities.create_entity(Entity {
            id: 0,
            entity_type: EntityType::Prop,
            pos: Pos::new_coord(15, 10)
        });

        let stand_right_pos = {
            let stand_right = entities.props.get_mut(stand_right_id).unwrap();
            stand_right.depth = Some(100);
            stand_right.sprite = "stand";
            stand_right.flipped = true;
            stand_right.draw_offset = V2::new(-4.0, 0.0);
            stand_right.pos
        };

        (stand_left_pos, stand_right_pos)
    }

    pub fn tick(&mut self, rules_state: &RulesState, map: &Map, entities: &mut EntityManager, transitions: &StateTransition, screen_y: i32) {
        self.t += 1;

        let round_id = rules_state.fst.get_round_id() as i32;
        let game_id = rules_state.game_id as i32;

        let rand = FroggyRand::from_hash((map.get_seed(), (round_id, game_id)));

        for player in entities.players.inner.iter_mut() {
            if let CrossyRulesetFST::Lobby { .. } = &rules_state.fst {
                player.visible = true;
            }
        }

        if (transitions.into_lobby) {
            entities.clear_round_entities();
            //_ = Self::create_stands(entities);
        }
        if (transitions.into_winner) {
            entities.clear_round_entities();
        }

        if (transitions.into_round_warmup) {
            entities.clear_round_entities();
            crate::console::info(&format!("PropController Resetting, gameid {} roundid {}", game_id, round_id));

            self.last_generated_game = game_id;
            self.last_generated_round = round_id;

            self.gen_to = 20;

            let (stand_left_pos, stand_right_pos) = Self::create_stands(entities);

            let prob_stands = 0.7;
            let ymin = stand_left_pos.y as f32 * 8.0 + 8.0;
            for ix in 0..4 {
                for iy in 0..4 {
                    let x = stand_left_pos.x as f32 * 8.0 + ix as f32 * 8.0 + 4.0;
                    let y = ymin + x / 2.0 + 4.0 + 8.0 * iy as f32;// + 2.0;
                    Spectator::rand(rand, V2::new(x + 4.0, y), false, prob_stands, entities);
                }
            }

            for ix in 0..4 {
                for iy in 0..4 {
                    let x = stand_right_pos.x as f32 * 8.0 + ix as f32 * 8.0 - 4.0;
                    let y = ymin - 4.0 * ix as f32 + 16.0 + 8.0 * iy as f32;// + 2.0;
                    Spectator::rand(rand, V2::new(x + 4.0, y), true, prob_stands, entities);
                }
            }

            let prob_front = 0.35;
            for iy in 0..7 {
                // In front of left stand
                let yy = 13.0 * 8.0 + iy as f32 * 8.0;
                let xx = stand_left_pos.x as f32 * 8.0 + 4.0 * 8.0 + 8.0;
                Spectator::rand(rand, V2::new(xx, yy), false, prob_front, entities);

                // In front of right stand
                let xx = 14.0 * 8.0;
                Spectator::rand(rand, V2::new(xx, yy), true, prob_front, entities);
            }

            let prob_below = 0.2;
            for ix in 0..5 {
                for iy in 0..2 {
                    let yy = 18.0 * 8.0 + iy as f32 * 8.0;

                    // Below left stand
                    let xx = stand_left_pos.x as f32 + ix as f32 * 8.0 - 8.0 + 4.0;
                    Spectator::rand(rand, V2::new(xx, yy), false, prob_below, entities);

                    // Below right stand
                    let xx = 15.0 * 8.0 + ix as f32 * 8.0;
                    Spectator::rand(rand, V2::new(xx, yy), true, prob_below, entities);
                }
            }
        }

        let gen_to_target = rules_state.fst.get_screen_y();
        //while (self.gen_to > gen_to_target - 4) {
        while (self.gen_to > gen_to_target - 32) {
            let row = map.get_row(round_id as u8, self.gen_to);
            match &row.row_type {
                RowType::Path{wall_width} => {
                    for xu in *wall_width..(19-*wall_width) {
                        let x = xu as i32;
                        if rand.gen_unit((x, self.gen_to, "prop")) < 0.15 {
                            let pos = Pos::new_coord(x as i32, self.gen_to);
                            //println!("Pos wallwidth {} {} {:?}", *wall_width, xu, pos);
                            let prop_id = entities.create_entity(Entity {
                                id: 0,
                                entity_type: EntityType::Prop,
                                pos,
                            });
                            let foliage = entities.props.get_mut(prop_id).unwrap();
                            foliage.sprite = "foliage";
                            let image_count = sprites::get_sprite("foliage").len();
                            foliage.image_index = (rand.gen_unit((x, self.gen_to, "ii")) * image_count as f64).floor() as i32;
                            foliage.dynamic_depth = Some(-100.0);
                        }
                    }
                },
                RowType::LobbyMain => {
                    // @Dedup with above
                    // Copypaste
                    for xu in 1..18 {
                        let x = xu as i32;
                        if rand.gen_unit((x, self.gen_to, "prop")) < 0.15 {
                            let pos = Pos::new_coord(x as i32, self.gen_to);
                            //println!("Pos wallwidth {} {} {:?}", *wall_width, xu, pos);
                            let prop_id = entities.create_entity(Entity {
                                id: 0,
                                entity_type: EntityType::Prop,
                                pos,
                            });
                            let foliage = entities.props.get_mut(prop_id).unwrap();
                            foliage.sprite = "foliage";
                            let image_count = sprites::get_sprite("foliage").len();
                            foliage.image_index = (rand.gen_unit((x, self.gen_to, "ii")) * image_count as f64).floor() as i32;
                            foliage.dynamic_depth = Some(-100.0);
                        }
                    }
                },
                _ => {},
            }

            self.gen_to -= 1;
        }

        let rows = map.get_row_view(rules_state.fst.get_round_id(), screen_y);
        for row in &rows {
            if let RowType::IcyRow(_icy_state) = &row.row.row_type {
                if rand.gen_unit((self.t, row.y, "snow")) < 0.01 {
                    let x = rand.gen_unit((self.t, row.y, "x")) as f32 * 160.0;
                    entities.snowflakes.create(Pos::Absolute(V2::new(x, row.y as f32 * 8.0 - 32.0)));
                }
            }
        }
    }
}

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum EntityType {
    #[default]
    Unknown,
    Prop,
    Spectator,
    Car,
    Lillipad,
    Player,
    Corpse,
    Bubble,
    Dust,
    Crown,
    Snowflake,
    OutfitSwitcher,
    RaftSail,
}

#[derive(Debug, Clone, Copy)]
pub struct Entity {
    pub entity_type: EntityType,
    pub pos: Pos,
    pub id: i32,
}

impl Entity {
    pub fn get_r(&self) -> f32 {
        8.0
    }
}

pub trait IsEntity {
    fn create(e: Entity) -> Self;
    fn get(&self) -> Entity;
    fn set_pos(&mut self, p: Pos);
    fn get_depth(&self) -> i32;
    fn draw(&mut self);

    fn alive(&self, _camera_y_max: f32) -> bool {
        true
    }
}

pub struct EntityContainer<T : IsEntity> {
    pub entity_type: EntityType,
    pub inner: Vec<T>,
}

impl<T: IsEntity> EntityContainer<T> {
    pub fn new(entity_type: EntityType) -> Self {
        Self {
            entity_type,
            inner: Default::default(),
        }
    }

    pub fn update_from_entity(&mut self, e : Entity) {
        assert!(self.entity_type == e.entity_type);
        if let Some(x) = self.get_mut(e.id) {
            x.set_pos(e.pos);
        }
    }

    pub fn create_entity(&mut self, mut e: Entity) -> i32 {
        assert!(self.entity_type == e.entity_type);
        unsafe {
            e.id = g_next_id;
            g_next_id += 1;
        }
        self.inner.push(T::create(e));
        e.id
    }

    pub fn create(&mut self, pos: Pos) -> &mut T {
        self.create_entity(Entity {
            id: 0,
            entity_type: self.entity_type,
            pos,
        });

        // Assumes we push to the end of the inner vector
        self.inner.last_mut().unwrap()
    }

    pub fn get(&self, id: i32) -> Option<&T> {
        self.inner.iter().find(|x| x.get().id == id)
    }

    pub fn draw(&mut self, e: Entity) {
        if let Some(entity) = self.get_mut(e.id) {
            entity.draw();
        }
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut T> {
        self.inner.iter_mut().find(|x| x.get().id == id)
    }

    pub fn delete_entity_id(&mut self, id: i32) -> bool {
        let mut found_index: Option<usize> = None;
        for (i, x) in self.inner.iter().enumerate() {
            if x.get().id == id {
                found_index = Some(i);
            }
        }
        if let Some(i) = found_index {
            _ =self.inner.remove(i);
            true
        }
        else {
            false
        }
    }

    pub fn delete_entity(&mut self, e: Entity) -> bool {
        self.delete_entity_id(e.id)
    }

    pub fn prune_dead(&mut self, camera_y_max: f32) {
        let mut new = Vec::with_capacity(self.inner.len());

        let existing = std::mem::take(&mut self.inner);
        for e in existing {
            if e.alive(camera_y_max) {
                new.push(e);
            }
        }

        self.inner = new
    }

    pub fn extend_all_entities_depth(&self, all_entities: &mut Vec<(Entity, i32)>) {
        for x in &self.inner {
            let e = x.get();
            all_entities.push((e, x.get_depth()));
        }
    }
}

impl<'a, T: IsEntity> IntoIterator for &'a EntityContainer<T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

// This may come back to bite me.
static mut g_next_id : i32 = 1;

pub struct EntityManager {
    pub props: EntityContainer<Prop>,
    pub spectators: EntityContainer<Spectator>,
    pub cars: EntityContainer<Car>,
    pub lillipads: EntityContainer<Lillipad>,
    pub players: EntityContainer<PlayerLocal>,
    pub bubbles: EntityContainer<Bubble>,
    pub corpses: EntityContainer<Corpse>,
    pub dust: EntityContainer<Dust>,
    pub crowns: EntityContainer<Crown>,
    pub snowflakes: EntityContainer<Snowflake>,
    pub outfit_switchers: EntityContainer<OutfitSwitcher>,
    pub raft_sails: EntityContainer<RaftSail>,
}

macro_rules! map_over_entity {
    ($self:expr, $e:expr, $entity_type:expr, $f:ident) => {
        match $entity_type {
            EntityType::Prop => $self.props.$f($e),
            EntityType::Spectator => $self.spectators.$f($e),
            EntityType::Car => $self.cars.$f($e),
            EntityType::Lillipad => $self.lillipads.$f($e),
            EntityType::Player => $self.players.$f($e),
            EntityType::Bubble => $self.bubbles.$f($e),
            EntityType::Corpse => $self.corpses.$f($e),
            EntityType::Dust => $self.dust.$f($e),
            EntityType::Crown => $self.crowns.$f($e),
            EntityType::Snowflake => $self.snowflakes.$f($e),
            EntityType::OutfitSwitcher => $self.outfit_switchers.$f($e),
            EntityType::RaftSail => $self.raft_sails.$f($e),
            EntityType::Unknown => {
                panic!()
            }
        }
    };
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            props: EntityContainer::<Prop>::new(EntityType::Prop),
            spectators: EntityContainer::<Spectator>::new(EntityType::Spectator),
            cars: EntityContainer::<Car>::new(EntityType::Car),
            lillipads: EntityContainer::<Lillipad>::new(EntityType::Lillipad),
            players: EntityContainer::<PlayerLocal>::new(EntityType::Player),
            corpses: EntityContainer::<Corpse>::new(EntityType::Corpse),
            bubbles: EntityContainer::<Bubble>::new(EntityType::Bubble),
            dust: EntityContainer::<Dust>::new(EntityType::Dust),
            crowns: EntityContainer::<Crown>::new(EntityType::Crown),
            snowflakes: EntityContainer::<Snowflake>::new(EntityType::Snowflake),
            outfit_switchers: EntityContainer::<OutfitSwitcher>::new(EntityType::OutfitSwitcher),
            raft_sails: EntityContainer::<RaftSail>::new(EntityType::RaftSail),
        }
    }

    pub fn update_entity(&mut self, e: Entity) {
        map_over_entity!(self, e, e.entity_type, update_from_entity);
    }

    pub fn create_entity(&mut self, e: Entity) -> i32 {
        map_over_entity!(self, e, e.entity_type, create_entity)
    }

    pub fn delete_entity(&mut self, e: Entity) -> bool {
        map_over_entity!(self, e, e.entity_type, delete_entity)
    }

    pub fn extend_all_depth(&self, all_entities: &mut Vec<(Entity, i32)>) {
        // Done like this to make sure we dont forget to add.
        for entity_type in EntityType::iter()
        {
            if entity_type == EntityType::Unknown {
                continue;
            }

            map_over_entity!(self, all_entities, entity_type, extend_all_entities_depth);
        }
    }

    pub fn draw_entity(&mut self, e: Entity) {
        map_over_entity!(self, e, e.entity_type, draw)
    }

    pub fn clear_round_entities(&mut self) {
        self.props.inner.clear();
        self.spectators.inner.clear();
        self.bubbles.inner.clear();
        self.corpses.inner.clear();
        self.dust.inner.clear();
        self.snowflakes.inner.clear();
        self.outfit_switchers.inner.clear();
    }
}

pub struct Prop {
    id : i32,
    sprite: &'static str,
    image_index: i32,
    pos: CoordPos,
    draw_offset: V2,
    flipped: bool,
    depth: Option<i32>,
    dynamic_depth: Option<f32>,
}

impl Prop {
    pub fn new(id: i32, pos: CoordPos) -> Self {
        Self {
            id,
            sprite: "unknown",
            image_index: 0,
            pos,
            draw_offset: V2::default(),
            flipped: false,
            depth: None,
            dynamic_depth: None,
        }
    }
}

pub struct Spectator {
    id : i32,
    sprite: &'static str,
    pos_0 : V2,
    pos: V2,
    dynamic_depth: i32,
    image_index: i32,
    flipped: bool,

    t: i32,

    jump_t: i32,
    jump_t_max: i32,
}

impl Spectator {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            sprite: "frog",
            pos_0: pos,
            pos,
            dynamic_depth: 100,
            image_index: 0,
            flipped: false,

            t: 0,

            jump_t: 0,
            jump_t_max: 16,
        }
    }

    pub fn rand(rand: FroggyRand, pos: V2, flipped: bool, prob: f32, entities: &mut EntityManager) {
        if ((rand.gen_unit((pos.x as i32, pos.y as i32, "create_spectotor")) as f32) < prob) {
            let id = entities.create_entity(Entity {
                id: 0,
                entity_type: EntityType::Spectator,
                pos: Pos::Absolute(pos),
            });
            let spectator = entities.spectators.get_mut(id).unwrap();
            spectator.flipped = flipped;

            const SPECTATOR_SPRITES: [&'static str;6] = [
                "frog",
                "duck",
                "mouse",
                "bird",
                "snake",
                //"snake_alt",
                "frog_3",
            ];

            let x : &'static str = *rand.choose((pos.x as i32, pos.y as i32, "s_sprite"), &SPECTATOR_SPRITES[..]);
            spectator.sprite = x;
        }
    }
}

pub struct Car {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
}

impl Car {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
        }
    }
}

pub struct Lillipad {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
}

impl Lillipad {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
        }
    }
}

pub struct Bubble {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
    pub scale: f32,
}

impl Bubble {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
            scale: 1.0,
        }
    }
}

pub struct Corpse {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
    pub skin: Skin,
}

impl Corpse {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
            skin: Skin::default(),
        }
    }
}

pub struct Dust {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
    pub scale: f32,
    pub tint: Color,
}

impl Dust {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
            scale: 1.0,
            tint: crate::WHITE,
        }
    }
}

pub fn create_dust(rand: FroggyRand, dust: &mut EntityContainer<Dust>, offset_min: f32, offset_max: f32, pos: V2) -> &mut Dust {
    let dust_off = rand.gen_unit("off") as f32 * (offset_max - offset_min) + offset_min;
    let dust_dir = rand.gen_unit("dir") * 3.141 * 2.0;
    let pos = pos + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
    //let pos = self.pos * 8.0 + V2::norm_from_angle(dust_dir as f32) * dust_off as f32;
    let dust_part = dust.create(Pos::Absolute(pos));
    dust_part.image_index = rand.gen_usize_range("frame", 0, 3) as i32;
    dust_part.scale = (0.5 + rand.gen_unit("scale") * 0.6) as f32;
    dust_part
}

pub struct Crown {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub t: i32,
    pub t_visible: i32,
    pub t_max: i32,
    pub owner: PlayerId,
    pub offset_i : usize,
}

impl Crown {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            t: 0,
            t_visible: 10,
            t_max: 120,
            owner: PlayerId(0),
            offset_i: 0,
        }
    }
}

pub struct Snowflake {
    pub id : i32,
    pub pos: V2,
    pub t: i32,
}

impl Snowflake {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            t: 0,
        }
    }
}

pub struct OutfitSwitcher {
    pub id : i32,
    pub pos: CoordPos,
    pub t: i32,
    pub skin: PlayerSkin,
}

impl OutfitSwitcher {
    pub fn new(id: i32, pos: CoordPos) -> Self {
        Self {
            id,
            pos,
            t: 0,
            skin: PlayerSkin::Frog,
        }
    }
}

pub struct RaftSail {
    pub id : i32,
    pub pos: V2,
    //pub prev_pos: V2,
    pub t: i32,
    pub flag_rope_world: RopeWorld,
    pub flag_grid: Vec<Vec<usize>>,

    pub sail_rope_world: RopeWorld,
    pub sail_grid: Vec<Vec<usize>>,

    pub wind_norm: f32,
}

impl RaftSail {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            //prev_pos: pos,
            t: 0,

            flag_rope_world: Default::default(),
            flag_grid: Default::default(),

            sail_rope_world: Default::default(),
            sail_grid: Default::default(),

            wind_norm: 0.0,
        }
    }

    pub fn setup(&mut self) {
        assert!(self.flag_grid.is_empty());
        assert!(self.flag_rope_world.nodes.is_empty());
        assert!(self.flag_rope_world.ropes.is_empty());

        assert!(self.sail_grid.is_empty());
        assert!(self.sail_rope_world.nodes.is_empty());
        assert!(self.sail_rope_world.ropes.is_empty());

        /*
        let top = raft.rope_world.add_node(0.0, 0.0);


        raft.rope_world.nodes[top].node_type = crate::rope::NodeType::Fixed;

        let mut prev = None;
        for i in 0..6 {
            let node = raft.rope_world.add_node(0.0 + 2.0 * i as f32, 24.0);
            if (i == 0) {
                //raft.rope_world.nodes[node].node_type = crate::rope::NodeType::Fixed;
            }
            raft.rope_world.add_rope(top, node);

            if let Some(p) = prev {
                raft.rope_world.add_rope(p, node);
            }

            prev = Some(node);
        }
        */

        {
            let max_width = 6;
            let height = 6;
            let x_offset = V2::new(16.0, 0.0);
            let y_offset = V2::new(0.0, 32.0);
            for y in 0..height {
                let p = V2::new(0.0, 0.0) + y_offset * (((y) as f32) * 1.0/((height) as f32));
                let mut row = Vec::new();
                for x in 0..=y {
                    let p = p + x_offset * (-(x as f32) * 1.0/((max_width - 1) as f32));
                    let id = self.sail_rope_world.add_node_p(p);

                    if (x == 0 && (y == 0 || y == height - 1)) {
                        self.sail_rope_world.nodes[id].node_type = NodeType::Fixed;
                    }

                    row.push(id);

                    if y > 0 {
                        if (x < self.sail_grid[y - 1].len()) {
                            let above = self.sail_grid[y - 1][x];
                            self.sail_rope_world.add_rope(above, id);
                        }
                        else {
                            // Triangular
                            let above = *self.sail_grid[y - 1].last().unwrap();
                            self.sail_rope_world.add_rope(above, id);
                        }
                    }

                    if x > 0 {
                        let left = row[row.len() - 2];
                        self.sail_rope_world.add_rope(left, id);
                    }
                }

                self.sail_grid.push(row);
            }
        }

        // @Dedup
        // @Hack
        // Copypasted from curtain
        {
            let width = 6;
            let height = 6;
            let x_offset = V2::new(10.0, 0.0);
            let y_offset = V2::new(0.0, 8.0);
            for y in 0..height {
                //let top_left = top_corner_center + y_offset * (((y + 1) as f32) * 1.0/((height+1) as f32));
                let top_left = V2::new(0.0, 0.0) + y_offset * (((y) as f32) * 1.0/((height) as f32));

                let mut row = Vec::new();
                for x in 0..width {
                    let mut created = false;
                    //if (y == 0) {
                    //    if (x == 0) {
                    //        row.push(node_top_corner_wall);
                    //        created = true;
                    //    }
                    //    if (x == width - 1) {
                    //        row.push(node_top_corner_center);
                    //        created = true;
                    //    }
                    //}

                    if !created {
                        //let p = top_left + x_offset * (((x + 1) as f32) * 1.0/((width+1) as f32));
                        let p = top_left + x_offset * (((x) as f32) * 1.0/((width - 1) as f32));
                        //println!("Creating {}", p);
                        let id = self.flag_rope_world.add_node_p(p);
                        row.push(id);
                    }

                    let id = *row.last().unwrap();

                    if y > 0 {
                        let above = self.flag_grid[y - 1][x];
                        self.flag_rope_world.add_rope(above, id);
                    }

                    if x > 0 {
                        let left = row[row.len() - 2];
                        self.flag_rope_world.add_rope(left, id);
                    }
                }

                self.flag_grid.push(row);
            }

            for row in self.flag_grid.iter() {
                self.flag_rope_world.nodes[row[0]].node_type = crate::rope::NodeType::Fixed;
            }
        }
    }

    pub fn tick(&mut self, pos: V2) {
        self.t += 1;

        self.wind_norm *= 0.9;
        let rand = FroggyRand::new(self.t as u64);
        if rand.gen_unit(0) < 0.01 {
            if rand.gen_unit(1) < 0.5 {
                self.wind_norm += 1.0;
            }
            else {
                self.wind_norm += -1.0;
            }
        }

        let delta = (pos - self.pos).x;
        // Add delta to the wind norm

        self.wind_norm -= delta * 1.5;

        self.flag_rope_world.forces.clear();
        self.flag_rope_world.forces.push(Box::new(crate::rope::ConstantForce {
            force: V2::new(self.wind_norm * 0.03, 0.03),
        }));

        self.sail_rope_world.forces.clear();
        self.sail_rope_world.forces.push(Box::new(crate::rope::ConstantForce {
            force: V2::new(self.wind_norm * 0.03, 0.03),
        }));


        //self.prev_pos = pos;
        self.pos = pos;
        self.flag_rope_world.tick(1.0);
        self.sail_rope_world.tick(1.0);
    }
}
/////////////////////////////////////////////////////////////

// Ugh

impl IsEntity for Prop {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_coord())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Prop,
            pos: Pos::Coord(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Coord(p) = pos {
            self.pos = p;
        }
    }

    fn alive(&self, camera_y_max: f32) -> bool {
        // @Perf
        let h = crate::sprites::get_sprite(self.sprite)[0].height;
        self.pos.y as f32 * 8.0 < h as f32 + camera_y_max
    }

    fn get_depth(&self) -> i32 {
        if let Some(d) = self.depth {
            return d;
        }

        if let Some(dynamic_depth) = self.dynamic_depth {
            return (self.pos.y as f32 * 8.0) as i32 + dynamic_depth as i32;
        }

        0
    }

    fn draw(&mut self) {
        crate::sprites::draw_with_flip(
            &self.sprite,
            self.image_index as usize,
            self.pos.x as f32 * 8.0 + self.draw_offset.x,
            self.pos.y as f32 * 8.0 + self.draw_offset.y,
            self.flipped);
    }
}

impl IsEntity for Spectator {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Spectator,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        return (self.dynamic_depth as f32 * self.pos.y as f32) as i32;
    }

    fn draw(&mut self) {
        self.t += 1;
        let rand = FroggyRand::from_hash((self.t, self.pos.x as i32, self.pos.y as i32));
        //if (self.jump_t <= 0 && rand.gen_unit("jump") < 0.016) {
        if (self.jump_t <= 0 && rand.gen_unit("jump") < 0.010) {
            self.jump_t = self.jump_t_max;
        }

        if (self.jump_t > 0) {
            self.jump_t -= 1;
            //self.pos.y = self.pos_0.y - (self.jump_t as f32 / self.jump_t_max as f32).sin() * 4.0;
            self.pos.y = self.pos_0.y;// - (self.jump_t as f32 / self.jump_t_max as f32).sin() * 4.0;
            self.image_index = (5.0 * (self.jump_t as f32 / self.jump_t_max as f32)).floor() as i32;
        }
        else {
            self.pos = self.pos_0;
            self.image_index = 0;
        }

        crate::sprites::draw("shadow", 0, self.pos_0.x, self.pos_0.y);
        crate::sprites::draw_with_flip(self.sprite, self.image_index as usize, self.pos.x, self.pos.y - 2.0, self.flipped);
    }
}

const spr_car_width : i32 = 24;
const spr_car_height : i32 = 16;
const car_sprite_count : i32 = 4;

impl IsEntity for Car {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Car,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        return self.pos.y as i32 + spr_car_height / 2
    }

    fn draw(&mut self) {
        let mut xx = self.pos.x - spr_car_width as f32 * 0.5;
        if self.flipped {
            xx = self.pos.x - spr_car_width as f32 * 0.5;
            //xx -= 24.0;
        }
        self.image_index = (((100.0 + self.pos.x) / 8.0).floor().abs()) as i32 % car_sprite_count;
        sprites::draw_with_flip("car_flipped", self.image_index as usize, xx, self.pos.y - spr_car_height as f32 * 0.5, self.flipped);

        /*
        unsafe {
            if self.flipped {
                raylib_sys::DrawCircle(self.pos.x as i32, self.pos.y as i32, 6.0, crate::PINK);
            }
            else {
                raylib_sys::DrawCircle(self.pos.x as i32, self.pos.y as i32, 6.0, crate::BEIGE);
            }
        }
        */
    }
}

impl IsEntity for Lillipad {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Lillipad,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 - 1000
    }

    fn draw(&mut self) {
        sprites::draw("log", 0, self.pos.x, self.pos.y);
    }
}

impl IsEntity for Corpse {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Corpse,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 - 10
    }

    fn draw(&mut self) {
        sprites::draw(self.skin.dead_sprite, self.image_index as usize, self.pos.x, self.pos.y);
    }
}

impl IsEntity for Bubble {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Bubble,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32
    }

    fn draw(&mut self) {
        self.scale -= 0.025;
        self.pos.y -= 0.2;
        if (self.scale > 0.0) {
            let size = 8.0 * self.scale;
            let x = self.pos.x - 0.5 * size;
            let y = self.pos.y - 0.5 * size;
            sprites::draw_scaled("bubble", self.image_index as usize, x, y, self.scale);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        self.scale > 0.0
    }
}

impl IsEntity for Dust {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Dust,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 - 20
    }

    fn draw(&mut self) {
        self.scale -= 0.025;
        if (self.scale > 0.0) {
            let size = 8.0 * self.scale;
            let x = self.pos.x - 0.5 * size;
            let y = self.pos.y + 2.0 - 0.5 * size;
            sprites::draw_scaled_tinted("dust", self.image_index as usize, x, y, self.scale, self.tint);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        self.scale > 0.0
    }
}

impl IsEntity for Crown {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Crown,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 + 20
    }

    fn draw(&mut self) {
        self.t += 1;
        if self.t >= self.t_visible {
            sprites::draw("crown", self.image_index as usize, self.pos.x, self.pos.y);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        self.t < self.t_max
    }
}

impl IsEntity for Snowflake {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::Snowflake,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 + 20
    }

    fn draw(&mut self) {
        self.t += 1;
        self.pos.y += 0.1;
        self.pos.x += ((self.t as f32) * 0.01).sin() * 0.08;
        unsafe {
            let scale = (0.01 * self.t as f32).min(0.5);
            raylib_sys::DrawCircleLinesV(crate::to_vector2(self.pos), scale, crate::WHITE);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        self.t < 500
    }
}

impl IsEntity for OutfitSwitcher {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_coord())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::OutfitSwitcher,
            pos: Pos::Coord(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Coord(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        self.pos.y as i32 * 8 - 10
    }

    fn draw(&mut self) {
        self.t += 1;
        {
            //let scale = 1.0 + 0.2 * (self.t as f32 / 100.0).sin();
            //let rand = FroggyRand::new(self.t as u64);
            //let rand = rand.subrand(self.pos);
            let t_offset = (FroggyRand::from_hash(self.pos).gen_unit(0) * 3.141 * 2.0) as f32;

            let scale = 1.0;
            let mut p = V2::new(self.pos.x as f32 * 8.0, self.pos.y as f32 * 8.0);
            p.y -= 2.0;
            //p = p + V2::norm_from_angle(rand.gen_unit(0) as f32 * 3.141 * 2.0) * 0.24;
            p = p + V2::norm_from_angle(self.t as f32 * 0.1 + t_offset) * 1.0;//0.24;
            let xx = p.x;
            let yy = p.y;

            let shadow_pos = p + V2::new(0.0, 3.0);
            sprites::draw("shadow", 0, shadow_pos.x, shadow_pos.y);

            let skin = Skin::from_enum(self.skin);
            unsafe {
                let r = 8.0 * scale;
                let rec = raylib_sys::Rectangle { 
                    //x: xx + 4.0 - r * 0.5,
                    //y: yy + 4.0 - r * 0.5,
                    x: xx,
                    y: yy,
                    width: r,
                    height: r,
                };

                let mut rec_border = rec;
                rec_border.x -= 1.0;
                rec_border.y -= 1.0;
                rec_border.width += 2.0;
                rec_border.height += 2.0;
                //raylib_sys::DrawRectangleLinesEx(rec_border, 1.0, crate::BLACK);
                raylib_sys::DrawRectangleRec(rec, crate::WHITE);
            }
            sprites::draw(&skin.sprite, 0, xx, yy);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        true
    }
}

impl IsEntity for RaftSail {
    fn create(e: Entity) -> Self {
        Self::new(e.id, e.pos.get_abs())
    }

    fn get(&self) -> Entity {
        Entity {
            id: self.id,
            entity_type: EntityType::RaftSail,
            pos: Pos::Absolute(self.pos),
        }
    }

    fn set_pos(&mut self, pos : Pos) {
        if let Pos::Absolute(p) = pos {
            self.pos = p;
        }
    }

    fn get_depth(&self) -> i32 {
        //self.pos.y as i32 + 40 - 8
        self.pos.y as i32 + 40
    }

    fn draw(&mut self) {
        self.t += 1;
        {
            //let xx = self.pos.x - 4.0;
            let xx = self.pos.x + 2.0;
            let yy = self.pos.y + 8.0;
            //sprites::draw("raft_sail_frame", 0, xx, yy);
            //sprites::draw("raft", 0, xx, yy);
        }

        const brown_frame: raylib_sys::Color = crate::hex_color("8f563b".as_bytes());

        unsafe {
            //raylib_sys::DrawCircleLinesV(to_vector2(self.pos), 3.0, crate::WHITE);
            //for n in self.rope_world.nodes.iter() {
            //    let pos = n.pos + self.pos;
            //    println!("Drawing {}", pos);
            //    raylib_sys::DrawCircleLinesV(to_vector2(pos), 3.0, crate::WHITE);
            //}

            let base_pos = self.pos + V2::new(2.0, 16.0);
            raylib_sys::DrawLineV(to_vector2(base_pos), to_vector2(base_pos + V2::new(0.0, 16.0)), brown_frame);

            for edge in self.flag_rope_world.ropes.iter() {
                let from_pos = base_pos + self.flag_rope_world.nodes[edge.from].pos;
                let to_pos = base_pos + self.flag_rope_world.nodes[edge.to].pos;
                raylib_sys::DrawLineV(to_vector2(from_pos), to_vector2(to_pos), crate::WHITE);
            }

            // @Dedup copypasted from curtains

            let h = self.flag_grid.len();
            let w = self.flag_grid[0].len();
            for y in 1..h {
                for x in 1..w {
                    let top_left = base_pos + self.flag_rope_world.get_node(self.flag_grid[y-1][x-1]).pos;
                    let top_right = base_pos + self.flag_rope_world.get_node(self.flag_grid[y-1][x]).pos;
                    let bot_left = base_pos + self.flag_rope_world.get_node(self.flag_grid[y][x-1]).pos;
                    let bot_right = base_pos + self.flag_rope_world.get_node(self.flag_grid[y][x]).pos;

                    let col_a = if (x + y) % 2 == 0 {
                        //crate::PURPLE
                        //curtain_lighter
                        crate::WHITE
                        //curtain_darker
                    }
                    else {
                        crate::GREEN
                    };

                    let col_b = if (x + y) % 2 == 0 {
                        //crate::PURPLE
                        //curtain_lighter
                        crate::WHITE
                        //curtain_darker
                    }
                    else {
                        crate::RED
                    };

                    //if false {
                        unsafe {
                            raylib_sys::DrawTriangle(
                                to_vector2(top_left),
                                to_vector2(bot_left),
                                to_vector2(top_right),
                                col_b);
                            raylib_sys::DrawTriangle(
                                to_vector2(bot_right),
                                to_vector2(top_right),
                                to_vector2(bot_left),
                                col_b);
                        }
                    //}
                    //else {
                        unsafe {
                            raylib_sys::DrawTriangle(
                                to_vector2(top_left),
                                to_vector2(top_right),
                                to_vector2(bot_left),
                                col_a);
                            raylib_sys::DrawTriangle(
                                to_vector2(bot_right),
                                to_vector2(bot_left),
                                to_vector2(top_right),
                                col_a);
                        }
                    //}
                }
            }
        }

        let base_pos = self.pos + V2::new(18.0, 20.0);

        // Shadow
        for x in 1..self.sail_grid.last().unwrap().len() {
            let h = self.sail_grid.len();
            let y = h - 1;
            let mut top_left = base_pos + self.sail_rope_world.get_node(self.sail_grid[y-1][x-1]).pos;
            let mut top_right = base_pos + if (x < self.sail_grid[y-1].len()) {
                self.sail_rope_world.get_node(self.sail_grid[y-1][x]).pos
            }
            else {
                self.sail_rope_world.get_node(*self.sail_grid[y-1].last().unwrap()).pos
            };

            let mut bot_left = base_pos + self.sail_rope_world.get_node(self.sail_grid[y][x-1]).pos;
            let mut bot_right = base_pos + self.sail_rope_world.get_node(self.sail_grid[y][x]).pos;

            let offset = 3.0;
            top_left.y += offset;
            top_right.y += offset;
            bot_left.y += offset;
            bot_right.y += offset;

            let col = crate::BLACK;
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

        unsafe {
            raylib_sys::DrawLineV(to_vector2(base_pos), to_vector2(base_pos + V2::new(0.0, 16.0)), brown_frame);

            for edge in self.sail_rope_world.ropes.iter() {
                let from_pos = base_pos + self.sail_rope_world.nodes[edge.from].pos;
                let to_pos = base_pos + self.sail_rope_world.nodes[edge.to].pos;
                raylib_sys::DrawLineV(to_vector2(from_pos), to_vector2(to_pos), crate::BEIGE);
            }

            let h = self.sail_grid.len();
            for y in 1..h {
                let w = self.sail_grid[y].len();
                for x in 1..w {
                    let top_left = base_pos + self.sail_rope_world.get_node(self.sail_grid[y-1][x-1]).pos;

                    let top_right = base_pos + if (x < self.sail_grid[y-1].len()) {
                        self.sail_rope_world.get_node(self.sail_grid[y-1][x]).pos
                    }
                    else {
                        self.sail_rope_world.get_node(*self.sail_grid[y-1].last().unwrap()).pos
                    };

                    //let top_right = base_pos + self.sail_rope_world.get_node(self.sail_grid[y-1][x]).pos;
                    let bot_left = base_pos + self.sail_rope_world.get_node(self.sail_grid[y][x-1]).pos;
                    let bot_right = base_pos + self.sail_rope_world.get_node(self.sail_grid[y][x]).pos;

                    let col_a = if (x + y) % 2 == 0 {
                        //crate::PURPLE
                        //curtain_lighter
                        crate::WHITE
                        //curtain_darker
                    }
                    else {
                        crate::WHITE
                    };

                    let col_b = if (x + y) % 2 == 0 {
                        //crate::PURPLE
                        //curtain_lighter
                        crate::WHITE
                        //curtain_darker
                    }
                    else {
                        crate::WHITE
                    };

                    //if false {
                        unsafe {
                            raylib_sys::DrawTriangle(
                                to_vector2(top_left),
                                to_vector2(bot_left),
                                to_vector2(top_right),
                                col_b);
                            raylib_sys::DrawTriangle(
                                to_vector2(bot_right),
                                to_vector2(top_right),
                                to_vector2(bot_left),
                                col_b);
                        }
                    //}
                    //else {
                        unsafe {
                            raylib_sys::DrawTriangle(
                                to_vector2(top_left),
                                to_vector2(top_right),
                                to_vector2(bot_left),
                                col_a);
                            raylib_sys::DrawTriangle(
                                to_vector2(bot_right),
                                to_vector2(bot_left),
                                to_vector2(top_right),
                                col_a);
                        }
                    //}
                }
            }
        }

        //unsafe {
        //    let text = crate::c_str_temp(&format!("{} / {}", 0, 2));
        //    //let pos = V2::new(*raft_pos as f32 * 8.0 + 16.0, y as f32 * 8.0);
        //    let pos = self.pos + V2::new(0.0, 4.0) * 8.0;
        //    raylib_sys::DrawTextEx(crate::FONT_m3x6.assume_init_read(), text, crate::to_vector2(pos), 16.0, 1.0, crate::WHITE);
        //    //raylib_sys::DrawText(text, pos.x as i32, pos.y as i32, 6, crate::WHITE);
        //}
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        true
    }
}