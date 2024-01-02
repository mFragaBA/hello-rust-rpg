use std::collections::HashMap;

use rltk::RandomNumberGenerator;

use crate::{spawner, Map, Position, Rect, TileType, SHOW_MAPGEN_VISUALIZER};

use super::{common, MapBuilder};

const MIN_ROOM_SIZE: i32 = 8;

#[derive(PartialEq, Copy, Clone)]
pub enum DrunkSpawnMode {
    StartingPoint,
    Random,
}

pub struct DrunkardSettings {
    pub spawn_mode: DrunkSpawnMode,
    pub drunken_lifetime: i32,
    pub floor_percent: f32
}

pub struct DrunkardsWalkBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
    settings: DrunkardSettings,
}

impl MapBuilder for DrunkardsWalkBuilder {
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
        self.build();
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

impl DrunkardsWalkBuilder {
    pub fn new(new_depth: i32, settings: DrunkardSettings) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            settings,
        }
    }

    pub fn open_area(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            settings: DrunkardSettings { spawn_mode: DrunkSpawnMode::StartingPoint, drunken_lifetime: 400, floor_percent: 0.5 },
        }
    }

    pub fn open_halls(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            settings: DrunkardSettings { spawn_mode: DrunkSpawnMode::Random, drunken_lifetime: 400, floor_percent: 0.5 },
        }
    }

    pub fn widening_passages(new_depth: i32) -> DrunkardsWalkBuilder {
        DrunkardsWalkBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
            settings: DrunkardSettings { spawn_mode: DrunkSpawnMode::Random, drunken_lifetime: 100, floor_percent: 0.4 },
        }
    }

    /// The basic idea behind the algorithm is simple:
    /// Pick a central starting point, and convert it to a floor.
    /// We count how much of the map is floor space, and iterate until we have converted a percentage (we use 50% in the example) of the map to floors.
    ///     Spawn a drunkard at the starting point. The drunkard has a "lifetime" and a "position".
    ///     While the drunkard is still alive:
    ///         Decrement the drunkard's lifetime (I like to think that they pass out and sleep).
    ///         Roll a 4-sided dice.
    ///             If we rolled a 1, move the drunkard North.
    ///             If we rolled a 2, move the drunkard South.
    ///             If we rolled a 3, move the drunkard East.
    ///             If we rolled a 4, move the drunkard West.
    ///         The tile on which the drunkard landed becomes a floor.
    #[allow(clippy::map_entry)]
    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        // Set a central starting point
        self.starting_position = Position {
            x: self.map.width / 2,
            y: self.map.height / 2,
        };
        let start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);
        self.map.tiles[start_idx] = TileType::Floor;

        let total_tiles = self.map.width * self.map.height;
        let desired_floor_tiles = (total_tiles as f32 * self.settings.floor_percent) as usize;
        let mut floor_tile_count = 1; // The starting position one
        let mut digger_count = 0;
        let mut active_digger_count = 0;

        while floor_tile_count < desired_floor_tiles {
            let mut did_something = false;
            let mut drunk_x;
            let mut drunk_y;
            let mut drunk_lifetime = self.settings.drunken_lifetime;

            match self.settings.spawn_mode {
                DrunkSpawnMode::StartingPoint => {
                    drunk_x = self.starting_position.x;
                    drunk_y = self.starting_position.y;
                }
                DrunkSpawnMode::Random => {
                    drunk_x = rng.roll_dice(1, self.map.width - 3) + 1;
                    drunk_y = rng.roll_dice(1, self.map.height - 3) + 1;
                }
            }

            while drunk_lifetime > 0 {
                let drunk_pos_id = self.map.xy_idx(drunk_x, drunk_y);
                if self.map.tiles[drunk_pos_id] == TileType::Wall {
                    did_something = true;
                }
                self.map.tiles[drunk_pos_id] = TileType::DownStairs;

                let rolled_direction = rng.roll_dice(1, 4);
                match rolled_direction {
                    1 if drunk_x > 2 => drunk_x -= 1,
                    2 if drunk_x < self.map.width - 2 => drunk_x += 1,
                    3 if drunk_y > 2 => drunk_y -= 1,
                    4 if drunk_y < self.map.height - 2 => drunk_y += 1,
                    _ => {}
                }

                drunk_lifetime -= 1;
            }

            if did_something {
                self.take_snapshot();
                active_digger_count += 1;
            }

            digger_count += 1;
            for t in self.map.tiles.iter_mut() {
                if *t == TileType::DownStairs {
                    *t = TileType::Floor;
                }
            }
            floor_tile_count = self
                .map
                .tiles
                .iter()
                .filter(|a| **a == TileType::Floor)
                .count();
        }
        rltk::console::log(format!(
            "{} dwarves gave up their sobriety, of whom {} actually found a wall.",
            digger_count, active_digger_count
        ));

        let exit_tile_idx =
            common::cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);
        self.take_snapshot();

        // Place the stairs
        self.map.tiles[exit_tile_idx] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        self.noise_areas = common::generate_voronoi_spawn_regions(&self.map, &mut rng);
    }
}
