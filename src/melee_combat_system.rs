use rltk::console;
use specs::prelude::*;

use super::{CombatStats, WantsToMelee, Name, SufferDamage, GameLog, MeleePowerBonus, DefenseBonus, Equipped, ParticleBuilder, Position};

pub struct MeleeCombatSystem {}

impl<'a> System<'a> for MeleeCombatSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = ( Entities<'a>,
                        WriteExpect<'a, GameLog>,
                        WriteStorage<'a, WantsToMelee>,
                        ReadStorage<'a, Name>,
                        ReadStorage<'a, CombatStats>,
                        WriteStorage<'a, SufferDamage>,
                        ReadStorage<'a, MeleePowerBonus>,
                        ReadStorage<'a, DefenseBonus>,
                        ReadStorage<'a, Equipped>,
                        WriteExpect<'a, ParticleBuilder>,
                        ReadStorage<'a, Position>,
                    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            entities,
            mut log,
            mut wants_melee,
            names,
            combat_stats,
            mut inflict_damage,
            melee_power_bonuses,
            defense_bonuses,
            equipped,
            mut particle_builder,
            positions,
        ) = data;

        for (entity, wants_melee, name, stats) in (&entities, &wants_melee, &names, &combat_stats).join() {
            if stats.hp > 0 {
                let target_stats = combat_stats.get(wants_melee.target).unwrap();
                if target_stats.hp > 0 {
                    let offensive_bonus : i32 = 
                    (&entities, &melee_power_bonuses, &equipped)
                        .join()
                        .filter(|(_item_entity, _power_bonus, equipped_by)| equipped_by.owner == entity)
                        .map(|(_item_entity, power_bonus, _equipped_by)| power_bonus.power)
                        .sum();

                    let defensive_bonus : i32 = 
                    (&entities, &defense_bonuses, &equipped)
                        .join()
                        .filter(|(_item_entity, _defense_bonus, equipped_by)| equipped_by.owner == wants_melee.target)
                        .map(|(_item_entity, defense_bonus, _equipped_by)| defense_bonus.defense)
                        .sum();

                    let target_name = names.get(wants_melee.target).unwrap();
                    
                    if let Some(pos) = positions.get(wants_melee.target) {
                        particle_builder.request(pos.x, pos.y, rltk::RGB::named(rltk::ORANGE), rltk::RGB::named(rltk::BLACK), rltk::to_cp437('‼'), 150.0);
                    }

                    let damage = i32::max(0, (stats.power+offensive_bonus) - (target_stats.defense + defensive_bonus));
                    if damage == 0 {
                        log.entries.push(format!("{}: Hmm, must have been the wind (Took 0 Damage)", target_name.name));
                    } else {
                        log.entries.push(format!("{} hits {}, for {} hp.", &name.name, &target_name.name, damage));
                        SufferDamage::new_damage(&mut inflict_damage, wants_melee.target, damage);
                    }
                }
            }
        }

        wants_melee.clear();
    }
}
