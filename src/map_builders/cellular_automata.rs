use std::collections::HashMap;

use rltk::RandomNumberGenerator;

use crate::{spawner, Map, Position, Rect, SHOW_MAPGEN_VISUALIZER, TileType};

use super::MapBuilder;

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
        for y in 1..self.map.height-1 {
            for x in 1..self.map.width-1 {
                let roll = rng.roll_dice(1, 100);
                let idx = self.map.xy_idx(x, y);
                if roll > 55 { self.map.tiles[idx] = TileType::Floor; }
                else { self.map.tiles[idx] = TileType::Wall; }
            }
        }
        self.take_snapshot();

        // Now iteratively refine by applying cellular automata rules
        for _iteration in 0..15 {
            let mut newtiles = self.map.tiles.clone();

            for y in 1..self.map.height-1 {
                for x in 1..self.map.width-1 {
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
        self.starting_position = Position { x: self.map.width / 2, y: self.map.height / 2 };
        let mut start_idx = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        while self.map.tiles[start_idx] != TileType::Floor {
            self.starting_position.x -= 1;
            start_idx = self.map.xy_idx(self.starting_position.x, self.starting_position.y);
        }

        // This is so unreachable tiles are actually unreachable. Otherwise wall tiles wouldn't be
        // marked as blocked
        self.map.populate_blocked();

        // Find all tiles reachable from the starting point
        let map_starts : Vec<usize> = vec![start_idx];
        let dijkstra_map = rltk::DijkstraMap::new(self.map.width, self.map.height, &map_starts, &self.map, 200.0);
        let mut exit_tile = (0, 0.0f32);
        for (i, tile) in self.map.tiles.iter_mut().enumerate() {
            if *tile == TileType::Floor {
                let distance_to_start = dijkstra_map.map[i];
                if distance_to_start == std::f32::MAX {
                    *tile = TileType::Wall;
                } else {
                    if distance_to_start > exit_tile.1 {
                        exit_tile.0 = i;
                        exit_tile.1 = distance_to_start;
                    }
                }
            }
        }
        self.take_snapshot();

        self.map.tiles[exit_tile.0] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
        noise.set_frequency(0.08);
        noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Manhattan);

        for y in 1 .. self.map.height - 1 {
            for x in 1 .. self.map.width - 1 {
                let idx = self.map.xy_idx(x, y);
                if self.map.tiles[idx] == TileType::Floor {
                    // On the tutorial it uses 10240.0 but using it results in ~1k areas being
                    // created instead of the 20-30 it claims
                    let cell_value_f = noise.get_noise(x as f32, y as f32) * 15.0;
                    let cell_value = cell_value_f as i32;

                    if self.noise_areas.contains_key(&cell_value) {
                        self.noise_areas.get_mut(&cell_value).unwrap().push(idx);
                    } else {
                        self.noise_areas.insert(cell_value, vec![idx]);
                    }
                }
            }
        }
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
        let mut neighbors = 0 ;

        if self.map.tiles[idx - 1] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx - self.map.width as usize] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx + self.map.width as usize] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx - (self.map.width - 1) as usize] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx - (self.map.width + 1) as usize] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx + (self.map.width - 1) as usize] == TileType::Wall { neighbors += 1; }
        if self.map.tiles[idx + (self.map.width + 1) as usize] == TileType::Wall { neighbors += 1; }

        return neighbors;
    }
}
