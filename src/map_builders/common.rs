use super::{Map, Rect, TileType};
use std::{
    cmp::{max, min},
    collections::HashMap,
};

pub fn apply_room_to_map(map: &mut Map, room: &Rect) {
    for y in (room.y1 + 1)..=room.y2 {
        for x in (room.x1 + 1)..=room.x2 {
            let idx = map.xy_idx(x, y);
            map.tiles[idx] = TileType::Floor;
        }
    }
}

pub fn apply_horizontal_tunnel(map: &mut Map, x1: i32, x2: i32, y: i32) {
    for x in min(x1, x2)..=max(x1, x2) {
        let idx = map.xy_idx(x, y);
        if idx > 0 && idx < (map.width * map.height) as usize {
            map.tiles[idx as usize] = TileType::Floor;
        }
    }
}

pub fn apply_vertical_tunnel(map: &mut Map, y1: i32, y2: i32, x: i32) {
    for y in min(y1, y2)..=max(y1, y2) {
        let idx = map.xy_idx(x, y);
        if idx > 0 && idx < (map.width * map.height) as usize {
            map.tiles[idx as usize] = TileType::Floor;
        }
    }
}

pub fn cull_unreachables_and_return_most_distant_tile(map: &mut Map, start_idx: usize) -> usize {
    map.populate_blocked();

    // Find all tiles reachable from the starting point
    let map_starts: Vec<usize> = vec![start_idx];
    let dijkstra_map = rltk::DijkstraMap::new(map.width, map.height, &map_starts, map, 200.0);
    let mut exit_tile = (0, 0.0f32);
    for (i, tile) in map.tiles.iter_mut().enumerate() {
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

    exit_tile.0
}

pub fn generate_voronoi_spawn_regions(
    map: &Map,
    rng: &mut rltk::RandomNumberGenerator,
) -> HashMap<i32, Vec<usize>> {
    let mut noise_areas: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut noise = rltk::FastNoise::seeded(rng.roll_dice(1, 65536) as u64);
    noise.set_noise_type(rltk::NoiseType::Cellular);
    noise.set_frequency(0.08);
    noise.set_cellular_distance_function(rltk::CellularDistanceFunction::Manhattan);

    for y in 1..map.height - 1 {
        for x in 1..map.width - 1 {
            let idx = map.xy_idx(x, y);
            if map.tiles[idx] == TileType::Floor {
                // On the tutorial it uses 10240.0 but using it results in ~1k areas being
                // created instead of the 20-30 it claims
                let cell_value_f = noise.get_noise(x as f32, y as f32) * 15.0;
                let cell_value = cell_value_f as i32;

                if noise_areas.contains_key(&cell_value) {
                    noise_areas.get_mut(&cell_value).unwrap().push(idx);
                } else {
                    noise_areas.insert(cell_value, vec![idx]);
                }
            }
        }
    }

    noise_areas
}
