use crate::{rex_assets::RexAssets, Hidden};

use super::{
    CombatStats, Equipped, GameLog, HungerClock, HungerState, InBackpack, MagicStats, Map, Name,
    Player, Position, RunState, State, Viewshed,
};
use rltk::{Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;

pub fn draw_ui(ecs: &World, ctx: &mut Rltk) {
    ctx.draw_box(
        0,
        43,
        79,
        6,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );

    // Health Bar
    let combat_stats = ecs.read_storage::<CombatStats>();
    let magic_stats = ecs.read_storage::<MagicStats>();
    let player = ecs.read_storage::<Player>();
    let hunger_status = ecs.read_storage::<HungerClock>();

    let map = ecs.fetch::<Map>();
    let depth = format!("Depth: {}", map.depth);
    ctx.print_color(
        2,
        43,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        &depth,
    );

    // Health Bar
    for (_player, stats) in (&player, &combat_stats).join() {
        let health = format!("HP: {} / {}", stats.hp, stats.max_hp);
        ctx.print_color(
            14,
            43,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            health,
        );

        ctx.draw_bar_horizontal(
            28,
            43,
            22,
            stats.hp,
            stats.max_hp,
            RGB::named(rltk::RED),
            RGB::named(rltk::BLACK),
        );
    }
    // Mana Bar
    for (_player, stats) in (&player, &magic_stats).join() {
        let mana = format!("MANA: {} / {}", stats.mana, stats.max_mana);
        ctx.print_color(
            51,
            43,
            RGB::named(rltk::MAGENTA),
            RGB::named(rltk::BLACK),
            mana,
        );

        ctx.draw_bar_horizontal(
            56,
            43,
            22,
            stats.mana,
            stats.max_mana,
            RGB::named(rltk::MAGENTA),
            RGB::named(rltk::BLACK),
        );
    }

    // Hunger
    for (_player, hunger) in (&player, &hunger_status).join() {
        match hunger.state {
            HungerState::WellFed => ctx.print_color(
                35,
                49,
                RGB::named(rltk::GREEN),
                RGB::named(rltk::BLACK),
                "Well Fed",
            ),
            HungerState::Hungry => ctx.print_color(
                35,
                49,
                RGB::named(rltk::ORANGE),
                RGB::named(rltk::BLACK),
                "Hungry",
            ),
            HungerState::Starving => ctx.print_color(
                35,
                49,
                RGB::named(rltk::RED),
                RGB::named(rltk::BLACK),
                "Starving",
            ),
            HungerState::Normal => {}
        }
    }

    // Log
    let log = ecs.fetch::<GameLog>();

    let mut y = 44;
    for s in log.entries.iter().rev() {
        if y < 49 {
            ctx.print(2, y, s);
        }
        y += 1;
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::MAGENTA));

    draw_tooltips(ecs, ctx);
}

fn draw_tooltips(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let names = ecs.read_storage::<Name>();
    let positions = ecs.read_storage::<Position>();
    let hidden = ecs.read_storage::<Hidden>();

    let mouse_pos = ctx.mouse_pos();
    if mouse_pos.0 >= map.width || mouse_pos.1 >= map.height {
        return;
    }

    let mut tooltip: Vec<String> = Vec::new();
    for (name, position, _hidden) in (&names, &positions, !&hidden).join() {
        let idx = map.xy_idx(position.x, position.y);
        if position.x == mouse_pos.0 && position.y == mouse_pos.1 && map.visible_tiles[idx] {
            tooltip.push(name.name.to_string());
        }
    }

    if !tooltip.is_empty() {
        let mut width: i32 = 0;
        for s in tooltip.iter() {
            if width < s.len() as i32 {
                width = s.len() as i32;
            }
        }
        width += 3;

        if mouse_pos.0 > 40 {
            let arrow_pos = Point::new(mouse_pos.0 - 2, mouse_pos.1);
            let left_x = mouse_pos.0 - width;
            let mut y = mouse_pos.1;

            for s in tooltip.iter() {
                ctx.print_color(
                    left_x,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GREY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x - i,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GREY),
                        &" ".to_string(),
                    );
                }
                y += 1;
            }

            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GREY),
                &"->".to_string(),
            );
        } else {
            let arrow_pos = Point::new(mouse_pos.0 + 1, mouse_pos.1);
            let left_x = mouse_pos.0 + 3;
            let mut y = mouse_pos.1;

            for s in tooltip.iter() {
                ctx.print_color(
                    left_x + 1,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GREY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x + i + 1,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GREY),
                        &" ".to_string(),
                    );
                }
                y += 1;
            }

            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GREY),
                &"<-".to_string(),
            );
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel,
    NoResponse,
    Selected,
}

pub fn show_inventory(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Inventory",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names)
        .join()
        .filter(|item| item.1.owner == *player_entity)
    {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(VirtualKeyCode::Escape) => (ItemMenuResult::Cancel, None),
        Some(key) => {
            let selection = rltk::letter_to_option(key);
            if selection > -1 && selection < count as i32 {
                return (
                    ItemMenuResult::Selected,
                    Some(equippable[selection as usize]),
                );
            }
            (ItemMenuResult::NoResponse, None)
        }
    }
}

