use rltk::{VirtualKeyCode, Rltk, console};
use specs::prelude::*;
use super::{Map, Position, Player, TileType, State, Viewshed, RunState, CombatStats, WantsToMelee};
use std::cmp::{min, max};

pub fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();
    let mut viewsheds = ecs.write_storage::<Viewshed>();
    let combat_stats = ecs.read_storage::<CombatStats>();
    let mut map = ecs.fetch_mut::<Map>();
    let mut ppos = ecs.write_resource::<rltk::Point>();

    let entities = ecs.entities();
    let mut wants_to_melee = ecs.write_storage::<WantsToMelee>();

    for (entity, _player, pos, viewshed) in (&entities, &mut players, &mut positions, &mut viewsheds).join() {
        let (dst_x, dst_y) = (min(79, max(0, pos.x + delta_x)), min(49, max(0, pos.y + delta_y)));
        let destination_idx = map.xy_idx(dst_x, dst_y);

        for potential_target in map.tile_content[destination_idx].iter() {
            if let Some(_t) = combat_stats.get(*potential_target) {
                console::log(&format!("From Hell's Hert, I stab thee!"));
                wants_to_melee.insert(entity, WantsToMelee{ target: *potential_target }).expect("Add target failed");
                return; // don't move after attacking
            }
        }

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
        None => { return RunState::AwaitingInput } // Nothing Happened
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

            // Diagonals
            VirtualKeyCode::Numpad9 |
            VirtualKeyCode::Y => try_move_player(-1, -1, &mut gs.ecs),

            VirtualKeyCode::Numpad7 |
            VirtualKeyCode::U => try_move_player(1, -1, &mut gs.ecs),

            VirtualKeyCode::Numpad3 |
            VirtualKeyCode::N => try_move_player(-1, 1, &mut gs.ecs),
            
            VirtualKeyCode::Numpad1 |
            VirtualKeyCode::M => try_move_player(1, 1, &mut gs.ecs),

            VirtualKeyCode::Escape => ctx.quit(),
            _ => { return RunState::AwaitingInput }
        }
    }
    RunState::PlayerTurn
}

