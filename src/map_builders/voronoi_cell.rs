use std::collections::HashMap;

use crate::map_builders::common;
use crate::{spawner, Position, TileType, SHOW_MAPGEN_VISUALIZER};

use super::Map;
use super::MapBuilder;
use rltk::RandomNumberGenerator;
use specs::World;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum DistanceAlgorithm {
    Pythagoras,
    Manhattan,
    Chebyshev,
}

pub struct VoronoiCellBuilder {
    map: Map,
    starting_position: Position,
    depth: i32,
    noise_areas: HashMap<i32, Vec<usize>>,
    history: Vec<Map>,
    n_seeds: usize,
    distance_algorithm: DistanceAlgorithm,
}

impl MapBuilder for VoronoiCellBuilder {
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

impl VoronoiCellBuilder {
    pub fn pythagoras(new_depth: i32) -> VoronoiCellBuilder {
        VoronoiCellBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            noise_areas: HashMap::new(),
            history: Vec::new(),
            n_seeds: 64,
            distance_algorithm: DistanceAlgorithm::Pythagoras,
        }
    }

    pub fn manhattan(new_depth: i32) -> VoronoiCellBuilder {
        VoronoiCellBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            noise_areas: HashMap::new(),
            history: Vec::new(),
            n_seeds: 64,
            distance_algorithm: DistanceAlgorithm::Manhattan,
        }
    }

    pub fn chebyshev(new_depth: i32) -> VoronoiCellBuilder {
        VoronoiCellBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth,
            noise_areas: HashMap::new(),
            history: Vec::new(),
            n_seeds: 32,
            distance_algorithm: DistanceAlgorithm::Chebyshev,
        }
    }

    /// Builds a Voronoi Diagram which ends up being the map
    fn build(&mut self) {
        let mut rng = RandomNumberGenerator::new();
        // Select `n_seeds` random positions in the map. We'll use `rltk::Point` since we can reuse
        // it with `rltk::DistanceAlg`
        let n_seeds = self.n_seeds;
        let mut voronoi_seeds: Vec<(usize, rltk::Point)> = Vec::with_capacity(n_seeds);

        while voronoi_seeds.len() < n_seeds {
            let random_x = rng.roll_dice(1, self.map.width - 3) + 1;
            let random_y = rng.roll_dice(1, self.map.height - 3) + 1;
            let seed_idx = self.map.xy_idx(random_x, random_y);

            // Ignore duplicates
            if voronoi_seeds
                .iter()
                .find(|(existing_seed_idx, _)| *existing_seed_idx == seed_idx)
                .is_none()
            {
                voronoi_seeds.push((seed_idx, rltk::Point::new(random_x, random_y)));
            }
        }

        // For each point in the map, set as its region the one represented by the closest initial
        // random position
        let mut voronoi_regions: Vec<usize> =
            vec![0; self.map.width as usize * self.map.height as usize];

        for (tile_idx, tile_region) in voronoi_regions.iter_mut().enumerate() {
            let tile_x = tile_idx as i32 % self.map.width;
            let tile_y = tile_idx as i32 / self.map.width;
            let tile_point = rltk::Point::new(tile_x, tile_y);
            let closest_region = voronoi_seeds
                .iter()
                .enumerate()
                .map(|(region, (_, seed_point))| {
                    let distance = match self.distance_algorithm {
                        DistanceAlgorithm::Pythagoras => {
                            rltk::DistanceAlg::PythagorasSquared.distance2d(tile_point, *seed_point)
                        }
                        DistanceAlgorithm::Manhattan => {
                            rltk::DistanceAlg::Manhattan.distance2d(tile_point, *seed_point)
                        }
                        DistanceAlgorithm::Chebyshev => {
                            rltk::DistanceAlg::Chebyshev.distance2d(tile_point, *seed_point)
                        }
                    };
                    (region, distance)
                })
                .min_by(|(_, distance1), (_, distance2)| {
                    (*distance1)
                        .partial_cmp(distance2)
                        .expect("Should be able to compare since both are supposed to be non-Nan")
                })
                .expect("Should always have a minimum");

            *tile_region = closest_region.0;
        }

        // Now, for each point in the map, count the amount of neighbors of different region. If
        // there are none, it's safe to say it's a floor tile. We'll also do the same if there is
        // only one to ensure all regions stay connected. Otherwise it stays as a wall.
        for y in 1..self.map.height - 1 {
            for x in 1..self.map.width - 1 {
                let tile_idx = self.map.xy_idx(x, y);
                let tile_region = voronoi_regions[tile_idx];

                let mut neighbors_count = 0;

                neighbors_count += (voronoi_regions[tile_idx + 1] != tile_region) as usize;
                neighbors_count += (voronoi_regions[tile_idx - 1] != tile_region) as usize;
                neighbors_count +=
                    (voronoi_regions[tile_idx + self.map.width as usize] != tile_region) as usize;
                neighbors_count +=
                    (voronoi_regions[tile_idx - self.map.width as usize] != tile_region) as usize;

                if neighbors_count < 2 {
                    self.map.tiles[tile_idx] = TileType::Floor;
                }
            }
            self.take_snapshot();
        }

        // Last part of setup, the exit and spawn info
        self.starting_position = Position {
            x: voronoi_seeds[0].1.x,
            y: voronoi_seeds[0].1.y,
        };
        let start_idx = voronoi_seeds[0].0;
        let stairs_idx =
            common::cull_unreachables_and_return_most_distant_tile(&mut self.map, start_idx);

        // Place the stairs
        self.map.tiles[stairs_idx] = TileType::DownStairs;
        self.take_snapshot();

        // Now build a noise map for use later when spawning entities
        self.noise_areas = common::generate_voronoi_spawn_regions(&self.map, &mut rng);
    }
}
