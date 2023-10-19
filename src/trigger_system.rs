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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (map, mut entity_moved, position, entry_trigger, mut hidden, names, entities, mut log) =
            data;

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

                        hidden.remove(*entity_id); // Not hidden anymore
                    }
                }
            }
        }

        // remove all entity movement markers
        entity_moved.clear();
    }
}
