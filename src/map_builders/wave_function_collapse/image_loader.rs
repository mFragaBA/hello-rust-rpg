use rltk::XpFile;

use crate::{Map, TileType};

/// Loads a RexPaint file, and converts it to our [Map] format
pub fn load_rex_map(new_depth: i32, xp_file: &XpFile) -> Map {
    let mut map: Map = Map::new(new_depth);

    for layer in &xp_file.layers {
        for y in 0..layer.height {
            for x in 0..layer.width {
                let cell = layer.get(x, y).expect("Should be able to index layer in bounds");

                // Check we're in-bounds
                if x < map.width as usize && y < map.height as usize {
                    let idx = map.xy_idx(x as i32, y as i32);

                    // decoding [cp437 encoding](https://en.wikipedia.org/wiki/Code_page_437#Character_set)
                    match cell.ch {
                        32 => map.tiles[idx] = TileType::Floor,
                        35 => map.tiles[idx] = TileType::Wall,
                        _ => { /* do nothing */ },
                    }
                }
            }
        }
    }

    map
}
