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
