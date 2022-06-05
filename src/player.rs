use rltk::{VirtualKeyCode, Rltk};
use specs::prelude::*;
use super::{Map, Position, Player, TileType, State};
use std::cmp::{min, max};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut map = ecs.fetch_mut::<Map>();

    for (_player, pos) in (&mut players, &mut positions).join() {
        let (dst_x, dst_y) = (min(79, max(0, pos.x + delta_x)), min(49, max(0, pos.y + delta_y)));
        let destination_idx = map.xy_idx(dst_x, dst_y);
        if map.tiles[destination_idx] != TileType::Wall {
            pos.x = dst_x;
            pos.y = dst_y;
            map.tiles[destination_idx] = TileType::VisitedFloor;
        }
    }
}

pub fn player_input(gs: &mut State, ctx: &mut Rltk) {
    // Player movement
    match ctx.key {
        None => {} // Nothing Happened
        Some(key) => match key {

            VirtualKeyCode::Left    |
            VirtualKeyCode::Numpad4 |
            VirtualKeyCode::H => try_move_player(-1, 0, &mut gs.ecs),
            
            VirtualKeyCode::Right   |
            VirtualKeyCode::Numpad6 |
            VirtualKeyCode::L => try_move_player(1, 0, &mut gs.ecs),
            
            VirtualKeyCode::Up      |
            VirtualKeyCode::Numpad8 |
            VirtualKeyCode::K => try_move_player(0, -1, &mut gs.ecs),
            
            VirtualKeyCode::Down    |
            VirtualKeyCode::Numpad2 |
            VirtualKeyCode::J => try_move_player(0, 1, &mut gs.ecs),
            
            VirtualKeyCode::Escape => ctx.quit(),
            _ => {}
        }
    }
}

