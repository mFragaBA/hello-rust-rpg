use crate::{InflictsDamage, ParticleBuilder, SingleActivation, SufferDamage};

use super::{gamelog::GameLog, EntityMoved, EntryTrigger, Hidden, Map, Name, Position};
use specs::prelude::*;

pub struct TriggerSystem {}

impl<'a> System<'a> for TriggerSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadExpect<'a, Map>,
        WriteStorage<'a, EntityMoved>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, EntryTrigger>,
        WriteStorage<'a, Hidden>,
        ReadStorage<'a, Name>,
        Entities<'a>,
        WriteExpect<'a, GameLog>,
        WriteExpect<'a, ParticleBuilder>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, InflictsDamage>,
        ReadStorage<'a, SingleActivation>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            mut entity_moved,
            position,
            entry_trigger,
            mut hidden,
            names,
            entities,
            mut log,
            mut particle_builder,
            mut inflict_damage,
            inflicts_damage,
            single_activation,
        ) = data;

        // Will store the triggered entities that should trigger once
        let mut entities_to_remove: Vec<Entity> = Vec::new();

        // Iterate entities that have moved
        for (entity, mut _entity_moved, pos) in (&entities, &mut entity_moved, &position).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            for entity_id in map.tile_content[idx].iter() {
                if entity != *entity_id {
                    // don't check against the entity itself
                    // if the other entity has an EntryTrigger component, then we trigger it
                    let maybe_trigger = entry_trigger.get(*entity_id);
                    if maybe_trigger.is_some() {
                        // We triggered it
                        let name = names.get(*entity_id);
                        if let Some(name) = name {
                            log.entries.push(format!("{} triggers!", &name.name));
                        }

                        let damage = inflicts_damage.get(*entity_id);
                        if let Some(damage) = damage {
                            particle_builder.request(
                                pos.x,
                                pos.y,
                                rltk::RGB::named(rltk::ORANGE),
                                rltk::RGB::named(rltk::BLACK),
                                rltk::to_cp437('‼'),
                                200.0,
                            );
                            SufferDamage::new_damage(&mut inflict_damage, entity, damage.damage);
                            log.entries
                                .push(format!("you suffer {} damage!", &damage.damage));
                        }

                        let sa = single_activation.get(*entity_id);
                        if sa.is_some() {
                            entities_to_remove.push(*entity_id);
                        }

                        hidden.remove(*entity_id); // Not hidden anymore
                    }
                }
            }
        }

        // Remove any single activation traps
        for trigger in entities_to_remove.iter() {
            entities
                .delete(*trigger)
                .expect("Unable to delete single activation trigger");
        }

        // remove all entity movement markers
        entity_moved.clear();
    }
}
