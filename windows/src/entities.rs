use core::slice;
use std::{mem::MaybeUninit, ops::Add};

use crossy_multi_core::{crossy_ruleset::{CrossyRulesetFST, GameConfig, RulesState}, game, map::RowType, player::{PlayerState, PlayerStatePublic}, timeline::{Timeline, TICK_INTERVAL_US}, CoordPos, Input, PlayerId, PlayerInputs, Pos};
use froggy_rand::FroggyRand;

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

    pub fn tick(&mut self, rules_state: &RulesState, entities: &mut EntityManager) {
        let round_id = rules_state.fst.get_round_id() as i32;
        let game_id = rules_state.game_id as i32;

        if (self.last_generated_game != game_id || self.last_generated_round != round_id) {
            // Regen.

            self.last_generated_game = game_id;
            self.last_generated_round = round_id;

            self.gen_to = 20;

            let stand_left_id = entities.create_entity(Entity {
                id: 0,
                entity_type: EntityType::Prop,
                pos: Pos::new_coord(1, 10)
            });
            let stand_left = entities.props.get_mut(stand_left_id).unwrap();
            stand_left.depth = Some(100);
            stand_left.sprite = "stand";

            let stand_right_id = entities.create_entity(Entity {
                id: 0,
                entity_type: EntityType::Prop,
                pos: Pos::new_coord(15, 10)
            });
            let stand_right = entities.props.get_mut(stand_right_id).unwrap();
            stand_right.depth = Some(100);
            stand_right.sprite = "stand";
        }
    }
}

pub struct Prop {
    id : i32,
    sprite: &'static str,
    image_index: i32,
    pos: CoordPos,
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
            flipped: false,
            depth: None,
            dynamic_depth: None,
        }
    }

    pub fn alive(&self, camera_y_max: f32) -> bool {
        // @Perf
        let h = crate::sprites::get_sprite(self.sprite)[0].height;
        self.pos.y as f32 * 8.0 < h as f32 + camera_y_max
    }
}

#[repr(u8)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    #[default]
    Unknown,
    Prop,
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
    fn draw(&self);
}

pub struct EntityContainer<T : IsEntity> {
    pub entity_type: EntityType,
    pub inner: Vec<T>,
}

impl<T: IsEntity> EntityContainer<T> {
    pub fn update_from_entity(&mut self, e : Entity) {
        assert!(self.entity_type == e.entity_type);
        if let Some(x) = self.get_mut(e.id) {
            x.set_pos(e.pos);
        }
    }

    pub fn create_entity(&mut self, e: Entity) {
        assert!(self.entity_type == e.entity_type);
        self.inner.push(T::create(e));
    }

    pub fn get(&self, id: i32) -> Option<&T> {
        self.inner.iter().find(|x| x.get().id == id)
    }

    pub fn draw(&self, e: Entity) {
        if let Some(entity) = self.get(e.id) {
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

pub struct EntityManager {
    pub next_id: i32,
    pub props: EntityContainer<Prop>,
}

macro_rules! map_over_entity {
    ($self:expr, $e:expr, $f:ident) => {
        match $e.entity_type {
            EntityType::Prop => $self.props.$f($e),
            EntityType::Unknown => {
                panic!()
            }
        }
    };
}

impl EntityManager {
    pub fn update_entity(&mut self, e: Entity) {
        map_over_entity!(self, e, update_from_entity);
    }

    pub fn create_entity(&mut self, mut e: Entity) -> i32 {
        let eid = self.next_id;
        e.id = eid;
        self.next_id += 1;
        map_over_entity!(self, e, create_entity);
        eid
    }

    pub fn delete_entity(&mut self, e: Entity) -> bool {
        map_over_entity!(self, e, delete_entity)
    }

    pub fn extend_all_depth(&self, all_entities: &mut Vec<(Entity, i32)>) {
        self.props.extend_all_entities_depth(all_entities);
    }

    pub fn draw_entity(&self, e: Entity) {
        map_over_entity!(self, e, draw)
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

    fn get_depth(&self) -> i32 {
        if let Some(d) = self.depth {
            return d;
        }

        if let Some(dynamic_depth) = self.dynamic_depth {
            return (dynamic_depth * self.pos.y as f32) as i32;
        }

        0
    }

    fn draw(&self) {
        // TODO flip
        crate::sprites::draw(&self.sprite, self.image_index as usize, self.pos.x as f32 * 8.0, self.pos.y as f32 * 8.0);
    }
}