use rltk::{VirtualKeyCode, Rltk, console, Point};
use specs::prelude::*;
use super::{Map, Position, Player, TileType, State, Viewshed, RunState, CombatStats, WantsToMelee, WantsToPickupItem, GameLog, Item, Monster};
use std::cmp::{min, max};
use crate::gui;

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

        if !map.blocked[destination_idx]  {
            pos.x = dst_x;
            pos.y = dst_y;
            ppos.x = pos.x;
            ppos.y = pos.y;

            if map.tiles[destination_idx] != TileType::DownStairs {
                map.tiles[destination_idx] = TileType::VisitedFloor;
            }

            viewshed.dirty = true;
        }
    }
}

fn get_item(ecs: &mut World) {
    let player_pos = ecs.fetch::<Point>();
    let player_entity = ecs.fetch::<Entity>();
    let entities = ecs.entities();
    let items = ecs.read_storage::<Item>();
    let positions = ecs.read_storage::<Position>();
    let mut gamelog = ecs.fetch_mut::<GameLog>();

    let mut target_item: Option<Entity> = None;
    for (item_entity, _item, position) in (&entities, &items, &positions).join() {
        if position.x == player_pos.x && position.y == player_pos.y {
            target_item = Some(item_entity);
        }
    }

    match target_item {
        None => gamelog.entries.push("There is nothing here to pick up.".to_string()),
        Some(item) => {
            let mut pickup = ecs.write_storage::<WantsToPickupItem>();
            pickup.insert(*player_entity, WantsToPickupItem{ collected_by: *player_entity, item}).expect("Unable to pick up item");
        }
    }
}

pub fn try_next_level(ecs: &mut World) -> bool {
    let player_pos = ecs.fetch::<Point>();
    let map = ecs.fetch::<Map>();
    let player_idx = map.xy_idx(player_pos.x, player_pos.y);
    
    if map.tiles[player_idx] != TileType::DownStairs {
        let mut gamelog = ecs.fetch_mut::<GameLog>();
        gamelog.entries.push("There is no way down from here.".to_string());
        return false;
    }

    true
}

fn skip_turn(ecs: &mut World) -> RunState {
    let player_ent = ecs.fetch::<Entity>();
    let viewshed_components = ecs.read_storage::<Viewshed>();
    let monsters = ecs.read_storage::<Monster>();

    let worldmap_resource = ecs.fetch::<Map>();

    let can_heal = viewshed_components.get(*player_ent).unwrap()
        .visible_tiles
        .iter()
        .all(|tile|{
            let idx = worldmap_resource.xy_idx(tile.x, tile.y);
            worldmap_resource.tile_content[idx]
                .iter()
                .all(|ent_id| monsters.get(*ent_id).is_none())
        });

    if can_heal {
        let mut health_components = ecs.write_storage::<CombatStats>();
        let player_hp = health_components.get_mut(*player_ent).unwrap();
        player_hp.hp = i32::min(player_hp.hp + 1, player_hp.max_hp);
    }

    RunState::PlayerTurn
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

            // Grabbing
            VirtualKeyCode::G => get_item(&mut gs.ecs),

            // Open Inventory
            VirtualKeyCode::I => return RunState::ShowInventory,

            // Open Delete Menu
            VirtualKeyCode::D => return RunState::ShowDropItem,

            // Level changes
            VirtualKeyCode::Period => {
                if try_next_level(&mut gs.ecs) {
                    return RunState::NextLevel;
                }
            }

            // Skip Turn
            VirtualKeyCode::Numpad5 |
            VirtualKeyCode::Space => return skip_turn(&mut gs.ecs),

            // Drop Item
            VirtualKeyCode::R => return RunState::ShowRemoveItem,

            VirtualKeyCode::Escape => return RunState::MainMenu{ menu_selection: gui::MainMenuSelection::NewGame },
            _ => { return RunState::AwaitingInput }
        }
    }
    RunState::PlayerTurn
}