pub fn drop_item_menu(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Inventory",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names)
        .join()
        .filter(|item| item.1.owner == *player_entity)
    {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(VirtualKeyCode::Escape) => (ItemMenuResult::Cancel, None),
        Some(key) => {
            let selection = rltk::letter_to_option(key);
            if selection > -1 && selection < count as i32 {
                return (
                    ItemMenuResult::Selected,
                    Some(equippable[selection as usize]),
                );
            }
            (ItemMenuResult::NoResponse, None)
        }
    }
}

pub fn ranged_target(
    gs: &mut State,
    ctx: &mut Rltk,
    range: i32,
) -> (ItemMenuResult, Option<Point>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    ctx.print_color(
        5,
        0,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Select Target:",
    );

    // Highlight available target cells
    let mut available_cells = Vec::new();
    if let Some(visible) = viewsheds.get(*player_entity) {
        for idx in visible.visible_tiles.iter() {
            let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
            if distance <= range as f32 {
                ctx.set_bg(idx.x, idx.y, RGB::named(rltk::BLUE));
                available_cells.push(idx);
            }
        }
    } else {
        return (ItemMenuResult::Cancel, None);
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    let valid_target = available_cells
        .iter()
        .any(|&idx| idx.x == mouse_pos.0 && idx.y == mouse_pos.1);
    if valid_target {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::CYAN));
        if ctx.left_click {
            return (
                ItemMenuResult::Selected,
                Some(Point::new(mouse_pos.0, mouse_pos.1)),
            );
        }
    } else {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::CYAN));
        if ctx.left_click {
            return (ItemMenuResult::Cancel, None);
        }
    }

    (ItemMenuResult::NoResponse, None)
}

/*
 *  MAIN MENU
 */

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
    NewGame,
    SaveGame,
    LoadGame,
    Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection { selected: MainMenuSelection },
    Selected { selected: MainMenuSelection },
}

pub fn main_menu(gs: &mut State, ctx: &mut Rltk) -> MainMenuResult {
    let runstate = gs.ecs.fetch::<RunState>();
    let save_exists = super::saveload_system::does_save_exist();

    let assets = gs.ecs.fetch::<RexAssets>();
    ctx.render_xp_sprite(&assets.menu, 0, 0);

    // Enclose everything in a box
    ctx.draw_box_double(
        24,
        21,
        31,
        10,
        RGB::named(rltk::WHEAT),
        RGB::named(rltk::BLACK),
    );

    ctx.print_color_centered(
        15,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Rust Roguelike Tutorial",
    );

    if let RunState::MainMenu {
        menu_selection: selection,
    } = *runstate
    {
        let mut y = 23;

        if selection == MainMenuSelection::NewGame {
            ctx.print_color_centered(
                y,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Begin New Game",
            );
        } else {
            ctx.print_color_centered(
                y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::BLACK),
                "Begin New Game",
            );
        }
        y += 2;

        if selection == MainMenuSelection::SaveGame {
            ctx.print_color_centered(
                y,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Save Game",
            );
        } else {
            ctx.print_color_centered(
                y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::BLACK),
                "Save Game",
            );
        }
        y += 2;

        if save_exists {
            if selection == MainMenuSelection::LoadGame {
                ctx.print_color_centered(
                    y,
                    RGB::named(rltk::MAGENTA),
                    RGB::named(rltk::BLACK),
                    "Load Game",
                );
            } else {
                ctx.print_color_centered(
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::BLACK),
                    "Load Game",
                );
            }
            y += 2;
        }

        if selection == MainMenuSelection::Quit {
            ctx.print_color_centered(
                y,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Quit",
            );
        } else {
            ctx.print_color_centered(y, RGB::named(rltk::WHITE), RGB::named(rltk::BLACK), "Quit");
        }

        match ctx.key {
            None => {
                return MainMenuResult::NoSelection {
                    selected: selection,
                }
            }
            Some(VirtualKeyCode::Escape) => {
                return MainMenuResult::NoSelection {
                    selected: MainMenuSelection::Quit,
                };
            }
            Some(VirtualKeyCode::Up) => {
                let mut newselection;
                match selection {
                    MainMenuSelection::NewGame => newselection = MainMenuSelection::Quit,
                    MainMenuSelection::SaveGame => newselection = MainMenuSelection::NewGame,
                    MainMenuSelection::LoadGame => newselection = MainMenuSelection::SaveGame,
                    MainMenuSelection::Quit => newselection = MainMenuSelection::LoadGame,
                }

                if newselection == MainMenuSelection::LoadGame && !save_exists {
                    newselection = MainMenuSelection::SaveGame;
                }

                return MainMenuResult::NoSelection {
                    selected: newselection,
                };
            }
            Some(VirtualKeyCode::Down) => {
                let mut newselection;
                match selection {
                    MainMenuSelection::NewGame => newselection = MainMenuSelection::SaveGame,
                    MainMenuSelection::SaveGame => newselection = MainMenuSelection::LoadGame,
                    MainMenuSelection::LoadGame => newselection = MainMenuSelection::Quit,
                    MainMenuSelection::Quit => newselection = MainMenuSelection::NewGame,
                }

                if newselection == MainMenuSelection::LoadGame && !save_exists {
                    newselection = MainMenuSelection::Quit;
                }

                return MainMenuResult::NoSelection {
                    selected: newselection,
                };
            }
            Some(VirtualKeyCode::Return) => {
                return MainMenuResult::Selected {
                    selected: selection,
                }
            }
            _ => {
                return MainMenuResult::NoSelection {
                    selected: selection,
                }
            }
        }
    }
    MainMenuResult::NoSelection {
        selected: MainMenuSelection::NewGame,
    }
}

