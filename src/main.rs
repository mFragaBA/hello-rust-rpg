use rltk::{Rltk, GameState, RGB};
use specs::prelude::*;

// Module Imports
mod components;
pub use components::*;
mod map;
pub use map::*;
mod player;
use player::*;
mod rect;
pub use rect::*;

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

#[derive(PartialEq, Copy, Clone)]
pub enum RunState { 
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
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
        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();
        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
        }

        {
            let mut runstatewriter = self.ecs.write_resource::<RunState>();
            *runstatewriter = newrunstate;
        }

        damage_system::delete_the_dead(&mut self.ecs);

        draw_map(&self.ecs, ctx);

        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();
        let map = self.ecs.fetch::<Map>();

        for (pos, render) in (&positions, &renderables).join() {
            let idx = map.xy_idx(pos.x, pos.y);
            if map.visible_tiles[idx] {
                ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
            }
        }
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
//        .with_fullscreen(true)
        .build()?;

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
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<WantsToMelee>();

    // Create Map
    let map = new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center();

    // ===== ENTITY CREATION =====
    
    // Player 
    let player_entity = gs.ecs
        .create_entity()
        .with(Position { x : player_x, y: player_y})
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::BLACK),
        })
        .with(Player {})
        .with(Viewshed{ visible_tiles: Vec::new(), range : 8, dirty: true })
        .with(Name{name: "Sir Player of Nottingham".to_string() })
        .with(CombatStats{ max_hp: 32, hp: 32, defense: 2, power: 5 })
        .build();

    // Monsters - One at the center of each room
    let mut rng = rltk::RandomNumberGenerator::new();
    for (i, room) in map.rooms.iter().skip(1).enumerate() {
        let (x, y) = room.center();

        let glyph : rltk::FontCharType;
        let name : String;
        let roll = rng.roll_dice(1, 2);

        if roll == 1 {
            glyph = rltk::to_cp437('g');
            name = "Goblin".to_string();
        } else {
            glyph = rltk::to_cp437('o');
            name = "Orc".to_string();
        }
        gs.ecs
            .create_entity()
            .with(Position {x: x, y: y})
            .with(Renderable {
                glyph: glyph,
                fg: RGB::named(rltk::RED),
                bg: RGB::named(rltk::BLACK)
            })
            .with(Viewshed{ visible_tiles: Vec::new(), range: 8, dirty: true})
            .with(Monster{})
            .with(Name{ name: format!("{} #{}", &name, i) })
            .with(BlocksTile{})
            .with(CombatStats{ max_hp: 16, hp: 16, defense: 1, power: 4 })
            .build();
    }

    // Insert map
    gs.ecs.insert(map);

    // Insert player position and entity. This is not encouraged for anything but player entities
    gs.ecs.insert(rltk::Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);

    // Turn RunState into a resource
    gs.ecs.insert(RunState::PreRun);

    rltk::main_loop(context, gs)
}
