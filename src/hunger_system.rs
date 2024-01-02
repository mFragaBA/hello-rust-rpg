use super::{gamelog::GameLog, HungerClock, HungerState, RunState, SufferDamage};
use specs::prelude::*;

pub struct HungerSystem {}

impl<'a> System<'a> for HungerSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, HungerClock>,
        ReadExpect<'a, Entity>,
        ReadExpect<'a, RunState>,
        WriteStorage<'a, SufferDamage>,
        WriteExpect<'a, GameLog>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut hunger_clock, player, runstate, mut inflict_damage, mut log) = data;

        for (entity, mut clock) in (&entities, &mut hunger_clock).join() {
            let is_player = entity == *player;
            match (*runstate, is_player) {
                (RunState::PlayerTurn, true) | (RunState::MonsterTurn, false) => {
                    clock.duration -= 1;
                    if clock.duration < 1 {
                        match clock.state {
                            HungerState::WellFed => {
                                clock.state = HungerState::Normal;
                                clock.duration = 200;
                                if is_player {
                                    log.entries.push("You are no longer well fed.".to_string());
                                }
                            }
                            HungerState::Normal => {
                                clock.state = HungerState::Hungry;
                                clock.duration = 200;
                                if is_player {
                                    log.entries.push("You are hungry.".to_string());
                                }
                            }
                            HungerState::Hungry => {
                                clock.state = HungerState::Starving;
                                clock.duration = 200;
                                if is_player {
                                    log.entries.push(
                                        "You are starving! Eat something, quick!".to_string(),
                                    );
                                }
                            }
                            HungerState::Starving => {
                                if is_player {
                                    log.entries.push("Your hunger pangs are getting painful! You suffer 1 hp damage.".to_string());
                                }
                                SufferDamage::new_damage(&mut inflict_damage, entity, 1);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
