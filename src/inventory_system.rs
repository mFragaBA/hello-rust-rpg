use specs::prelude::*;
use super::{WantsToPickupItem, WantsToDropItem, Name, InBackpack, Position, gamelog::GameLog, CombatStats, MagicStats, ProvidesHealing, ProvidesManaRestore, WantsToUseItem, Consumable};

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
                        WriteExpect<'a, GameLog>,
                        Entities<'a>,
                        WriteStorage<'a, WantsToUseItem>,
                        ReadStorage<'a, Name>,
                        ReadStorage<'a, Consumable>,
                        ReadStorage<'a, ProvidesHealing>,
                        ReadStorage<'a, ProvidesManaRestore>,
                        WriteStorage<'a, CombatStats>,
                        WriteStorage<'a, MagicStats>);

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, entities, mut useitem, names, consumables, healing, mana_restoring, mut combat_stats, mut magic_stats) = data;

        // Use consumables
        for (entity, useitem, stats) in (&entities, &useitem, &mut combat_stats).join() {
            // Maybe Healing?
            let item_heals = healing.get(useitem.item);
            if let Some(healer) = item_heals {
                stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                if entity == *player_entity {
                    gamelog.entries.push(format!("You drink the {}, healing {} hp", names.get(useitem.item).unwrap().name, healer.heal_amount));
                }
            }

        }

        for (entity, useitem, stats) in (&entities, &useitem, &mut magic_stats).join() {
            // Maybe Mana Restoring?
            let mana_restores = mana_restoring.get(useitem.item);
            if let Some(mana_restorer) = mana_restores {
                stats.mana = i32::min(stats.max_mana, stats.mana + mana_restorer.mana_amount);
                if entity == *player_entity {
                    gamelog.entries.push(format!("You drink the {}, restoring {} mana", names.get(useitem.item).unwrap().name, mana_restorer.mana_amount));
                }
            }
        }

        // Consume consumables
        for useitem in (&useitem).join() {
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
