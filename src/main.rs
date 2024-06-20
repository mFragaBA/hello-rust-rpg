extern crate serde;
use particle_system::ParticleSpawnSystem;
use rltk::{GameState, Point, Rltk, RGB};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

// Module Imports
mod components;
pub use components::*;
mod map;
pub use map::*;
pub mod map_builders;
mod player;
use player::*;
mod rect;
pub use rect::*;
mod gamelog;
mod gui;
mod rex_assets;
use gamelog::GameLog;
mod random_table;
mod spawner;
use random_table::*;

mod visibility_system;
use visibility_system::VisibilitySystem;
mod monster_ai_system;
use monster_ai_system::MonsterAI;
mod map_indexing_system;
use map_indexing_system::MapIndexingSystem;
mod melee_combat_system;
use melee_combat_system::MeleeCombatSystem;
mod damage_system;
use damage_system::DamageSystem;
mod inventory_system;
use inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemRemoveSystem, ItemUseSystem};
mod particle_system;
pub mod saveload_system;
pub use particle_system::ParticleBuilder;
mod hunger_system;
use hunger_system::HungerSystem;
mod trigger_system;
use trigger_system::TriggerSystem;

const SHOW_MAPGEN_VISUALIZER: bool = true;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting {
        range: i32,
        item: Entity,
    },
    MagicMapReveal {
        remaining_power: i32,
        offset: i32,
    },
    MainMenu {
        menu_selection: gui::MainMenuSelection,
    },
    LoadMenu {
        menu_selection: gui::LoadMenuSelection,
    },
    NextLevel,
    ShowRemoveItem,
    MapGeneration,
    GameOver,
}

