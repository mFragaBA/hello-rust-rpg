use rltk::{RandomNumberGenerator, RGB};
use specs::prelude::*;

use crate::{
    EntryTrigger, Hidden, HungerClock, MagicMapper, Map, ProvidesFood, SingleActivation, TileType,
};

use super::{
    AreaOfEffect, BlocksTile, CombatStats, Confusion, Consumable, DefenseBonus, EquipmentSlot,
    Equippable, InflictsDamage, Item, MagicStats, MeleePowerBonus, Monster, Name, Player, Position,
    ProvidesHealing, ProvidesManaRestore, RandomTable, Ranged, Rect, Renderable, SerializeMe,
    Viewshed, MAP_WIDTH,
};
use specs::saveload::{MarkedBuilder, SimpleMarker};

use std::collections::HashMap;

const MAX_MONSTERS: i32 = 4;

/// Spawns the player and returns his/her entity object.
pub fn spawn_player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
    ecs.create_entity()
        .with(Position {
            x: player_x,
            y: player_y,
        })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0,
        })
        .with(Player {})
        .with(Viewshed {
            visible_tiles: Vec::new(),
            range: 8,
            dirty: true,
        })
        .with(Name {
            name: "Sir Player of Nottingham".to_string(),
        })
        .with(CombatStats {
            max_hp: 30,
            hp: 30,
            defense: 2,
            power: 5,
        }) // TODO: revert max_hp to normal
        .with(MagicStats {
            max_mana: 10,
            mana: 10,
            power: 7,
        })
        .with(HungerClock {
            state: crate::HungerState::WellFed,
            duration: 30,
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build()
}

fn room_table(map_depth: i32) -> RandomTable {
    RandomTable::new()
        .add("Goblin", 14)
        .add("Orc", 1 + map_depth)
        .add("Health Potion", 4)
        .add("Greater Health Potion", 2)
        .add("Legendary Health Potion", 1)
        .add("Mana Potion", 3)
        .add("Greater Mana Potion", 2)
        .add("Legendary Mana Potion", 1)
        .add("Fireball Scroll", 1 + map_depth)
        .add("Confusion Scroll", 2 + map_depth)
        .add("Magic Missile Scroll", 3)
        .add("Dagger", 7)
        .add("Shield", 7)
        .add("Longsword", map_depth - 1)
        .add("Tower Shield", map_depth - 1)
        .add("Rations", 10)
        .add("Magic Mapping Scroll", 2)
        .add("Bear Trap", 7)
        .add("Spikes", 7)
}

fn orc(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, rltk::to_cp437('o'), "Orc");
}
fn goblin(ecs: &mut World, x: i32, y: i32) {
    monster(ecs, x, y, rltk::to_cp437('g'), "Goblin");
}

