use crate::Position;

use super::{Map, Rect, TileType};

mod simple_map;
use simple_map::SimpleMapBuilder;
use specs::World;
mod common;

pub trait MapBuilder {
    fn build_map(&mut self);
    fn spawn_entities(&mut self, ecs: &mut World);
    fn get_map(&mut self) -> Map;
    fn get_starting_position(&mut self) -> Position;
    fn get_snapshot_history(&self) -> Vec<Map>;
    fn take_snapshot(&mut self);
}

pub fn random_builder(new_depth: i32) -> Box<dyn MapBuilder> {
    Box::new(SimpleMapBuilder::new(new_depth))
}