pub struct State {
    pub ecs: World,
    mapgen_next_state: Option<RunState>,
    mapgen_history: Vec<Map>,
    mapgen_index: usize,
    mapgen_timer: f32,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);
        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);
        let mut map_index = MapIndexingSystem {};
        map_index.run_now(&self.ecs);
        let mut melee_system = MeleeCombatSystem {};
        melee_system.run_now(&self.ecs);
        let mut dmg_system = DamageSystem {};
        dmg_system.run_now(&self.ecs);
        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);
        let mut potions = ItemUseSystem {};
        potions.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);
        let mut unequip_items = ItemRemoveSystem {};
        unequip_items.run_now(&self.ecs);
        let mut particle_system = ParticleSpawnSystem {};
        particle_system.run_now(&self.ecs);
        let mut hunger_system = HungerSystem {};
        hunger_system.run_now(&self.ecs);
        let mut trigger_system = TriggerSystem {};
        trigger_system.run_now(&self.ecs);

        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        ctx.cls();
        particle_system::tick_and_cull_dead_particles(&mut self.ecs, ctx);

        match newrunstate {
            RunState::MainMenu { .. } => {}
            _ => {
                draw_map(&self.ecs.fetch::<Map>(), ctx);

                {
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let hidden = self.ecs.read_storage::<Hidden>();
                    let map = self.ecs.fetch::<Map>();

                    let mut data = (&positions, &renderables, !&hidden)
                        .join()
                        .collect::<Vec<_>>();
                    data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order));

                    for (pos, render, _hidden) in data.iter() {
                        let idx = map.xy_idx(pos.x, pos.y);
                        if map.visible_tiles[idx] {
                            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
                        }
                    }

                    gui::draw_ui(&self.ecs, ctx);
                }
            }
        }

        match newrunstate {
            RunState::MapGeneration => {
                if !SHOW_MAPGEN_VISUALIZER {
                    newrunstate = self.mapgen_next_state.unwrap();
                }
                ctx.cls();
                draw_map(&self.mapgen_history[self.mapgen_index], ctx);

                self.mapgen_timer += ctx.frame_time_ms;
                // change index every 150ms
                if self.mapgen_timer > 100.0 {
                    self.mapgen_timer = 0.0;
                    self.mapgen_index += 1;
                    if self.mapgen_index >= self.mapgen_history.len() {
                        newrunstate = self.mapgen_next_state.unwrap();
                    }
                }
            }
            RunState::PreRun => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                self.ecs.maintain();
                match *self.ecs.fetch::<RunState>() {
                    RunState::MagicMapReveal {
                        remaining_power,
                        offset,
                    } => {
                        newrunstate = RunState::MagicMapReveal {
                            remaining_power,
                            offset,
                        }
                    }
                    _ => newrunstate = RunState::MonsterTurn,
                };
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => match gui::show_inventory(self, ctx) {
                (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                (gui::ItemMenuResult::NoResponse, _) => {}
                (gui::ItemMenuResult::Selected, entity) => {
                    let entity = entity.unwrap();
                    if let Some(ranged_item) = self.ecs.read_storage::<Ranged>().get(entity) {
                        newrunstate = RunState::ShowTargeting {
                            range: ranged_item.range,
                            item: entity,
                        };
                    } else {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToUseItem {
                                    item: entity,
                                    target: None,
                                },
                            )
                            .expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            },
            RunState::ShowDropItem => match gui::drop_item_menu(self, ctx) {
                (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                (gui::ItemMenuResult::NoResponse, _) => {}
                (gui::ItemMenuResult::Selected, entity) => {
                    let entity = entity.unwrap();
                    let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                    intent
                        .insert(
                            *self.ecs.fetch::<Entity>(),
                            WantsToDropItem { item: entity },
                        )
                        .expect("Unable to insert intent");
                    newrunstate = RunState::PlayerTurn;
                }
            },
            RunState::ShowTargeting { range, item } => match gui::ranged_target(self, ctx, range) {
                (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                (gui::ItemMenuResult::NoResponse, _) => {}
                (gui::ItemMenuResult::Selected, entity) => {
                    let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                    intent
                        .insert(
                            *self.ecs.fetch::<Entity>(),
                            WantsToUseItem {
                                item,
                                target: entity,
                            },
                        )
                        .expect("Unable to insert intent");
                    newrunstate = RunState::PlayerTurn;
                }
            },
            RunState::MainMenu { .. } => match gui::main_menu(self, ctx) {
                gui::MainMenuResult::NoSelection { selected } => {
                    newrunstate = RunState::MainMenu {
                        menu_selection: selected,
                    };
                }
                gui::MainMenuResult::Selected {
                    selected: gui::MainMenuSelection::SaveGame,
                } => {
                    saveload_system::save_game(&mut self.ecs, "some_saved_game");
                    newrunstate = RunState::MainMenu {
                        menu_selection: gui::MainMenuSelection::LoadGame,
                    };
                }
                gui::MainMenuResult::Selected {
                    selected: gui::MainMenuSelection::NewGame,
                } => {
                    self.game_over_cleanup();
                    newrunstate = RunState::MapGeneration {};
                    self.mapgen_next_state = Some(RunState::PreRun);
                    self.generate_world_map(1);
                }
                gui::MainMenuResult::Selected {
                    selected: gui::MainMenuSelection::LoadGame,
                } => {
                    newrunstate = RunState::LoadMenu {
                        menu_selection: gui::LoadMenuSelection::Selecting(0),
                    };
                }
                gui::MainMenuResult::Selected {
                    selected: gui::MainMenuSelection::Quit,
                } => {
                    ::std::process::exit(0);
                }
            },
            RunState::LoadMenu { .. } => match gui::load_menu(self, ctx) {
                gui::LoadMenuResult::NoSelection { selected } => {
                    newrunstate = RunState::LoadMenu {
                        menu_selection: selected,
                    };
                }
                gui::LoadMenuResult::Selected {
                    selected: gui::LoadMenuSelection::Selecting(selected),
                } => {
                    let saved_files = saveload_system::list_save_files();
                    saveload_system::load_game(&mut self.ecs, &saved_files[selected as usize]);
                    newrunstate = RunState::AwaitingInput;
                    saveload_system::delete_save(&saved_files[selected as usize]);
                }
                gui::LoadMenuResult::Selected {
                    selected: gui::LoadMenuSelection::Quit,
                } => {
                    newrunstate = RunState::MainMenu {
                        menu_selection: gui::MainMenuSelection::NewGame,
                    };
                }
            },
            RunState::NextLevel => {
                self.goto_next_level();
                newrunstate = RunState::MapGeneration {};
                self.mapgen_next_state = Some(RunState::PreRun);
            }
            RunState::ShowRemoveItem => match gui::remove_item_menu(self, ctx) {
                (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                (gui::ItemMenuResult::NoResponse, _) => {}
                (gui::ItemMenuResult::Selected, item_entity) => {
                    let item_entity = item_entity.unwrap();
                    let mut intent = self.ecs.write_storage::<WantsToRemoveItem>();
                    intent
                        .insert(
                            *self.ecs.fetch::<Entity>(),
                            WantsToRemoveItem { item: item_entity },
                        )
                        .expect("Unable to insert intent");
                    newrunstate = RunState::PlayerTurn;
                }
            },
            RunState::GameOver => match gui::game_over(ctx) {
                gui::GameOverResult::NoSelection => {}
                gui::GameOverResult::QuitToMenu => {
                    newrunstate = RunState::MainMenu {
                        menu_selection: gui::MainMenuSelection::NewGame,
                    };
                    self.game_over_cleanup();
                }
            },
            RunState::MagicMapReveal {
                remaining_power,
                offset,
            } => {
                let player_pos = self.ecs.fetch::<Point>();
                let mut map = self.ecs.fetch_mut::<Map>();

                // first row
                let top_row = player_pos.y - offset;

                if top_row >= 0 {
                    for x in (player_pos.x - offset)..(player_pos.x + offset) {
                        if x < 0 || x >= (MAP_WIDTH as i32 - 1) {
                            continue;
                        }
                        let idx = map.xy_idx(x as i32, top_row);
                        map.revealed_tiles[idx] = true;
                    }
                }

                // bottom row
                let bottom_row = player_pos.y + offset;

                if bottom_row < (MAP_HEIGHT as i32 - 1) {
                    for x in (player_pos.x - offset)..(player_pos.x + offset) {
                        if x < 0 || x >= (MAP_WIDTH as i32 - 1) {
                            continue;
                        }
                        let idx = map.xy_idx(x as i32, bottom_row);
                        map.revealed_tiles[idx] = true;
                    }
                }

                // left col
                let left_col = player_pos.x - offset;

                if left_col >= 0 {
                    for y in (player_pos.y - offset)..(player_pos.y + offset) {
                        if y < 0 || y >= (MAP_HEIGHT as i32 - 1) {
                            continue;
                        }
                        let idx = map.xy_idx(left_col, y as i32);
                        map.revealed_tiles[idx] = true;
                    }
                }

                // right col
                let right_col = player_pos.x + offset;

                if right_col < (MAP_WIDTH as i32 - 1) {
                    for y in (player_pos.y - offset)..(player_pos.y + offset) {
                        if y < 0 || y >= (MAP_HEIGHT as i32 - 1) {
                            continue;
                        }
                        let idx = map.xy_idx(right_col, y as i32);
                        map.revealed_tiles[idx] = true;
                    }
                }

                if remaining_power as usize == 0 {
                    newrunstate = RunState::MonsterTurn;
                } else {
                    newrunstate = RunState::MagicMapReveal {
                        remaining_power: remaining_power - 1,
                        offset: offset + 1,
                    }
                }
            }
        }

        {
            let mut runstatewriter = self.ecs.write_resource::<RunState>();
            *runstatewriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);
    }
}

impl State {
    fn generate_world_map(&mut self, new_depth: i32) {
        self.mapgen_index = 0;
        self.mapgen_timer = 0.0;
        self.mapgen_history.clear();

        let mut builder = map_builders::random_builder(new_depth);
        builder.build_map();
        self.mapgen_history = builder.get_snapshot_history();
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            *worldmap_resource = builder.get_map();
        }

        // Spawn room
        builder.spawn_entities(&mut self.ecs);

        // Place the player and update resources
        let player_pos = builder.get_starting_position();
        let mut ppos = self.ecs.write_resource::<rltk::Point>();
        *ppos = rltk::Point::new(player_pos.x, player_pos.y);
        let mut position_components = self.ecs.write_storage::<Position>();
        let player_entity = self.ecs.fetch::<Entity>();
        let player_pos_comp = position_components.get_mut(*player_entity);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = player_pos.x;
            player_pos_comp.y = player_pos.y;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        if let Some(vs) = viewshed_components.get_mut(*player_entity) {
            vs.dirty = true;
        }
    }

    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();
        let equipped = self.ecs.read_storage::<Equipped>();

        // let mut to_delete: Vec<Entity> = Vec::new();
        // for entity in entities.join() {
        //     let mut should_delete = player.get(entity).is_none() && backpack.get(entity).is_none();

        //     if should_delete {
        //         to_delete.push(entity);
        //     }
        // }

        // to_delete
        entities
            .join()
            .filter(|ent| {
                player.get(*ent).is_none()
                    && if let Some(bp) = backpack.get(*ent) {
                        bp.owner != *player_entity
                    } else if let Some(eq) = equipped.get(*ent) {
                        eq.owner != *player_entity
                    } else {
                        true
                    }
            })
            .collect::<Vec<Entity>>()
    }

    fn goto_next_level(&mut self) {
        // Delete entities that aren't the player or his/her equipment
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs
                .delete_entity(target)
                .expect("Unable to delete entity");
        }

        let current_depth;
        {
            let worldmap_resource = self.ecs.write_resource::<Map>();
            current_depth = worldmap_resource.depth;
        }

        self.generate_world_map(current_depth + 1);

        // Notify the player and give them some health
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog
            .entries
            .push("You descend to the next level, and take a moment to heal.".to_string());
        let mut player_health_store = self.ecs.write_storage::<CombatStats>();
        let player_entity = self.ecs.fetch::<Entity>();
        if let Some(health) = player_health_store.get_mut(*player_entity) {
            health.hp = i32::max(health.hp, health.max_hp / 2);
        }
    }

    fn game_over_cleanup(&mut self) {
        // Delete everything
        let to_delete: Vec<_> = self.ecs.entities().join().collect();
        for del in to_delete.iter() {
            self.ecs
                .delete_entity(*del)
                .expect("Entity deletion failed");
        }

        // Clear the log
        {
            let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
            gamelog
                .entries
                .clear();
        }

        // Spawn a new player
        {
            let player_entity = spawner::spawn_player(&mut self.ecs, 0, 0);
            let mut player_entity_writer = self.ecs.write_resource::<Entity>();
            *player_entity_writer = player_entity;
        }
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        //.with_fullscreen(true)
        .build()?;

    context.with_post_scanlines(true);

    // Initialize Game State
    let mut gs = State {
        ecs: World::new(),
        mapgen_next_state: Some(RunState::MainMenu {
            menu_selection: gui::MainMenuSelection::NewGame,
        }),
        mapgen_index: 0,
        mapgen_history: Vec::new(),
        mapgen_timer: 0.0,
    };

    // Register Components
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<MagicStats>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<ProvidesManaRestore>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();
    gs.ecs.register::<Confusion>();
    gs.ecs.register::<SimpleMarker<SerializeMe>>();
    gs.ecs.register::<SerializationHelper>();
    gs.ecs.register::<Equippable>();
    gs.ecs.register::<Equipped>();
    gs.ecs.register::<MeleePowerBonus>();
    gs.ecs.register::<DefenseBonus>();
    gs.ecs.register::<WantsToRemoveItem>();
    gs.ecs.register::<ParticleLifetime>();
    gs.ecs.register::<HungerClock>();
    gs.ecs.register::<ProvidesFood>();
    gs.ecs.register::<MagicMapper>();
    gs.ecs.register::<Hidden>();
    gs.ecs.register::<EntryTrigger>();
    gs.ecs.register::<EntityMoved>();
    gs.ecs.register::<SingleActivation>();

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    // Add a Random Number Generator as a resource
    gs.ecs.insert(rltk::RandomNumberGenerator::new());

    // Insert placeholder values for map and player positions
    gs.ecs.insert(Map::new(1));
    gs.ecs.insert(Point::new(0, 0));

    let player_entity = spawner::spawn_player(&mut gs.ecs, 0, 0);
    gs.ecs.insert(player_entity);

    // Turn RunState into a resource
    gs.ecs.insert(RunState::MainMenu {
        menu_selection: gui::MainMenuSelection::NewGame,
    });

    // Add gamelog as a resource
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to Rusty Roguelike".to_string()],
    });

    // Add Particle System as a service/resource
    gs.ecs.insert(particle_system::ParticleBuilder::new());

    // Add Rex assets as a resource
    gs.ecs.insert(rex_assets::RexAssets::new());

    rltk::main_loop(context, gs)
}
