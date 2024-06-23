use std::collections::HashSet;

use crate::{Map, TileType};

/// Builds a list of all chunks extracted from `map`.
///
/// The provided chunk size is recommended to be a divisor of the map width and height.
///
/// This is a parametrized function by:
///
/// - `chunk_size`: determines the amount of tiles taken by each chunk and thus each pattern
/// - `include_flipped_chunks`: when set to true will include both the original chunks and the
/// flipped versions (horizontal, vertical and both)
/// - `dedup`: when set to true, removes duplicate patterns from the output list
pub fn build_patterns(
    map: &Map,
    chunk_size: i32,
    include_flipped_chunks: bool,
    dedup: bool,
) -> Vec<Vec<TileType>> {
    let chunks_on_x = map.width / chunk_size;
    let chunks_on_y = map.height / chunk_size;
    let mut patterns = Vec::new();

    for chunk_x in 0..chunks_on_x {
        for chunk_y in 0..chunks_on_y {
            let mut pattern = Vec::new();
            let start_x = chunk_x * chunk_size;
            let end_x = (chunk_x + 1) * chunk_size;
            let start_y = chunk_y * chunk_size;
            let end_y = (chunk_y + 1) * chunk_size;

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let tile_idx = map.xy_idx(x, y);
                    pattern.push(map.tiles[tile_idx])
                }
            }

            patterns.push(pattern);

            // Flipping
            if include_flipped_chunks {
                // Horizontal flip
                //
                // 1 2 3
                // 4 5 6
                // 7 8 9
                //
                // becomes:
                //
                // 3 2 1
                // 6 5 3
                // 9 8 7
                let mut pattern = Vec::new();
                for y in start_y..end_y {
                    for x in (start_x..end_x).rev() {
                        let tile_idx = map.xy_idx(x, y);
                        pattern.push(map.tiles[tile_idx])
                    }
                }
                patterns.push(pattern);

                // Vertical flip
                //
                // 1 2 3
                // 4 5 6
                // 7 8 9
                //
                // becomes:
                //
                // 7 8 9
                // 4 5 6
                // 1 2 3
                let mut pattern = Vec::new();
                for y in (start_y..end_y).rev() {
                    for x in start_x..end_x {
                        let tile_idx = map.xy_idx(x, y);
                        pattern.push(map.tiles[tile_idx])
                    }
                }
                patterns.push(pattern);

                // Both Horizontal and Vertical flip
                //
                // 1 2 3
                // 4 5 6
                // 7 8 9
                //
                // becomes:
                //
                // 9 8 7
                // 6 5 4
                // 3 2 1
                let mut pattern = Vec::new();
                for y in (start_y..end_y).rev() {
                    for x in (start_x..end_x).rev() {
                        let tile_idx = map.xy_idx(x, y);
                        pattern.push(map.tiles[tile_idx])
                    }
                }
                patterns.push(pattern);
            }
        }
    }

    // De-duplication
    if dedup {
        rltk::console::log(format!(
            "Pre de-duplication, there are {} patterns",
            patterns.len()
        ));
        let patterns_set: HashSet<Vec<TileType>> = patterns.drain(..).collect();
        patterns.extend(patterns_set.into_iter());
        rltk::console::log(format!(
            "Post de-duplication, there are {} patterns",
            patterns.len()
        ));
    }

    patterns
}

/// Writes the chunk identified by `pattern` into the map at `(start_x, start_y)`
pub fn render_pattern_to_map(
    map: &mut Map,
    pattern: &Vec<TileType>,
    chunk_size: i32,
    start_x: i32,
    start_y: i32,
) {
    let mut i = 0usize;
    for tile_y in 0..chunk_size {
        for tile_x in 0..chunk_size {
            let map_idx = map.xy_idx(start_x + tile_x, start_y + tile_y);
            map.tiles[map_idx] = pattern[i];
            map.visible_tiles[map_idx] = true;
            i += 1;
        }
    }
}

/// Writes the chunk identified by `pattern` into the map at `(start_x, start_y)`
pub fn render_chunk_to_map(
    map: &mut Map,
    chunk: &MapChunk,
    chunk_size: i32,
    start_x: i32,
    start_y: i32,
) {
    let mut i = 0usize;
    for tile_y in 0..chunk_size {
        for tile_x in 0..chunk_size {
            let map_idx = map.xy_idx(start_x + tile_x, start_y + tile_y);
            map.tiles[map_idx] = chunk.pattern[i];
            map.visible_tiles[map_idx] = true;
            i += 1;
        }
    }

    // Draw exits (doesn't tell us about compatibility between chunks directly but does indirectly)
    for (x, northbound) in chunk.exits[0].iter().enumerate() {
        if *northbound {
            let map_idx = map.xy_idx(start_x + x as i32, start_y);
            map.tiles[map_idx] = TileType::Debug('E');
        }
    }

    for (x, southbound) in chunk.exits[1].iter().enumerate() {
        if *southbound {
            let map_idx = map.xy_idx(start_x + x as i32, start_y + chunk_size - 1);
            map.tiles[map_idx] = TileType::Debug('E');
        }
    }

    for (y, westbound) in chunk.exits[2].iter().enumerate() {
        if *westbound {
            let map_idx = map.xy_idx(start_x, start_y + y as i32);
            map.tiles[map_idx] = TileType::Debug('E');
        }
    }

    for (y, eastbound) in chunk.exits[3].iter().enumerate() {
        if *eastbound {
            let map_idx = map.xy_idx(start_x + chunk_size - 1, start_y + y as i32);
            map.tiles[map_idx] = TileType::Debug('E');
        }
    }
}

