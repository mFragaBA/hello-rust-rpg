use std::collections::HashMap;

use rltk::RandomNumberGenerator;

use crate::{
    map_builders::wave_function_collapse::constraints::patterns_to_constraints, spawner, Map,
    Position, TileType, SHOW_MAPGEN_VISUALIZER,
};

use self::constraints::{render_chunk_to_map, render_pattern_to_map, MapChunk};

use super::{common, MapBuilder};

mod image_loader;
use image_loader::load_rex_map;
mod constraints;
use constraints::build_patterns;
mod solver;
use solver::Solver;

pub struct WaveFunctionCollapseBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    history: Vec<Map>,
    noise_areas: HashMap<i32, Vec<usize>>,
}

impl MapBuilder for WaveFunctionCollapseBuilder {
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
        self.build()
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

impl WaveFunctionCollapseBuilder {
    pub fn new(new_depth: i32) -> WaveFunctionCollapseBuilder {
        WaveFunctionCollapseBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            history: Vec::new(),
            noise_areas: HashMap::new(),
        }
    }

    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();

        // TODO: build process goes here
        self.map = load_rex_map(
            self.depth,
            &rltk::rex::XpFile::from_resource("../resources/wfc-demo1.xp").unwrap(),
        );
        self.take_snapshot();

        // The amount of tiles that conform a chunk (we use chunks to generate adjacency rules)
        const CHUNK_SIZE: i32 = 7;

        // Carve Patterns
        let patterns = build_patterns(&self.map, CHUNK_SIZE, true, true);

        self.render_tile_gallery(&patterns, CHUNK_SIZE);

        let constraints = patterns_to_constraints(patterns, CHUNK_SIZE);

        self.render_constraint_gallery(&constraints, CHUNK_SIZE);

        // Now actually write the map
        self.map = Map::new(self.depth);
        loop {
            let mut solver = Solver::new(constraints.clone(), CHUNK_SIZE, &self.map);
            while !solver.step(&mut self.map, &mut rng) {
                self.take_snapshot();
            }
            self.take_snapshot();

            // If it's stuck at an impossible condition, try again. Otherwise exit
            if solver.possible { break; }
        }

        // Pick a starting position. Start at the middle and walk left until we find an open tile
        self.starting_position = Position {
            x: self.map.width / 2,
            y: self.map.height / 2,
        };
        let start_idx = self
            .map
            .xy_idx(self.starting_position.x, self.starting_position.y);

        let exit_tile_idx =
            common::cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);
        self.take_snapshot();

        // Place the stairs
        self.map.tiles[exit_tile_idx] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        self.noise_areas = common::generate_voronoi_spawn_regions(&self.map, &mut rng);
    }

    /// Renders all tiles from `patterns` into the snapshotted map
    ///
    /// It tries fitting as many patterns as possible per row and as many rows per map. If exceeded
    /// it starts from a fresh "gallery page" a.k.a. a new map.
    fn render_tile_gallery(&mut self, patterns: &Vec<Vec<TileType>>, chunk_size: i32) {
        self.map = Map::new(0);
        let mut counter = 0;
        let mut x = 1;
        let mut y = 1;
        while counter < patterns.len() {
            render_pattern_to_map(&mut self.map, &patterns[counter], chunk_size, x, y);

            x += chunk_size + 1;
            if x + chunk_size >= self.map.width {
                // Move to next row
                x = 1;
                y += chunk_size + 1;

                if y + chunk_size >= self.map.height {
                    self.take_snapshot();
                    self.map = Map::new(0);

                    x = 1;
                    y = 1;
                }
            }

            counter += 1;
        }

        self.take_snapshot();
    }

    /// Renders all chunks from `constraints` into the snapshotted map
    ///
    /// It tries fitting as many as possible per row and as many rows per map. If exceeded
    /// it starts from a fresh "gallery page" a.k.a. a new map.
    fn render_constraint_gallery(&mut self, constraints: &Vec<MapChunk>, chunk_size: i32) {
        self.map = Map::new(0);
        let mut counter = 0;
        let mut x = 1;
        let mut y = 1;
        while counter < constraints.len() {
            render_chunk_to_map(&mut self.map, &constraints[counter], chunk_size, x, y);

            x += chunk_size + 1;
            if x + chunk_size >= self.map.width {
                // Move to next row
                x = 1;
                y += chunk_size + 1;

                if y + chunk_size >= self.map.height {
                    self.take_snapshot();
                    self.map = Map::new(0);

                    x = 1;
                    y = 1;
                }
            }

            counter += 1;
        }

        self.take_snapshot();
    }
}
