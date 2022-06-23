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
mod random_table;
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
    NextLevel,
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
                match gui::drop_item_menu(self, ctx) {
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
            RunState::NextLevel => {
                self.goto_next_level();
                newrunstate = RunState::PreRun;
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
    fn entities_to_remove_on_level_change(&mut self) -> Vec<Entity> {
        let entities = self.ecs.entities();
        let player = self.ecs.read_storage::<Player>();
        let backpack = self.ecs.read_storage::<InBackpack>();
        let player_entity = self.ecs.fetch::<Entity>();

       // let mut to_delete: Vec<Entity> = Vec::new();
       // for entity in entities.join() {
       //     let mut should_delete = player.get(entity).is_none() && backpack.get(entity).is_none();

       //     if should_delete {
       //         to_delete.push(entity);
       //     }
       // }

       // to_delete
       entities.join().filter(|ent| 
           player.get(*ent).is_none() &&
           if let Some(bp) = backpack.get(*ent) { bp.owner != *player_entity } else { true }
        ).collect::<Vec<Entity>>()
    }

    fn goto_next_level(&mut self) {
        // Delete entities that aren't the player or his/her equipment
        let to_delete = self.entities_to_remove_on_level_change();
        for target in to_delete {
            self.ecs.delete_entity(target).expect("Unable to delete entity");
        }

        // Build a new map and place the player
        let current_depth;
        let worldmap;
        {
            let mut worldmap_resource = self.ecs.write_resource::<Map>();
            current_depth = worldmap_resource.depth;
            *worldmap_resource = new_map_rooms_and_corridors(current_depth + 1);
            worldmap = worldmap_resource.clone();
        }

        // Spawn rooms
        for room in worldmap.rooms.iter().skip(1) {
            spawner::spawn_room(&mut self.ecs, room, current_depth + 1);
        }

        // Place the player and update resources
        let (px, py) = worldmap.rooms[0].center();
        let mut ppos = self.ecs.write_resource::<rltk::Point>();
        *ppos = rltk::Point::new(px, py);
        let mut position_components = self.ecs.write_storage::<Position>();
        let player_ent = self.ecs.fetch::<Entity>();
        let player_pos_comp = position_components.get_mut(*player_ent);
        if let Some(player_pos_comp) = player_pos_comp {
            player_pos_comp.x = px;
            player_pos_comp.y = py;
        }

        // Mark the player's visibility as dirty
        let mut viewshed_components = self.ecs.write_storage::<Viewshed>();
        if let Some(vs) = viewshed_components.get_mut(*player_ent) {
            vs.dirty = true;
        }

        // Notify the player and give them some health
        let mut gamelog = self.ecs.fetch_mut::<gamelog::GameLog>();
        gamelog.entries.push("You descend to the next level, and take a moment to heal.".to_string());
        let mut player_health_store = self.ecs.write_storage::<CombatStats>();
        if let Some(health) = player_health_store.get_mut(*player_ent) {
            health.hp = i32::max(health.hp, health.max_hp / 2);
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

    gs.ecs.insert(SimpleMarkerAllocator::<SerializeMe>::new());

    // Create Map
    let map = new_map_rooms_and_corridors(1);
    let (player_x, player_y) = map.rooms[0].center();

    // ===== ENTITY CREATION =====
    
    // Add a Random Number Generator as a resource
    gs.ecs.insert(rltk::RandomNumberGenerator::new());
    
    // Player 
    let player_entity = spawner::spawn_player(&mut gs.ecs, player_x, player_y);
    
    // Monsters - One at the center of each room
    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room, 1);
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