/// A `MapChunk` is a processed chunk containing extra information for adjacency constraints
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct MapChunk {
    pub pattern: Vec<TileType>,
    /// `exits` tells us which tiles are "exit" (a.k.a. floor) tiles in any of the 4 chunk borders
    pub exits: [Vec<bool>; 4],
    /// `has_exits` is a faster way to check whether any tile at the border is an exit tile
    pub has_exits: bool,
    /// `compatible_with` has, for each of the 4 borders, the indices of the chunks compatible with
    /// this one
    pub compatible_with: [Vec<usize>; 4],
}

/// Builds a [MapChunk] list out of the provided pattern list.
pub fn patterns_to_constraints(patterns: Vec<Vec<TileType>>, chunk_size: i32) -> Vec<MapChunk> {
    // Build `MapChunk` objects without the adjacency compatibility info
    let mut constraints: Vec<MapChunk> = Vec::new();

    for p in patterns {
        let mut new_chunk = MapChunk {
            pattern: p,
            exits: [
                vec![false; chunk_size as usize],
                vec![false; chunk_size as usize],
                vec![false; chunk_size as usize],
                vec![false; chunk_size as usize],
            ],
            has_exits: false,
            compatible_with: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        };

        // Load northern exits
        for x in 0..chunk_size {
            let tile_idx = tile_idx_in_chunk(x, 0, chunk_size);
            new_chunk.exits[0][x as usize] = new_chunk.pattern[tile_idx] == TileType::Floor;
            new_chunk.has_exits = new_chunk.has_exits || new_chunk.exits[0][x as usize]
        }

        // Load southern exits
        for x in 0..chunk_size {
            let tile_idx = tile_idx_in_chunk(x, chunk_size - 1, chunk_size);
            new_chunk.exits[1][x as usize] = new_chunk.pattern[tile_idx] == TileType::Floor;
            new_chunk.has_exits = new_chunk.has_exits || new_chunk.exits[1][x as usize]
        }

        // Load western exits
        for y in 0..chunk_size {
            let tile_idx = tile_idx_in_chunk(0, y, chunk_size);
            new_chunk.exits[2][y as usize] = new_chunk.pattern[tile_idx] == TileType::Floor;
            new_chunk.has_exits = new_chunk.has_exits || new_chunk.exits[2][y as usize]
        }

        // Load eastern exits
        for y in 0..chunk_size {
            let tile_idx = tile_idx_in_chunk(chunk_size - 1, y, chunk_size);
            new_chunk.exits[3][y as usize] = new_chunk.pattern[tile_idx] == TileType::Floor;
            new_chunk.has_exits = new_chunk.has_exits || new_chunk.exits[3][y as usize]
        }

        constraints.push(new_chunk);
    }

    // Update compatibility status: If both tiles have exits, we say they are compatible in some
    // border if either some has no exit tiles or both have it
    let constraints_duplicate = constraints.clone();

    for constraint in constraints.iter_mut() {
        for (chunk_idx, potential) in constraints_duplicate.iter().enumerate() {
            // If either have no exits we know for sure they are compatible
            if !constraint.has_exits || !potential.has_exits {
                for c in constraint.compatible_with.iter_mut() {
                    c.push(chunk_idx);
                }
            } else {
                // Check compatibility based on direction
                for (direction, exit_list) in constraint.exits.iter_mut().enumerate() {
                    let opposite = match direction {
                        0 => 1, // North should match with the south of the other one.
                        1 => 0, // South should match with the north of the other one.
                        2 => 3, // West should match with the east of the other one.
                        _ => 2, // East should match with the west of the other one.
                    };

                    let mut does_fit = false;
                    let mut has_exit_on_this_side = false;

                    for (border_idx, can_enter) in exit_list.iter().enumerate() {
                        has_exit_on_this_side = has_exit_on_this_side || *can_enter;
                        if *can_enter && potential.exits[opposite][border_idx] {
                            does_fit = true;
                        }
                    }

                    // If this tile has an exit (and the other one matches an exit on the border)
                    // or if this border has no exits, then they're compatible
                    if does_fit || !has_exit_on_this_side {
                        constraint.compatible_with[direction].push(chunk_idx);
                    }
                }
            }
        }
    }

    constraints
}

#[inline]
fn tile_idx_in_chunk(x: i32, y: i32, chunk_size: i32) -> usize {
    (y * chunk_size + x) as usize
}
