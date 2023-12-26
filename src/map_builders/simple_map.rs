use crate::{Position, spawner};

use super::MapBuilder;
use super::{Map, Rect, TileType};
use super::common;
use rltk::RandomNumberGenerator;
use specs::World;

pub struct SimpleMapBuilder {
    map: Map,
    starting_position: Position,
    depth: i32
}

impl MapBuilder for SimpleMapBuilder {
    fn build_map(&mut self) {
        self.rooms_and_corridors();
    }

    fn spawn_entities(&mut self, ecs: &mut World) {
        for room in self.map.rooms.iter().skip(1) {
            spawner::spawn_room(ecs, room, self.depth);
        }
    }

    fn get_map(&mut self) -> Map {
        self.map.clone()
    }

    fn get_starting_position(&mut self) -> Position {
        self.starting_position.clone()
    }
}

impl SimpleMapBuilder {
    pub fn new(new_depth: i32) -> SimpleMapBuilder {
        SimpleMapBuilder {
            map: Map::new(new_depth),
            starting_position: Position { x: 0, y: 0 },
            depth: new_depth
        }
    }

    fn rooms_and_corridors(&mut self) {
        const MAX_ROOMS: i32 = 32;
        const MIN_SIZE: i32 = 6;
        const MAX_SIZE: i32 = 10;

        let mut rng = RandomNumberGenerator::new();

        for _ in 0..MAX_ROOMS {
            let w = rng.range(MIN_SIZE, MAX_SIZE);
            let h = rng.range(MIN_SIZE, MAX_SIZE);
            let x = rng.roll_dice(1, self.map.width - w - 1) - 1;
            let y = rng.roll_dice(1, self.map.height - h - 1) - 1;
            let new_room = Rect::new(x, y, w, h);

            if !self.map.rooms.iter().any(|room| new_room.intersect(room)) {
                common::apply_room_to_map(&mut self.map, &new_room);

                // join the room with another one
                if !self.map.rooms.is_empty() {
                    let (new_x, new_y) = new_room.center();
                    let (prev_x, prev_y) = self.map.rooms[self.map.rooms.len() - 1].center();
                    if rng.range(0, 2) == 1 {
                        
                        common::apply_horizontal_tunnel(&mut self.map, prev_x, new_x, prev_y);
                        common::apply_vertical_tunnel(&mut self.map, prev_y, new_y, new_x);
                    } else {
                        common::apply_vertical_tunnel(&mut self.map, prev_y, new_y, prev_x);
                        common::apply_horizontal_tunnel(&mut self.map, prev_x, new_x, new_y);
                    }
                }

                self.map.rooms.push(new_room)
            }
        }

        let stairs_position = self.map.rooms[self.map.rooms.len() - 1].center();
        let stairs_idx = self.map.xy_idx(stairs_position.0, stairs_position.1);
        self.map.tiles[stairs_idx] = TileType::DownStairs;

        let (player_x, player_y) = self.map.rooms[0].center();
        self.starting_position = Position { x: player_x, y: player_y };
    }
}