/*
 *  LOAD MENU
 */

#[derive(PartialEq, Copy, Clone)]
pub enum LoadMenuSelection {
    Quit,
    Selecting(i32),
}

#[derive(PartialEq, Copy, Clone)]
pub enum LoadMenuResult {
    NoSelection { selected: LoadMenuSelection },
    Selected { selected: LoadMenuSelection },
}

pub fn load_menu(gs: &mut State, ctx: &mut Rltk) -> LoadMenuResult {
    let runstate = gs.ecs.fetch::<RunState>();
    let saved_files = super::saveload_system::list_save_files();

    let assets = gs.ecs.fetch::<RexAssets>();
    ctx.render_xp_sprite(&assets.menu, 0, 0);

    // Enclose everything in a box
    ctx.draw_box_double(
        24,
        21,
        31,
        10,
        RGB::named(rltk::WHEAT),
        RGB::named(rltk::BLACK),
    );

    if let RunState::LoadMenu {
        menu_selection: LoadMenuSelection::Selecting(selection),
    } = *runstate
    {
        let y = 23;

        for (i, file_name) in saved_files.iter().enumerate() {
            if selection == i as i32 {
                ctx.print_color_centered(
                    y + i,
                    RGB::named(rltk::MAGENTA),
                    RGB::named(rltk::BLACK),
                    &file_name,
                );
            } else {
                ctx.print_color_centered(
                    y + i,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::BLACK),
                    &file_name,
                );
            }
        }

        match ctx.key {
            None => {
                return LoadMenuResult::NoSelection {
                    selected: LoadMenuSelection::Selecting(selection),
                }
            }
            Some(VirtualKeyCode::Escape) => {
                return LoadMenuResult::Selected {
                    selected: LoadMenuSelection::Quit,
                };
            }
            Some(VirtualKeyCode::Down) => {
                return LoadMenuResult::NoSelection {
                    selected: LoadMenuSelection::Selecting(
                        (selection + 1) % saved_files.len() as i32,
                    ),
                };
            }
            Some(VirtualKeyCode::Up) => {
                return LoadMenuResult::NoSelection {
                    selected: LoadMenuSelection::Selecting(
                        (selection - 1 + saved_files.len() as i32) % saved_files.len() as i32,
                    ),
                };
            }
            Some(VirtualKeyCode::Return) => {
                return LoadMenuResult::Selected {
                    selected: LoadMenuSelection::Selecting(selection),
                }
            }
            _ => {
                return LoadMenuResult::NoSelection {
                    selected: LoadMenuSelection::Selecting(selection),
                }
            }
        }
    }
    LoadMenuResult::NoSelection {
        selected: LoadMenuSelection::Selecting(0),
    }
}

pub fn remove_item_menu(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let equipped = gs.ecs.read_storage::<Equipped>();
    let entities = gs.ecs.entities();

    let inventory = (&equipped, &names)
        .join()
        .filter(|item| item.0.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Inventory",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESCAPE to cancel",
    );

    let mut removable: Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &equipped, &names)
        .join()
        .filter(|item| item.1.owner == *player_entity)
    {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print(21, y, &name.name.to_string());
        removable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(VirtualKeyCode::Escape) => (ItemMenuResult::Cancel, None),
        Some(key) => {
            let selection = rltk::letter_to_option(key);
            if selection > -1 && selection < count as i32 {
                return (
                    ItemMenuResult::Selected,
                    Some(removable[selection as usize]),
                );
            }
            (ItemMenuResult::NoResponse, None)
        }
    }
}

pub enum GameOverResult {
    NoSelection,
    QuitToMenu,
}

pub fn game_over(ctx: &mut Rltk) -> GameOverResult {
    // Enclose everything in a box
    ctx.draw_box_double(
        10,
        13,
        61,
        10,
        RGB::named(rltk::WHEAT),
        RGB::named(rltk::BLACK),
    );

    ctx.print_color_centered(
        15,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Your journey has ended!",
    );
    ctx.print_color_centered(
        17,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "One day, we'll tell you all about how you did.",
    );
    ctx.print_color_centered(
        18,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "That day, sadly is not in this chapter..",
    );

    ctx.print_color_centered(
        20,
        RGB::named(rltk::MAGENTA),
        RGB::named(rltk::BLACK),
        "Press any key to return to the main menu",
    );

    match ctx.key {
        None => GameOverResult::NoSelection,
        Some(_) => GameOverResult::QuitToMenu,
    }
}
