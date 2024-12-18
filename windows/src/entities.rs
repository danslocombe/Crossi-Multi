use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crossy_multi_core::{crossy_ruleset::{AliveState, CrossyRulesetFST, GameConfig, RulesState}, game, map::{Map, RowType}, math::V2, player::{PlayerState, PlayerStatePublic}, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, GameState, Input, PlayerId, PlayerInputs, Pos};
use froggy_rand::FroggyRand;

use crate::{player_local::PlayerLocal, sprites};

pub struct PropController {
    gen_to : i32,
    last_generated_round: i32,
    last_generated_game: i32,
}

impl PropController {
    pub fn new() -> Self {
        Self {
            gen_to: 20,
            last_generated_game: -1,
            last_generated_round: -1,
        }
    }

    pub fn tick(&mut self, rules_state: &RulesState, map: &Map, entities: &mut EntityManager) {
        let round_id = rules_state.fst.get_round_id() as i32;
        let game_id = rules_state.game_id as i32;

        let rand = FroggyRand::from_hash((map.get_seed(), (round_id, game_id)));

        if (self.last_generated_game != game_id || self.last_generated_round != round_id) {
            // Regen.

            // Destroy all props.
            crate::console::info(&format!("PropController Resetting, gameid {} roundid {}", game_id, round_id));
            entities.props.inner.clear();
            entities.spectators.inner.clear();
            entities.bubbles.inner.clear();
            entities.corpses.inner.clear();
            entities.dust.inner.clear();
            for player in entities.players.inner.iter_mut() {
                player.reset();
            }

            self.last_generated_game = game_id;
            self.last_generated_round = round_id;

            self.gen_to = 20;

            let in_lobby = if let CrossyRulesetFST::Lobby { .. } = &rules_state.fst {
                true
            }
            else {
                false
            };

            //if let CrossyRulesetFST::Lobby { .. } = &rules_state.fst {
            if (false) {
                // ...
            }
            else {
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

                if (!in_lobby) {

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
            }
        }

        let gen_to_target = rules_state.fst.get_screen_y();
        while (self.gen_to > gen_to_target - 4) {
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
                _ => {},
            }

            self.gen_to -= 1;
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

    pub fn delete_entity(&mut self, e: Entity) -> bool {
        let mut found_index: Option<usize> = None;
        for (i, x) in self.inner.iter().enumerate() {
            if x.get().id == e.id {
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
}

impl Corpse {
    pub fn new(id: i32, pos: V2) -> Self {
        Self {
            id,
            pos,
            image_index: 0,
            flipped: false,
        }
    }
}

pub struct Dust {
    pub id : i32,
    pub pos: V2,
    pub image_index: i32,
    pub flipped: bool,
    pub scale: f32,
}

impl Dust {
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
        self.pos.y as i32
    }

    fn draw(&mut self) {
        sprites::draw("frog_dead", 0, self.pos.x, self.pos.y);
        //sprites::draw("log", 0, self.pos.x, self.pos.y);
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
            sprites::draw_scaled("dust", self.image_index as usize, x, y, self.scale);
        }
    }

    fn alive(&self, _camera_y_max: f32) -> bool {
        self.scale > 0.0
    }
}