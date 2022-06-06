use rltk::{VirtualKeyCode, Rltk};
use specs::prelude::*;
use super::{Map, Position, Player, TileType, State, Viewshed, RunState};
use std::cmp::{min, max};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let mut map = ecs.fetch_mut::<Map>();
    let mut ppos = ecs.write_resource::<rltk::Point>();

    for (_player, pos, viewshed) in (&mut players, &mut positions, &mut viewsheds).join() {
        let (dst_x, dst_y) = (min(79, max(0, pos.x + delta_x)), min(49, max(0, pos.y + delta_y)));
        let destination_idx = map.xy_idx(dst_x, dst_y);
        if !map.blocked[destination_idx] {
            pos.x = dst_x;
            pos.y = dst_y;
            ppos.x = pos.x;
            ppos.y = pos.y;
            map.tiles[destination_idx] = TileType::VisitedFloor;

            viewshed.dirty = true;
        }
    }
}

pub fn player_input(gs: &mut State, ctx: &mut Rltk) -> RunState {
    // Player movement
    match ctx.key {
        None => { return RunState::Paused } // Nothing Happened
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
            _ => { return RunState::Paused }
        }
    }
    RunState::Running
}

