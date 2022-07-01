use rltk::{ RGB, RandomNumberGenerator };
use specs::prelude::*;

use super::{CombatStats, MagicStats, Player, Renderable, Name, Position, Viewshed, Monster, BlocksTile, Rect, MAP_WIDTH, MAP_HEIGHT, ProvidesHealing, ProvidesManaRestore, Item, Consumable, Ranged, InflictsDamage, AreaOfEffect, Confusion, SerializeMe, RandomTable, Equippable, EquipmentSlot, MeleePowerBonus, DefenseBonus};
use specs::saveload::{MarkedBuilder, SimpleMarker};

use std::collections::HashMap;

const MAX_MONSTERS : i32 = 4;

/// Spawns the player and returns his/her entity object.
pub fn spawn_player(ecs: &mut World, player_x: i32, player_y: i32) -> Entity {
    ecs
        .create_entity()
        .with(Position { x : player_x, y: player_y })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 0,
        })
        .with(Player {})
        .with(Viewshed{ visible_tiles: Vec::new(), range : 8, dirty: true })
        .with(Name{name: "Sir Player of Nottingham".to_string() })
        .with(CombatStats{ max_hp: 30, hp: 30, defense: 2, power: 5 }) // TODO: revert max_hp to normal
        .with(MagicStats{ max_mana: 10, mana: 10, power: 7})
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
}

fn orc(ecs: &mut World, x: i32, y: i32) { monster(ecs, x, y, rltk::to_cp437('o'), "Orc"); }
fn goblin(ecs: &mut World, x: i32, y: i32) { monster(ecs, x, y, rltk::to_cp437('g'), "Goblin"); }

fn monster<S: ToString>(ecs: &mut World, x: i32, y: i32, glyph: rltk::FontCharType, name: S) {
    ecs
        .create_entity()
        .with(Position { x, y })
        .with(Renderable {
            glyph,
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 1,
        })
        .with(Viewshed{ visible_tiles: Vec::new(), range : 8, dirty: true })
        .with(Monster{})
        .with(Name{name: name.to_string() })
        .with(BlocksTile{})
        .with(CombatStats{ max_hp: 16, hp: 16, defense: 1, power: 4 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

#[allow(clippy::map_entry)]
pub fn spawn_room(ecs: &mut World, room: &Rect, map_depth: i32) {
    let spawn_table = room_table(map_depth);

    let mut spawn_points : HashMap<usize, String> = HashMap::new();

    // Score to keep the borrow checker happy
    {
        let mut rng = ecs.write_resource::<RandomNumberGenerator>();
        let num_spawns = rng.roll_dice(1, MAX_MONSTERS + 3) + (map_depth - 1) - 3;

        for _i in 0..num_spawns {
            let mut added = false;
            let mut tries = 0;
            while !added && tries < 20 {
                let x = (room.x1 + rng.roll_dice(1, i32::abs(room.x2 - room.x1))) as usize;
                let y = (room.y1 + rng.roll_dice(1, i32::abs(room.y2 - room.y1))) as usize;
                let idx = (y * MAP_WIDTH) + x;
                if !spawn_points.contains_key(&idx) {
                    spawn_points.insert(idx, spawn_table.roll(&mut rng));
                    added = true;
                } else {
                    tries += 1;
                }
            }
        }
    }


    // Actually spawn stuff
    for spawn in spawn_points.iter() {
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
            _ => {}
        }
    }
}

pub fn potion_of_healing(ecs: &mut World, x: i32, y: i32) { health_potion(ecs, x, y, "Potion of Healing", 8); }
pub fn greater_potion_of_healing(ecs: &mut World, x: i32, y: i32) { health_potion(ecs, x, y, "Greater Potion of Healing", 12); }
pub fn legendary_potion_of_healing(ecs: &mut World, x: i32, y: i32) { health_potion(ecs, x, y, "Legendary Potion of Healing", 20); }

fn health_potion<S: ToString>(ecs: &mut World, x: i32, y: i32, name: S, heal_amount: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('¡'),
            fg: RGB::named(rltk::RED),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: name.to_string() })
        .with(Item{})
        .with(Consumable{})
        .with(ProvidesHealing{ heal_amount: heal_amount })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

pub fn potion_of_mana(ecs: &mut World, x: i32, y: i32) { mana_potion(ecs, x, y, "Potion of Mana", 3); }
pub fn greater_potion_of_mana(ecs: &mut World, x: i32, y: i32) { mana_potion(ecs, x, y, "Greater Potion of Mana", 12); }
pub fn legendary_potion_of_mana(ecs: &mut World, x: i32, y: i32) { mana_potion(ecs, x, y, "Legendary Potion of Mana", 25); }

fn mana_potion<S: ToString>(ecs: &mut World, x: i32, y: i32, name: S, mana_amount: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('¡'),
            fg: RGB::named(rltk::MAGENTA),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: name.to_string() })
        .with(Item{})
        .with(Consumable{})
        .with(ProvidesManaRestore{ mana_amount: mana_amount })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn magic_missile_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
    .with(Name{ name: "Magic Missile Scroll".to_string() })
        .with(Item{})
        .with(Consumable{})
        .with(Ranged{ range: 6 })
        .with(InflictsDamage{ damage: 8 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn fireball_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::ORANGE),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
    .with(Name{ name: "Fireball Scroll".to_string() })
        .with(Item{})
        .with(Consumable{})
        .with(Ranged{ range: 6 })
        .with(InflictsDamage{ damage: 8 })
        .with(AreaOfEffect{ radius: 3 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn confusion_scroll(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437(')'),
            fg: RGB::named(rltk::PINK),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
    .with(Name{ name: "Confusion Scroll".to_string() })
        .with(Item{})
        .with(Consumable{})
        .with(Ranged{ range: 6 })
        .with(Confusion{ turns: 4 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn dagger(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('/'),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: "Dagger".to_string() })
        .with(Item{})
        .with(Equippable { slot: EquipmentSlot::Melee })
        .with(MeleePowerBonus { power: 2 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn shield(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('('),
            fg: RGB::named(rltk::CYAN),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: "Shield".to_string() })
        .with(Item{})
        .with(Equippable { slot: EquipmentSlot::Shield })
        .with(DefenseBonus { defense: 1 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn longsword(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('/'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: "Longsword".to_string() })
        .with(Item{})
        .with(Equippable { slot: EquipmentSlot::Melee })
        .with(MeleePowerBonus { power: 4 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}

fn tower_shield(ecs: &mut World, x: i32, y: i32) {
    ecs.create_entity()
        .with(Position{ x, y })
        .with(Renderable{
            glyph: rltk::to_cp437('('),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
            render_order: 2
        })
        .with(Name{ name: "Tower Shield".to_string() })
        .with(Item{})
        .with(Equippable { slot: EquipmentSlot::Shield })
        .with(DefenseBonus { defense: 3 })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();
}
