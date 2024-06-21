use std::collections::HashMap;

use crate::{spawner, Position, SHOW_MAPGEN_VISUALIZER};

use super::common;
use super::common::paint;
use super::common::Symmetry;
use super::MapBuilder;
use super::{Map, TileType};
use rltk::RandomNumberGenerator;
use specs::World;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DLAAlgorithm {
    /// Walk Inwards algorithm spawns drunkwards which walks until it finds a floor tile. When
    /// that happens the drunkward stops and the previous tile gets turned into floor
    WalkInwards,
    /// Walk Outwards algorithm spawns drunkwards at the center of the map which walks until it
    /// finds a wall tile. Once that happens, the drunkward stops and the wall tile gets turned
    /// into floor
    WalkOutwards,
    /// Central Attractor algorithm spawns drunkwards at some random location that walk through a
    /// line towards the center of the map. Once it hits a floor tile it stops and the previous
    /// tile gets turned into floor as well, and the drunkward stops.
    CentralAttractor,
}

pub struct DLABuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    algorithm: DLAAlgorithm,
    /// Specifies how many floor tiles we "paint" in one go
    brush_size: i32,
    symmetry: Symmetry,
    /// Lower bound percentage of the tiles that must be floor tiles
    floor_percent: f32,
}

impl MapBuilder for DLABuilder {
    fn build_map(&mut self) {
        self.build();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for area in self.noise_areas.iter() {
            spawner::spawn_region(ecs, area.1, self.depth);
        }
    }

    fn get_map(&mut self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn take_snapshot(&mut self) {
        if SHOW_MAPGEN_VISUALIZER {
            // stores a copy of the map while making all tiles visible
            let mut snapshot = self.map.clone();
            for v in snapshot.revealed_tiles.iter_mut() {
                *v = true;
            }
            self.history.push(snapshot);
        }
    }
}

impl DLABuilder {
    pub fn walk_inwards(new_depth: i32) -> DLABuilder {
        DLABuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            algorithm: DLAAlgorithm::WalkInwards,
            brush_size: 1,
            symmetry: Symmetry::Vertical,
            floor_percent: 0.25,
        }
    }

    pub fn walk_outwards(new_depth: i32) -> DLABuilder {
        DLABuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            algorithm: DLAAlgorithm::WalkOutwards,
            brush_size: 2,
            symmetry: Symmetry::None,
            floor_percent: 0.25,
        }
    }

    pub fn central_attractor(new_depth: i32) -> DLABuilder {
        DLABuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            algorithm: DLAAlgorithm::CentralAttractor,
            brush_size: 2,
            symmetry: Symmetry::Both,
            floor_percent: 0.25,
        }
    }

    pub fn insectoid(new_depth: i32) -> DLABuilder {
        DLABuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            algorithm: DLAAlgorithm::CentralAttractor,
            brush_size: 2,
            symmetry: Symmetry::Horizontal,
            floor_percent: 0.25,
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        // Generate a random initial floor setting around the center of the map
        self.starting_position = Position {
            x: self.map.width / 2,
            y: self.map.height / 2,
        };
        let start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);
        self.take_snapshot();

        self.map.tiles[start_idx] = TileType::Floor;
        self.map.tiles[start_idx - 1] = TileType::Floor;
        self.map.tiles[start_idx + 1] = TileType::Floor;
        self.map.tiles[start_idx - self.map.width as usize] = TileType::Floor;
        self.map.tiles[start_idx + self.map.width as usize] = TileType::Floor;
        self.take_snapshot();

        let mut floor_count = 5;
        let desired_floor_count =
            ((self.map.width * self.map.height) as f32 * self.floor_percent) as usize;

        // Apply each algorithm case
        match self.algorithm {
            DLAAlgorithm::WalkInwards => {
                while floor_count < desired_floor_count {
                    let mut drunk_x = rng.roll_dice(1, self.map.width - 3) + 1;
                    let mut drunk_y = rng.roll_dice(1, self.map.height - 3) + 1;
                    let mut previous_pos_x = drunk_x;
                    let mut previous_pos_y = drunk_y;

                    loop {
                        let drunk_pos_id = self.map.xy_idx(drunk_x, drunk_y);
                        if self.map.tiles[drunk_pos_id] == TileType::Floor {
                            floor_count += paint(
                                &mut self.map,
                                self.symmetry,
                                self.brush_size,
                                previous_pos_x,
                                previous_pos_y,
                            );
                            self.take_snapshot();
                            break;
                        }

                        previous_pos_x = drunk_x;
                        previous_pos_y = drunk_y;

                        let rolled_direction = rng.roll_dice(1, 4);
                        match rolled_direction {
                            1 if drunk_x > 2 => drunk_x -= 1,
                            2 if drunk_x < self.map.width - 2 => drunk_x += 1,
                            3 if drunk_y > 2 => drunk_y -= 1,
                            4 if drunk_y < self.map.height - 2 => drunk_y += 1,
                            _ => {}
                        }
                    }
                }
            }
            DLAAlgorithm::WalkOutwards => {
                while floor_count < desired_floor_count {
                    let mut drunk_x = self.starting_position.x;
                    let mut drunk_y = self.starting_position.y;
                    let mut drunk_pos_id = self.map.xy_idx(drunk_x, drunk_y);

                    while self.map.tiles[drunk_pos_id] == TileType::Floor {
                        let rolled_direction = rng.roll_dice(1, 4);
                        match rolled_direction {
                            1 if drunk_x > 2 => drunk_x -= 1,
                            2 if drunk_x < self.map.width - 2 => drunk_x += 1,
                            3 if drunk_y > 2 => drunk_y -= 1,
                            4 if drunk_y < self.map.height - 2 => drunk_y += 1,
                            _ => {}
                        }
                        drunk_pos_id = self.map.xy_idx(drunk_x, drunk_y)
                    }

                    floor_count += paint(
                        &mut self.map,
                        self.symmetry,
                        self.brush_size,
                        drunk_x,
                        drunk_y,
                    );
                    self.take_snapshot();
                }
            }
            DLAAlgorithm::CentralAttractor => {
                while floor_count < desired_floor_count {
                    let mut drunk_x = rng.roll_dice(1, self.map.width - 3) + 1;
                    let mut drunk_y = rng.roll_dice(1, self.map.height - 3) + 1;
                    let mut pos_id = self.map.xy_idx(drunk_x, drunk_y);
                    let mut previous_pos_x = drunk_x;
                    let mut previous_pos_y = drunk_y;

                    let mut path = rltk::line2d(
                        rltk::LineAlg::Bresenham,
                        rltk::Point::new(drunk_x, drunk_y),
                        rltk::Point::new(self.starting_position.x, self.starting_position.y),
                    );

                    while self.map.tiles[pos_id] == TileType::Wall && !path.is_empty() {
                        previous_pos_x = drunk_x;
                        previous_pos_y = drunk_y;
                        drunk_x = path[0].x;
                        drunk_y = path[0].y;
                        path.remove(0);
                        pos_id = self.map.xy_idx(drunk_x, drunk_y);
                    }

                    floor_count += paint(
                        &mut self.map,
                        self.symmetry,
                        self.brush_size,
                        previous_pos_x,
                        previous_pos_y,
                    );
                    self.take_snapshot();
                }
            }
        }

        let stairs_idx =
            common::cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);

        // Place the stairs
        self.map.tiles[stairs_idx] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        self.noise_areas = common::generate_voronoi_spawn_regions(&self.map, &mut rng);
    }
}
