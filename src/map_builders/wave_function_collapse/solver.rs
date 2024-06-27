use std::collections::HashSet;

use rltk::RandomNumberGenerator;

use crate::Map;

use super::constraints::MapChunk;

pub struct Solver {
    constraints: Vec<MapChunk>,
    chunk_size: i32,
    // chunks will be `None` if it hasn't been set yet. Otherwise the `usize` will point to a chunk
    // in `constraints`
    chunks: Vec<Option<usize>>,
    // Number of chunks it can fit (horizontally)
    chunks_x: usize,
    // Number of chunks it can fit (vertically)
    chunks_y: usize,
    // (index, # neighbors)
    remaining: Vec<(usize, i32)>,
    pub possible: bool,
}

impl Solver {
    pub fn new(constraints: Vec<MapChunk>, chunk_size: i32, map: &Map) -> Solver {
        let chunks_x = (map.width / chunk_size) as usize;
        let chunks_y = (map.height / chunk_size) as usize;
        let mut remaining : Vec<(usize, i32)> = Vec::new();
        for i in 0..(chunks_x*chunks_y) {
            remaining.push((i, 0));
        }

        Solver {
            constraints,
            chunk_size,
            chunks: vec![None; chunks_x * chunks_y],
            chunks_x,
            chunks_y,
            remaining,
            possible: true,
        }
    }

    // Runs a single step of the wave function collapse algorithm.
    pub fn step(&mut self, map: &mut Map, rng: &mut RandomNumberGenerator) -> bool {
        if self.remaining.is_empty() { return true; }

        // Populate the neighbor count of the remaining list. This way we only count for the
        // remaining ones and not all of them
        let mut remaining_copy = self.remaining.clone();
        for r in remaining_copy.iter_mut() {
            let map_idx = r.0;
            let chunk_x = map_idx % self.chunks_x;
            let chunk_y = map_idx / self.chunks_x;
            let neighbor_count = self.count_neighbors(chunk_x, chunk_y);
            *r = (r.0, neighbor_count);
        }
        self.remaining = remaining_copy;

        let collapsed_remaining_chunk_index = (rng.roll_dice(1, self.remaining.len() as i32) - 1) as usize;
        let collapsed_chunk = self.remaining[collapsed_remaining_chunk_index].0;
        self.remaining.remove(collapsed_remaining_chunk_index);

        let chunk_x = collapsed_chunk % self.chunks_x;
        let chunk_y = collapsed_chunk / self.chunks_x;

        // A vec of MapChunk idxs. Candidates come from any of the neighboring chunks
        let mut candidate_chunks : Vec<HashSet<usize>> = Vec::new();

        // Try west
        if chunk_x > 0 {
            let west_idx = self.chunk_idx(chunk_x - 1, chunk_y);
            if let Some(west_chunk_idx) = self.chunks[west_idx] {
                // west tile checks compatibility against east one
                candidate_chunks.push(HashSet::from_iter(self.constraints[west_chunk_idx].compatible_with[3].iter().cloned()))
            };
        }

        // Try east
        if chunk_x < self.chunks_x - 1 {
            let east_idx = self.chunk_idx(chunk_x + 1, chunk_y);
            if let Some(east_chunk_idx) = self.chunks[east_idx] {
                // east tile checks compatibility against west one
                candidate_chunks.push(HashSet::from_iter(self.constraints[east_chunk_idx].compatible_with[2].iter().cloned()))
            }
        }

        // Try North
        if chunk_y > 0 {
            let north_idx = self.chunk_idx(chunk_x, chunk_y - 1);
            if let Some(north_chunk_idx) = self.chunks[north_idx] {
                // north tile checks compatibility against south one
                candidate_chunks.push(HashSet::from_iter(self.constraints[north_chunk_idx].compatible_with[1].iter().cloned()))
            }
        }

        // Try South
        if chunk_y < self.chunks_y - 1 {
            let south_idx = self.chunk_idx(chunk_x, chunk_y + 1);
            if let Some(south_chunk_idx) = self.chunks[south_idx] {
                // south tile checks compatibility against north
                candidate_chunks.push(HashSet::from_iter(self.constraints[south_chunk_idx].compatible_with[0].iter().cloned()))
            }
        }

        // If candidate_chunks is empty then that's because we have nothing around setup yet. So we
        // can pick any pattern we want
        let new_chunk_pattern_idx = if candidate_chunks.is_empty() {
            rng.roll_dice(1, self.constraints.len() as i32) - 1
        } else {
            // Compute the intersection of all compatibility list
            let candidates_intersection : HashSet<usize> = candidate_chunks
                .iter()
                .cloned()
                .fold(candidate_chunks[0].clone(), |acc, candidate_chunk| {
                    acc.intersection(&candidate_chunk).cloned().collect()
                });
            
            if candidates_intersection.is_empty() {
                rltk::console::log("F in the chat, we can't complete the map");
                self.possible = false;
                return true;
            } 

            // Pick a random chunk we havent collapsed yet and collapse it
            rng.roll_dice(1, candidates_intersection.len() as i32) - 1
        };

        // Insert the tiles in the map and set the chunk as taken
        let left_x = chunk_x as i32 * self.chunk_size as i32;
        let right_x = (chunk_x + 1) as i32 * self.chunk_size as i32;
        let top_y = chunk_y as i32 * self.chunk_size as i32;
        let bottom_y = (chunk_y + 1) as i32 * self.chunk_size as i32;

        let mut pattern_idx : usize = 0;
        for y in top_y .. bottom_y {
            for x in left_x .. right_x {
                let map_idx = map.xy_idx(x, y);
                let tile = self.constraints[new_chunk_pattern_idx as usize].pattern[pattern_idx];
                map.tiles[map_idx] = tile;
                pattern_idx += 1;
            }
        }

        // Lastly, set the tile as collapsed
        self.chunks[collapsed_chunk] = Some(new_chunk_pattern_idx as usize);
        
        false
    }

    #[inline]
    fn chunk_idx(&self, x: usize, y: usize) -> usize {
        (y * self.chunks_x + x) as usize
    }

    fn count_neighbors(&self, chunk_x: usize, chunk_y: usize) -> i32 {
        let mut neighbors = 0;

        if chunk_x > 0 {
            let left_idx = self.chunk_idx(chunk_x-1, chunk_y);
            neighbors += (self.chunks[left_idx].is_some()) as i32;
        }

        if chunk_x < (self.chunks_x - 1) {
            let right_idx = self.chunk_idx(chunk_x+1, chunk_y);
            neighbors += (self.chunks[right_idx].is_some()) as i32;
        }

        if chunk_y > 0 {
            let up_idx = self.chunk_idx(chunk_x, chunk_y - 1);
            neighbors += (self.chunks[up_idx].is_some()) as i32;
        }

        if chunk_y < (self.chunks_y - 1) {
            let down_idx = self.chunk_idx(chunk_x, chunk_y + 1);
            neighbors += (self.chunks[down_idx].is_some()) as i32;
        }

        neighbors
    }
}

