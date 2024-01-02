use std::collections::HashMap;

use rltk::RandomNumberGenerator;

use crate::{spawner, Map, Position, Rect, TileType, SHOW_MAPGEN_VISUALIZER};

use super::{common, MapBuilder};

const MIN_ROOM_SIZE: i32 = 8;

pub struct CellularAutomataBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
}

impl MapBuilder for CellularAutomataBuilder {
    fn get_map(&mut self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> Position {
        self.starting_position.clone()
    }

    fn get_snapshot_history(&self) -> Vec<Map> {
        self.history.clone()
    }

    fn build_map(&mut self) {
        let mut rng = RandomNumberGenerator::new();
        // Initialize random map
        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let roll = rng.roll_dice(1, 100);
                let idx = self.map.xy_idx(x, y);
                if roll > 55 {
                    self.map.tiles[idx] = TileType::Floor;
                } else {
                    self.map.tiles[idx] = TileType::Wall;
                }
            }
        }
        self.take_snapshot();

        // Now iteratively refine by applying cellular automata rules
        for _iteration in 0..15 {
            let mut newtiles = self.map.tiles.clone();

            for y in 1..self.map.height - 1 {
                for x in 1..self.map.width - 1 {
                    let idx = self.map.xy_idx(x, y);
                    let neighbors = self.count_neighbors(idx);

                    if neighbors > 4 || neighbors == 0 {
                        newtiles[idx] = TileType::Wall;
                    } else {
                        newtiles[idx] = TileType::Floor;
                    }
                }
            }

            self.map.tiles = newtiles.clone();
            self.take_snapshot();
        }

        // Pick a starting position. Start at the middle and walk left until we find an open tile
        self.starting_position = Position {
            x: self.map.width / 2,
            y: self.map.height / 2,
        };
        let mut start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);
        while self.map.tiles[start_idx] != TileType::Floor {
            self.starting_position.x -= 1;
            start_idx = self
                .map
                .xy_idx(self.starting_position.x, self.starting_position.y);
        }

        let exit_tile_idx =
            common::cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);
        self.take_snapshot();

        // Place the stairs
        self.map.tiles[exit_tile_idx] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        self.noise_areas = common::generate_voronoi_spawn_regions(&self.map, &mut rng);
    }

    fn spawn_entities(&mut self, ecs: &mut specs::World) {
        for area in self.noise_areas.iter() {
            spawner::spawn_region(ecs, area.1, self.depth);
        }
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

impl CellularAutomataBuilder {
    pub fn new(new_depth: i32) -> CellularAutomataBuilder {
        CellularAutomataBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
        }
    }

    fn count_neighbors(&self, idx: usize) -> i32 {
        let mut neighbors = 0;

        if self.map.tiles[idx - 1] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx - self.map.width as usize] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx + self.map.width as usize] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx - (self.map.width - 1) as usize] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx - (self.map.width + 1) as usize] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx + (self.map.width - 1) as usize] == TileType::Wall {
            neighbors += 1;
        }
        if self.map.tiles[idx + (self.map.width + 1) as usize] == TileType::Wall {
            neighbors += 1;
        }

        return neighbors;
    }
}
