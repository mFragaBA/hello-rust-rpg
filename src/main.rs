extern crate serde;
use rltk::{Rltk, GameState, RGB};
use specs::prelude::*;
use specs::saveload::{SimpleMarker, SimpleMarkerAllocator};

// Module Imports
mod components;
pub use components::*;
mod map;
pub use map::*;
mod player;
use player::*;
mod rect;
pub use rect::*;
mod gui;
mod gamelog;
use gamelog::GameLog;
mod spawner;

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
use inventory_system::{ItemCollectionSystem, ItemUseSystem, ItemDropSystem};
pub mod saveload_system;

#[derive(PartialEq, Copy, Clone)]
pub enum RunState { 
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting { range: i32, item: Entity},
    MainMenu { menu_selection: gui::MainMenuSelection },
}

pub struct State {
    pub ecs: World,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);
        let mut mob = MonsterAI{};
        mob.run_now(&self.ecs);
        let mut map_index = MapIndexingSystem{};
        map_index.run_now(&self.ecs);
        let mut melee_system = MeleeCombatSystem{};
        melee_system.run_now(&self.ecs);
        let mut dmg_system = DamageSystem{};
        dmg_system.run_now(&self.ecs);
        let mut pickup = ItemCollectionSystem{};
        pickup.run_now(&self.ecs);
        let mut potions = ItemUseSystem{};
        potions.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem{};
        drop_items.run_now(&self.ecs);
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

        match newrunstate {
            RunState::MainMenu{..} => {}
            _ => {
                draw_map(&self.ecs, ctx);

                {
                    let positions = self.ecs.read_storage::<Position>();
                    let renderables = self.ecs.read_storage::<Renderable>();
                    let map = self.ecs.fetch::<Map>();

                    let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
                    data.sort_by(|&a, &b| b.1.render_order.cmp(&a.1.render_order) );

                    for (pos, render) in data.iter() {
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
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                self.ecs.maintain();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                match gui::show_inventory(self, ctx) {
                    (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                    (gui::ItemMenuResult::NoResponse, _) => {}
                    (gui::ItemMenuResult::Selected, entity) => {
                        let entity = entity.unwrap();
                        if let Some(ranged_item) = self.ecs.read_storage::<Ranged>().get(entity) {
                            newrunstate = RunState::ShowTargeting{ range: ranged_item.range, item: entity };
                        } else {
                            let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                            intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem{ item: entity, target: None }).expect("Unable to insert intent");
                            newrunstate = RunState::PlayerTurn;
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                match gui::show_inventory(self, ctx) {
                    (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                    (gui::ItemMenuResult::NoResponse, _) => {}
                    (gui::ItemMenuResult::Selected, entity) => {
                        let entity = entity.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToDropItem{ item: entity }).expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowTargeting{range, item} => {
                match gui::ranged_target(self, ctx, range) {
                    (gui::ItemMenuResult::Cancel, _) => newrunstate = RunState::AwaitingInput,
                    (gui::ItemMenuResult::NoResponse, _) => {}
                    (gui::ItemMenuResult::Selected, entity) => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent.insert(*self.ecs.fetch::<Entity>(), WantsToUseItem{ item: item, target: entity }).expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::MainMenu { .. } => {
                match gui::main_menu(self, ctx) {
                    gui::MainMenuResult::NoSelection { selected } => {
                        newrunstate = RunState::MainMenu{ menu_selection: selected };
                    }
                    gui::MainMenuResult::Selected { selected: gui::MainMenuSelection::SaveGame } => {
                        saveload_system::save_game(&mut self.ecs);
                        newrunstate = RunState::MainMenu{ menu_selection: gui::MainMenuSelection::LoadGame };
                    }
                    gui::MainMenuResult::Selected { selected: gui::MainMenuSelection::NewGame } => {
                        newrunstate = RunState::PreRun;
                    }
                    gui::MainMenuResult::Selected { selected: gui::MainMenuSelection::LoadGame } => {
                        saveload_system::load_game(&mut self.ecs);
                        newrunstate = RunState::AwaitingInput;
                        saveload_system::delete_save();
                    }
                    gui::MainMenuResult::Selected { selected: gui::MainMenuSelection::Quit } => {
                        ::std::process::exit(0);
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

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    // Create Map
    let map = new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center();

    // ===== ENTITY CREATION =====
    
    // Add a Random Number Generator as a resource
    gs.ecs.insert(rltk::RandomNumberGenerator::new());
    
    // Player 
    let player_entity = spawner::spawn_player(&mut gs.ecs, player_x, player_y);
    
    // Monsters - One at the center of each room
    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }

    // Insert map
    gs.ecs.insert(map);

    // Insert player position and entity. This is not encouraged for anything but player entities
    gs.ecs.insert(rltk::Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);

    // Turn RunState into a resource
    gs.ecs.insert(RunState::MainMenu{ menu_selection: gui::MainMenuSelection::NewGame });
    
    // Add gamelog as a resource
    gs.ecs.insert(GameLog{ entries : vec!["Welcome to Rusty Roguelike".to_string()] });

    rltk::main_loop(context, gs)
}