fn monster<S: ToString>(ecs: &mut World, x: i32, y: i32, glyph: rltk::FontCharType, name: S) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph,
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 1,
        })
        .with(Viewshed {
            visible_tiles: Vec::new(),
            range: 8,
            dirty: true,
        })
        .with(Monster {})
        .with(Name {
            name: name.to_string(),
        })
        .with(BlocksTile {})
        .with(CombatStats {
            max_hp: 16,
            hp: 16,
            defense: 1,
            power: 4,
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

pub fn spawn_room(ecs: &mut World, room: &Rect, map_depth: i32) {
    let mut possible_targets: Vec<usize> = Vec::new();
    {
        // Borrow scope
        let map = ecs.fetch::<Map>();
        for y in room.y1..room.y2 {
            for x in room.x1..room.x2 {
                let idx = map.xy_idx(x, y);
                if map.tiles[idx] == TileType::Floor {
                    possible_targets.push(idx);
                }
            }
        }
    }

    spawn_region(ecs, &possible_targets, map_depth);
}

#[allow(clippy::map_entry)]
pub fn spawn_region(ecs: &mut World, area: &[usize], map_depth: i32) {
    let spawn_table = room_table(map_depth);

    let mut spawn_points: HashMap<usize, String> = HashMap::new();
    let mut areas: Vec<usize> = Vec::from(area);

    // Score to keep the borrow checker happy
    {
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        let num_spawns = i32::min(
            areas.len() as i32,
            rng.roll_dice(1, MAX_MONSTERS + 3) + (map_depth - 1) - 3,
        );
        if num_spawns == 0 {
            return;
        }

        for _i in 0..num_spawns {
            let array_index = if areas.len() == 1 {
                0usize
            } else {
                (rng.roll_dice(1, areas.len() as i32) - 1) as usize
            };
            let map_idx = areas[array_index];
            spawn_points.insert(map_idx, spawn_table.roll(&mut rng));
            areas.remove(array_index);
        }
    }

    // Actually spawn stuff
    for spawn in spawn_points.iter() {
        spawn_entity(ecs, &spawn);
    }
}

fn spawn_entity(ecs: &mut World, spawn: &(&usize, &String)) {
    let x = (*spawn.0 % MAP_WIDTH) as i32;
    let y = (*spawn.0 / MAP_WIDTH) as i32;

    match spawn.1.as_ref() {
        "Goblin" => goblin(ecs, x, y),
        "Orc" => orc(ecs, x, y),
        "Health Potion" => potion_of_healing(ecs, x, y),
        "Grater Health Potion" => greater_potion_of_healing(ecs, x, y),
        "Legendary Health Potion" => legendary_potion_of_healing(ecs, x, y),
        "Mana Potion" => potion_of_mana(ecs, x, y),
        "Grater Mana Potion" => greater_potion_of_mana(ecs, x, y),
        "Legendary Mana Potion" => legendary_potion_of_mana(ecs, x, y),
        "Fireball Scroll" => fireball_scroll(ecs, x, y),
        "Confusion Scroll" => confusion_scroll(ecs, x, y),
        "Magic Missile Scroll" => magic_missile_scroll(ecs, x, y),
        "Dagger" => dagger(ecs, x, y),
        "Shield" => shield(ecs, x, y),
        "Longsword" => longsword(ecs, x, y),
        "Tower Shield" => tower_shield(ecs, x, y),
        "Rations" => rations(ecs, x, y),
        "Magic Mapping Scroll" => magic_mapping_scroll(ecs, x, y),
        "Bear Trap" => bear_trap(ecs, x, y),
        "Spikes" => spikes(ecs, x, y),
        _ => {}
    }
}

pub fn potion_of_healing(ecs: &mut World, x: i32, y: i32) {
    health_potion(ecs, x, y, "Potion of Healing", 8);
}
pub fn greater_potion_of_healing(ecs: &mut World, x: i32, y: i32) {
    health_potion(ecs, x, y, "Greater Potion of Healing", 12);
}
pub fn legendary_potion_of_healing(ecs: &mut World, x: i32, y: i32) {
    health_potion(ecs, x, y, "Legendary Potion of Healing", 20);
}

fn health_potion<S: ToString>(ecs: &mut World, x: i32, y: i32, name: S, heal_amount: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('¡'),
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: name.to_string(),
        })
        .with(Item {})
        .with(Consumable {})
        .with(ProvidesHealing {
            heal_amount: heal_amount,
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

pub fn potion_of_mana(ecs: &mut World, x: i32, y: i32) {
    mana_potion(ecs, x, y, "Potion of Mana", 3);
}
pub fn greater_potion_of_mana(ecs: &mut World, x: i32, y: i32) {
    mana_potion(ecs, x, y, "Greater Potion of Mana", 12);
}
pub fn legendary_potion_of_mana(ecs: &mut World, x: i32, y: i32) {
    mana_potion(ecs, x, y, "Legendary Potion of Mana", 25);
}

fn mana_potion<S: ToString>(ecs: &mut World, x: i32, y: i32, name: S, mana_amount: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('¡'),
            fg: RGB::named(rltk::MAGENTA),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: name.to_string(),
        })
        .with(Item {})
        .with(Consumable {})
        .with(ProvidesManaRestore {
            mana_amount: mana_amount,
        })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn magic_missile_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Magic Missile Scroll".to_string(),
        })
        .with(Item {})
        .with(Consumable {})
        .with(Ranged { range: 6 })
        .with(InflictsDamage { damage: 8 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn fireball_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::ORANGE),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Fireball Scroll".to_string(),
        })
        .with(Item {})
        .with(Consumable {})
        .with(Ranged { range: 6 })
        .with(InflictsDamage { damage: 8 })
        .with(AreaOfEffect { radius: 3 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn confusion_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::PINK),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Confusion Scroll".to_string(),
        })
        .with(Item {})
        .with(Consumable {})
        .with(Ranged { range: 6 })
        .with(Confusion { turns: 4 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn dagger(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('/'),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Dagger".to_string(),
        })
        .with(Item {})
        .with(Equippable {
            slot: EquipmentSlot::Melee,
        })
        .with(MeleePowerBonus { power: 2 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn shield(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('('),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Shield".to_string(),
        })
        .with(Item {})
        .with(Equippable {
            slot: EquipmentSlot::Shield,
        })
        .with(DefenseBonus { defense: 1 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn longsword(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('/'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Longsword".to_string(),
        })
        .with(Item {})
        .with(Equippable {
            slot: EquipmentSlot::Melee,
        })
        .with(MeleePowerBonus { power: 4 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn tower_shield(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('('),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Tower Shield".to_string(),
        })
        .with(Item {})
        .with(Equippable {
            slot: EquipmentSlot::Shield,
        })
        .with(DefenseBonus { defense: 3 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn rations(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('%'),
            fg: RGB::named(rltk::GREEN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Rations".to_string(),
        })
        .with(Item {})
        .with(ProvidesFood {})
        .with(Consumable {})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn magic_mapping_scroll(ecs: &mut World, x: i32, y: i32) {
    let scroll_kind = {
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        rng.roll_dice(1, 3)
    };

    let (name, power) = match scroll_kind {
        1 => ("Scroll of Magic Mapping", 7),
        2 => ("Greater Scroll of Magic Mapping", 20),
        _ => ("Legendary Scroll of Magic Mapping", 70),
    };

    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::CYAN3),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: name.to_string(),
        })
        .with(Item {})
        .with(MagicMapper { power })
        .with(Consumable {})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn bear_trap(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('w'),
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Bear Trap".to_string(),
        })
        .with(Hidden {})
        .with(EntryTrigger {})
        .with(InflictsDamage { damage: 6 })
        .with(SingleActivation {})
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn spikes(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph: rltk::to_cp437('^'),
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 2,
        })
        .with(Name {
            name: "Spikes".to_string(),
        })
        .with(Hidden {})
        .with(EntryTrigger {})
        .with(InflictsDamage { damage: 6 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}
