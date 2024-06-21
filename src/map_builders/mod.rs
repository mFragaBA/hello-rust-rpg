use crate::Position;

use super::{Map, Rect, TileType};

mod simple_map;
use rltk::RandomNumberGenerator;
use simple_map::SimpleMapBuilder;
mod bsp_dungeon;
use bsp_dungeon::BspDungeonBuilder;
mod bsp_interior;
use bsp_interior::BspInteriorBuilder;
mod cellular_automata;
use cellular_automata::CellularAutomataBuilder;
mod dla;
use dla::DLABuilder;
mod drunkard;
use drunkard::DrunkardsWalkBuilder;
mod maze;
use maze::MazeBuilder;
mod voronoi_cell;
use voronoi_cell::VoronoiCellBuilder;
mod wave_function_collapse;
use specs::World;
use wave_function_collapse::WaveFunctionCollapseBuilder;
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
    let mut rng = RandomNumberGenerator::new();
    // let builder = rng.roll_dice(1, 14);
    // match builder {
    //     1 => Box::new(BspDungeonBuilder::new(new_depth)),
    //     2 => Box::new(BspInteriorBuilder::new(new_depth)),
    //     3 => Box::new(CellularAutomataBuilder::new(new_depth)),
    //     4 => Box::new(DrunkardsWalkBuilder::open_area(new_depth)),
    //     5 => Box::new(DrunkardsWalkBuilder::open_halls(new_depth)),
    //     6 => Box::new(DrunkardsWalkBuilder::widening_passages(new_depth)),
    //     7 => Box::new(DrunkardsWalkBuilder::wider_passages(new_depth)),
    //     8 => Box::new(DrunkardsWalkBuilder::fearful_symmetry(new_depth)),
    //     9 => Box::new(MazeBuilder::new(new_depth)),
    //     10 => Box::new(DLABuilder::walk_inwards(new_depth)),
    //     11 => Box::new(DLABuilder::walk_outwards(new_depth)),
    //     12 => Box::new(DLABuilder::central_attractor(new_depth)),
    //     13 => Box::new(DLABuilder::insectoid(new_depth)),
    //     14 => Box::new(VoronoiCellBuilder::pythagoras(new_depth)),
    //     15 => Box::new(VoronoiCellBuilder::manhattan(new_depth)),
    //     16 => Box::new(VoronoiCellBuilder::chebyshev(new_depth)),
    //     _ => Box::new(SimpleMapBuilder::new(new_depth))
    // }
    Box::new(WaveFunctionCollapseBuilder::new(new_depth))
}
