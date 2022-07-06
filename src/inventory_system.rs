use specs::prelude::*;
use crate::{ParticleBuilder, HungerState, HungerClock, ProvidesFood};

use super::{WantsToPickupItem, WantsToDropItem, Name, InBackpack, Position, gamelog::GameLog, CombatStats, MagicStats, ProvidesHealing, ProvidesManaRestore, WantsToUseItem, Consumable, InflictsDamage, SufferDamage, Map, AreaOfEffect, Confusion, Equippable, Equipped, WantsToRemoveItem};
pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
                ReadExpect<'a, Entity>,
                WriteExpect<'a, GameLog>,
                WriteStorage<'a, WantsToPickupItem>,
                WriteStorage<'a, Position>,
                ReadStorage<'a, Name>,
                WriteStorage<'a, InBackpack>,
            );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) = data;

        for pickup in wants_pickup.join() {
            positions.remove(pickup.item);
            backpack.insert(pickup.item, InBackpack{ owner: pickup.collected_by }).expect("Unable to insert backpack entry");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!("You pick up a {}.", names.get(pickup.item).unwrap().name));
            }
        }

        wants_pickup.clear();
    }
}

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadExpect<'a, Entity>,
                        ReadExpect<'a, Map>,
                        WriteExpect<'a, GameLog>,
                        Entities<'a>,
                        WriteStorage<'a, WantsToUseItem>,
                        ReadStorage<'a, Name>,
                        ReadStorage<'a, Consumable>,
                        ReadStorage<'a, ProvidesHealing>,
                        ReadStorage<'a, ProvidesManaRestore>,
                        WriteStorage<'a, CombatStats>,
                        WriteStorage<'a, MagicStats>,
                        ReadStorage<'a, InflictsDamage>,
                        WriteStorage<'a, SufferDamage>,
                        ReadStorage<'a, AreaOfEffect>,
                        WriteStorage<'a, Confusion>,
                        ReadStorage<'a, Equippable>,    
                        WriteStorage<'a, Equipped>,    
                        WriteStorage<'a, InBackpack>,    
                        WriteExpect<'a, ParticleBuilder>,
                        ReadStorage<'a, Position>,
                        WriteStorage<'a, HungerClock>,
                        ReadStorage<'a, ProvidesFood>,
                    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            player_entity, 
            map, 
            mut gamelog, 
            entities, 
            mut useitem, 
            names, 
            consumables, 
            healing, 
            mana_restoring, 
            mut combat_stats, 
            mut magic_stats, 
            inflict_damage, 
            mut suffer_damage, 
            aoe, 
            mut confused, 
            equippable, 
            mut equipped, 
            mut backpack,
            mut particle_builder,
            positions,
            mut hunger_clocks,
            provides_food
        ) = data;

        // Targeting
        for (entity, useitem) in (&entities, &useitem).join() {
            let mut targets : Vec<Entity> = Vec::new();
            match useitem.target {
                None => { targets.push( *player_entity ); }
                Some(target) => {
                    let area_effect = aoe.get(useitem.item);
                    match area_effect {
                        None => {
                            // Single target in tile
                            let idx = map.xy_idx(target.x, target.y);
                            for mob in map.tile_content[idx].iter() {
                                targets.push(*mob);
                            }
                        }
                        Some(area_effect) => {
                            // AoE
                            let mut blast_tiles = rltk::field_of_view(target, area_effect.radius, &*map);
                            blast_tiles.retain(|p| p.x > 0 && p.x < map.width-1 && p.y > 0 && p.y < map.height-1 );
                            for tile_idx in blast_tiles.iter() {
                                let idx = map.xy_idx(tile_idx.x, tile_idx.y);
                                for mob in map.tile_content[idx].iter() {
                                    targets.push(*mob);
                                }

                                particle_builder.request(tile_idx.x, tile_idx.y, rltk::RGB::named(rltk::ORANGE), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('░'), 200.0);
                            }
                        }
                    }
                }
            }

            // If it is equippable then we want to equip it. And unequip whatever else was equipped
            let can_equip = equippable.get(useitem.item);
            if can_equip.is_some() { 
                let target_slot = can_equip.unwrap().slot;
                let target = targets[0];

                // Remove any items the target has in the item's slot
                let mut to_unequip: Vec<Entity> = Vec::new();
                for (item_entity, already_equipped, name) in (&entities, &equipped, &names).join() {
                    if already_equipped.owner == target && already_equipped.slot == target_slot {
                        to_unequip.push(item_entity);
                        if target == *player_entity {
                            gamelog.entries.push(format!("You unequip {}.", name.name));
                        }
                    }
                }

                for item in to_unequip.iter() {
                    equipped.remove(*item);
                    backpack.insert(*item, InBackpack{ owner: target }).expect("Unable to insert backpack entry");
                }

                // Wield the item
                equipped.insert(useitem.item, Equipped{ owner: target, slot: target_slot }).expect("Unable to insert equipped component");
                backpack.remove(useitem.item);
                if target == *player_entity {
                    gamelog.entries.push(format!("You equip {}.", names.get(useitem.item).unwrap().name));
                }
            }
        
            // Use consumables
            // Maybe Healing?
            if let Some(healer) = healing.get(useitem.item) {
                for target in targets.iter() {
                    let stats = combat_stats.get_mut(*target);
                    if let Some(stats) = stats {
                        stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                        if entity == *player_entity {
                            gamelog.entries.push(format!("You drink the {}, healing {} hp", names.get(useitem.item).unwrap().name, healer.heal_amount));
                        }

                        if let Some(pos) = positions.get(*target) {
                            particle_builder.request(pos.x, pos.y, rltk::RGB::named(rltk::GREEN), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('♥'), 200.0);
                        }
                    }
                }
            }

            // Maybe Mana Restoring?
            if let Some(mana_restorer) = mana_restoring.get(useitem.item) {
                for target in targets.iter() {
                    let stats = magic_stats.get_mut(*target);
                    if let Some(stats) = stats {
                        stats.mana = i32::min(stats.max_mana, stats.mana + mana_restorer.mana_amount);
                        if entity == *player_entity {
                            gamelog.entries.push(format!("You drink the {}, restoring {} mana", names.get(useitem.item).unwrap().name, mana_restorer.mana_amount));
                        }
                        
                        if let Some(pos) = positions.get(*target) {
                            particle_builder.request(pos.x, pos.y, rltk::RGB::named(rltk::BLUE), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('♥'), 200.0);
                        }
                    }
                }
            }

            // if it's edible, eat it
            if provides_food.get(useitem.item).is_some() {
                let target = targets[0];
                if let Some(hunger_clock) = hunger_clocks.get_mut(target) {
                    gamelog.entries.push(format!("You eat the {}.", names.get(useitem.item).unwrap().name));

                    // This is sort of filling a hunger bar
                    hunger_clock.duration += 150;
                    if hunger_clock.duration > 200 {
                        match hunger_clock.state {
                            HungerState::WellFed => {
                                hunger_clock.duration = 30;
                            }
                            HungerState::Normal => {
                                hunger_clock.duration = i32::min(hunger_clock.duration - 200, 30);
                                hunger_clock.state = HungerState::WellFed;
                            }
                            HungerState::Hungry => {
                                hunger_clock.duration = hunger_clock.duration - 200;
                                hunger_clock.state = HungerState::Normal;
                            }
                            HungerState::Starving => {
                                hunger_clock.duration = hunger_clock.duration - 200;
                                hunger_clock.state = HungerState::Hungry;
                            }
                        }
                    }
                }
            }
           
            // Deals Damage?
            if let Some(damage) = inflict_damage.get(useitem.item) {
                for mob in targets.iter() {
                    SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                    if entity == *player_entity {
                        let mob_name = names.get(*mob).unwrap();
                        let item_name = names.get(useitem.item).unwrap();
                        gamelog.entries.push(format!("You use {} on {}, inflicting {} hp.", item_name.name, mob_name.name, damage.damage));
                    }
                            
                    if let Some(pos) = positions.get(*mob) {
                        particle_builder.request(pos.x, pos.y, rltk::RGB::named(rltk::RED), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('‼'), 200.0);
                    }
                }
            }

            let mut add_confusion = Vec::new(); 
            {
                // Applies Confusion?
                if let Some(confusion) = confused.get(useitem.item) {
                    for mob in targets.iter() {
                        add_confusion.push( (*mob, confusion.turns) );
                        if entity == *player_entity {
                            let mob_name = names.get(*mob).unwrap();
                            let item_name = names.get(useitem.item).unwrap();
                            gamelog.entries.push(format!("You use {} on {}, confusing them.", item_name.name, mob_name.name));
                        
                            if let Some(pos) = positions.get(*mob) {
                                particle_builder.request(pos.x, pos.y, rltk::RGB::named(rltk::MAGENTA), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('?'), 200.0);
                            }
                        }
                    }
                }
            }

            for mob in add_confusion.iter() {
                confused.insert(mob.0, Confusion{ turns: mob.1 }).expect("Unable to insert status");
            }

            // Consume consumables
            let consumable = consumables.get(useitem.item);
            if consumable.is_some() {
                entities.delete(useitem.item).expect("Delete Failed");
            }
        }

        useitem.clear();
    }
}

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( ReadExpect<'a, Entity>,
                        WriteExpect<'a, GameLog>,
                        Entities<'a>,
                        WriteStorage<'a, WantsToDropItem>,
                        ReadStorage<'a, Name>,
                        WriteStorage<'a, Position>,
                        WriteStorage<'a, InBackpack>);

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut wants_drop, names, mut positions, mut backpack) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos : Position = Position{x: 0, y: 0};
            {
                let dropped_pos = positions.get(entity).unwrap();
                dropper_pos.x = dropped_pos.x;
                dropper_pos.y = dropped_pos.y;
            }
            positions.insert(to_drop.item, Position{ x: dropper_pos.x, y: dropper_pos.y }).expect("Unable to insert position");
            backpack.remove(to_drop.item);

            if entity == *player_entity {
                gamelog.entries.push(format!("You drop the {}.", names.get(to_drop.item).unwrap().name));
            }
        }

        wants_drop.clear();
    }
}

pub struct ItemRemoveSystem {}

impl<'a> System<'a> for ItemRemoveSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
                        Entities<'a>,
                        WriteStorage<'a, WantsToRemoveItem>,
                        WriteStorage<'a, Equipped>,
                        WriteStorage<'a, InBackpack>,
                    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities, 
            mut wants_remove, 
            mut equipped, 
            mut backpack
        ) = data;

        for (entity, to_remove) in (&entities, &wants_remove).join() {
            equipped.remove(to_remove.item);
            backpack.insert(to_remove.item, InBackpack{ owner: entity }).expect("Unable to insert in backpack");
        }

        wants_remove.clear();
    }
}
