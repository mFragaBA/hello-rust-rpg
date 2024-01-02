use super::{CombatStats, GameLog, Map, Name, Player, Position, RunState, SufferDamage};
use specs::prelude::*;

pub struct DamageSystem {}

impl<'a> System<'a> for DamageSystem {
    type SystemData = (
        WriteStorage<'a, CombatStats>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, Position>,
        WriteExpect<'a, Map>,
        Entities<'a>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut stats, mut damage, positions, mut map, entities) = data;

        for (entity, mut stats, damage) in (&entities, &mut stats, &damage).join() {
            stats.hp -= damage.amount.iter().sum::<i32>();

            if let Some(pos) = positions.get(entity) {
                let idx = map.xy_idx(pos.x, pos.y);
                map.bloodstains.insert(idx);
            }
        }

        damage.clear();
    }
}

/// deletes all the dead entities and returns true if any died, false otherwise
pub fn delete_the_dead(ecs: &mut World) {
    let mut dead: Vec<Entity> = Vec::new();

    // Using a scope to make the borrow checker happy
    {
        let combat_stats = ecs.read_storage::<CombatStats>();
        let players = ecs.read_storage::<Player>();
        let entities = ecs.entities();
        let mut log = ecs.write_resource::<GameLog>();
        let names = ecs.read_storage::<Name>();
        for (entity, stats) in (&entities, &combat_stats).join() {
            if stats.hp < 1 {
                if players.get(entity).is_some() {
                    log.entries.push("You are dead!".to_string());
                    let mut runstate = ecs.write_resource::<RunState>();
                    *runstate = RunState::GameOver;
                } else {
                    let victim_name = names.get(entity);
                    if let Some(victim_name) = victim_name {
                        log.entries.push(format!("{} is dead.", &victim_name.name));
                    }
                    dead.push(entity);
                }
            }
        }
    }

    for victim in &dead {
        ecs.delete_entity(*victim)
            .expect("Unable to delete entity!");
    }
}
