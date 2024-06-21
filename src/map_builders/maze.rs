use std::collections::HashMap;

use crate::{spawner, Position, SHOW_MAPGEN_VISUALIZER};

use super::common::cull_unreachables_and_return_most_distant_tile;
use super::common::generate_voronoi_spawn_regions;
use super::MapBuilder;
use super::{Map, TileType};
use rltk::RandomNumberGenerator;
use specs::World;

pub struct MazeBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
}

impl MapBuilder for MazeBuilder {
    fn build_map(&mut self) {
        self.build();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for area in self.noise_areas.iter().skip(1) {
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

const NORTH: u8 = 0b00000001;
const SOUTH: u8 = 0b00000010;
const WEST: u8 = 0b00000100;
const EAST: u8 = 0b00001000;

const CELL_COUNT_BEFORE_SNAPSHOT: u64 = 20;

impl MazeBuilder {
    pub fn new(new_depth: i32) -> MazeBuilder {
        MazeBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        MazeGrid::new_from_map(self, &mut rng);

        // Since we start at (0, 0) in the grid which is half our map size, we start at 2 * (0, 0) + (1, 1) = (1, 1) in
        // the actual map
        self.starting_position = Position { x: 1, y: 1 };
        let start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);

        // // Find all tiles we can reach from the starting point
        let exit_tile = cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);
        self.take_snapshot();

        // // Place the stairs
        self.map.tiles[exit_tile] = TileType::DownStairs;
        self.take_snapshot();

        // // Now build a noise map for use in spawning entities later
        self.noise_areas = generate_voronoi_spawn_regions(&self.map, &mut rng);
    }
}

struct MazeGrid {
    pub grid_width: i32,
    pub grid_height: i32,
    // Walls is an array representing, for each tile on the map it's 4 "walls" that conect it
    // to neighboring tiles and whether that wall has been closed. we can represent this with 4
    // bits out of the 8 from the u8. Each bit represents the crossed status of south, east,
    // north and west wall.
    pub walls: Vec<u8>,
    is_visited: Vec<bool>,
}

impl MazeGrid {
    // The following maze generation assumes that walls are tiles that surround the actual
    // tiles. So we'll run it on a grid half the size of the actual map and we then double the
    // resolution assuming all other tiles are walls.
    fn new_from_map(builder: &mut MazeBuilder, rng: &mut RandomNumberGenerator) -> Self {
        let grid_width = (builder.map.width - 2) / 2;
        let grid_height = (builder.map.height - 2) / 2;
        let grid_size = (grid_width * grid_height) as usize;

        let walls: Vec<u8> = vec![0; grid_size];
        let is_visited: Vec<bool> = vec![false; grid_size];
        let grid = Self {
            grid_width,
            grid_height,
            walls,
            is_visited,
        };

        grid.generate(builder, rng)
    }

    #[inline]
    fn xy_idx(&self, pos: Position) -> i32 {
        if pos.x >= 0 && pos.x < self.grid_width && pos.y >= 0 && pos.y < self.grid_height {
            (pos.y * self.grid_width) + pos.x
        } else {
            -1
        }
    }

    fn connect_tiles(&mut self, from_pos: Position, to_pos: Position) {
        let x_diff = from_pos.x - to_pos.x;
        let y_diff = from_pos.y - to_pos.y;
        let from_idx = self.xy_idx(from_pos);
        let to_idx = self.xy_idx(to_pos);

        match (x_diff, y_diff) {
            (1, _) => {
                self.walls[from_idx as usize] |= WEST;
                self.walls[to_idx as usize] |= EAST;
            }
            (-1, _) => {
                self.walls[from_idx as usize] |= EAST;
                self.walls[to_idx as usize] |= WEST;
            }
            (_, 1) => {
                self.walls[from_idx as usize] |= NORTH;
                self.walls[to_idx as usize] |= SOUTH;
            }
            (_, -1) => {
                self.walls[from_idx as usize] |= SOUTH;
                self.walls[to_idx as usize] |= NORTH;
            }
            _ => { /* SHOULDN'T BE HERE */ }
        }
    }

    fn available_neighbors(&self, pos: Position) -> Vec<Position> {
        let mut available_neighs = vec![];

        let neighbors = [
            Position {
                x: pos.x + 1,
                y: pos.y,
            },
            Position {
                x: pos.x - 1,
                y: pos.y,
            },
            Position {
                x: pos.x,
                y: pos.y + 1,
            },
            Position {
                x: pos.x,
                y: pos.y - 1,
            },
        ];

        for pos in neighbors {
            let pos_idx = self.xy_idx(pos);
            if pos_idx != -1 && !self.is_visited[pos_idx as usize] {
                available_neighs.push(pos);
            }
        }

        available_neighs
    }

    fn next_neighbor(&self, pos: Position, rng: &mut RandomNumberGenerator) -> Option<Position> {
        let neighbors = self.available_neighbors(pos);
        if neighbors.len() > 0 {
            Some(neighbors[(rng.roll_dice(1, neighbors.len() as i32) - 1) as usize])
        } else {
            None
        }
    }

    #[inline]
    fn reset(&mut self) {
        let grid_size = (self.grid_width * self.grid_height) as usize;
        self.walls = vec![0; grid_size];
        self.is_visited = vec![false; grid_size];
    }

    fn generate(mut self, builder: &mut MazeBuilder, rng: &mut RandomNumberGenerator) -> Self {
        self.reset();
        let mut cell_count: u64 = 0;
        let start_pos = Position { x: 0, y: 0 };
        let mut stack: Vec<Position> = Vec::new();
        stack.push(start_pos);

        while let Some(current_pos) = stack.last().copied() {
            let idx = self.xy_idx(current_pos);
            self.is_visited[idx as usize] = true;
            if let Some(neigh) = self.next_neighbor(current_pos, rng) {
                stack.push(neigh);
                self.connect_tiles(current_pos, neigh);

                // add 1 for padding so we don't write over borders
                let map_idx = builder
                    .map
                    .xy_idx(current_pos.x * 2 + 1, current_pos.y * 2 + 1);

                builder.map.tiles[map_idx] = TileType::Floor;
                // account for tore down walls
                if self.walls[idx as usize] & NORTH != 0 {
                    builder.map.tiles[map_idx - builder.map.width as usize] = TileType::Floor;
                }
                if self.walls[idx as usize] & SOUTH != 0 {
                    builder.map.tiles[map_idx + builder.map.width as usize] = TileType::Floor;
                }
                if self.walls[idx as usize] & WEST != 0 {
                    builder.map.tiles[map_idx - 1] = TileType::Floor;
                }
                if self.walls[idx as usize] & EAST != 0 {
                    builder.map.tiles[map_idx + 1] = TileType::Floor;
                }

                cell_count += 1;
                if cell_count % CELL_COUNT_BEFORE_SNAPSHOT == 0 {
                    builder.take_snapshot();
                }
            } else {
                stack.pop();
            }
        }

        self
    }
}
