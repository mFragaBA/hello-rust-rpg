use super::components::*;
use specs::error::NoError;
use specs::prelude::*;
use specs::saveload::{
    DeserializeComponents, MarkedBuilder, SerializeComponents, SimpleMarker, SimpleMarkerAllocator,
};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::Path;

macro_rules! serialize_individually {
    ($ecs:expr, $ser:expr, $data:expr, $( $type:ty),*) => {
        $(
        SerializeComponents::<NoError, SimpleMarker<SerializeMe>>::serialize(
            &( $ecs.read_storage::<$type>(), ),
            &$data.0,
            &$data.1,
            &mut $ser,
        )
        .unwrap();
        )*
    };
}

macro_rules! deserialize_individually {
    ($ecs:expr, $de:expr, $data:expr, $( $type:ty),*) => {
        $(
        DeserializeComponents::<NoError, _>::deserialize(
            &mut ( &mut $ecs.write_storage::<$type>(), ),
            &mut $data.0, // entities
            &mut $data.1, // marker
            &mut $data.2, // allocater
            &mut $de,
        )
        .unwrap();
        )*
    };
}

#[cfg(target_arch = "wasm32")]
pub fn save_game(ecs: &mut World) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_game(ecs: &mut World, save_name: &str) {
    // Create helper
    let mapcopy = ecs.get_mut::<super::map::Map>().unwrap().clone();
    let savehelper = ecs
        .create_entity()
        .with(SerializationHelper { map: mapcopy })
        .marked::<SimpleMarker<SerializeMe>>()
        .build();

    // Actualliy serialize
    {
        let data = (
            ecs.entities(),
            ecs.read_storage::<SimpleMarker<SerializeMe>>(),
        );

        let writer = File::create(&format!("./save_files/{}.json", save_name)).unwrap();
        let mut serializer = serde_json::Serializer::new(writer);
        serialize_individually!(
            ecs,
            serializer,
            data,
            Position,
            Renderable,
            Player,
            Viewshed,
            Monster,
            Name,
            BlocksTile,
            CombatStats,
            SufferDamage,
            WantsToMelee,
            Item,
            Consumable,
            Ranged,
            InflictsDamage,
            AreaOfEffect,
            Confusion,
            ProvidesHealing,
            InBackpack,
            WantsToPickupItem,
            WantsToUseItem,
            WantsToDropItem,
            MagicStats,
            ProvidesManaRestore,
            SerializationHelper,
            Equippable,
            Equipped,
            MeleePowerBonus,
            DefenseBonus,
            WantsToRemoveItem,
            ParticleLifetime,
            HungerClock,
            ProvidesFood,
            MagicMapper
        );
    }

    // Clean up
    ecs.delete_entity(savehelper).expect("Crash on cleanup");
}

pub fn does_save_exist() -> bool {
    let dir = Path::new("./save_files/");

    if !dir.is_dir() {
        std::fs::create_dir(dir).expect("Unable to delete file");
    }

    fs::read_dir(dir)
        .expect("couldn't read save files directory")
        .any(|dir_entry| {
            let entry = dir_entry.unwrap();
            let path = entry.path();

            path.is_file() && path.extension() == Some(OsStr::new("json"))
        })
}

pub fn list_save_files() -> Vec<String> {
    let dir = Path::new("./save_files/");

    if !dir.is_dir() {
        std::fs::create_dir(dir).expect("Unable to delete file");
    }

    fs::read_dir(dir)
        .expect("couldn't read save files directory")
        .map(|dir_entry| {
            let entry = dir_entry.unwrap();
            let path = entry.path();

            path.file_stem().unwrap().to_str().unwrap().to_string()
        })
        .collect::<Vec<_>>()
}

pub fn load_game(ecs: &mut World, save_name: &str) {
    {
        // Delete everything
        let mut to_delete = Vec::new();
        for e in ecs.entities().join() {
            to_delete.push(e);
        }
        for del in to_delete.iter() {
            ecs.delete_entity(*del).expect("Deletion failed");
        }
    }

    let data = fs::read_to_string(&format!("./save_files/{}.json", save_name)).unwrap();
    let mut de = serde_json::Deserializer::from_str(&data);

    {
        let mut data = (
            &mut ecs.entities(),
            &mut ecs.write_storage::<SimpleMarker<SerializeMe>>(),
            &mut ecs.write_resource::<SimpleMarkerAllocator<SerializeMe>>(),
        );
        deserialize_individually!(
            ecs,
            de,
            data,
            Position,
            Renderable,
            Player,
            Viewshed,
            Monster,
            Name,
            BlocksTile,
            CombatStats,
            SufferDamage,
            WantsToMelee,
            Item,
            Consumable,
            Ranged,
            InflictsDamage,
            AreaOfEffect,
            Confusion,
            ProvidesHealing,
            InBackpack,
            WantsToPickupItem,
            WantsToUseItem,
            WantsToDropItem,
            MagicStats,
            ProvidesManaRestore,
            SerializationHelper,
            Equippable,
            Equipped,
            MeleePowerBonus,
            DefenseBonus,
            WantsToRemoveItem,
            ParticleLifetime,
            HungerClock,
            ProvidesFood,
            MagicMapper
        );
    }

    let mut deleteme: Option<Entity> = None;
    {
        let entities = ecs.entities();
        let helper = ecs.read_storage::<SerializationHelper>();
        let player = ecs.read_storage::<Player>();
        let position = ecs.read_storage::<Position>();
        for (e, h) in (&entities, &helper).join() {
            let mut worldmap = ecs.write_resource::<super::map::Map>();
            *worldmap = h.map.clone();
            worldmap.tile_content = vec![Vec::new(); super::map::MAP_COUNT];
            deleteme = Some(e);
        }
        for (e, _p, pos) in (&entities, &player, &position).join() {
            let mut ppos = ecs.write_resource::<rltk::Point>();
            *ppos = rltk::Point::new(pos.x, pos.y);
            let mut player_resource = ecs.write_resource::<Entity>();
            *player_resource = e;
        }
    }
    ecs.delete_entity(deleteme.unwrap())
        .expect("Unable to delete helper");
}

pub fn delete_save(save_name: &str) {
    if Path::new(&format!("./save_files/{}.json", save_name)).exists() {
        std::fs::remove_file(&format!("./save_files/{}.json", save_name))
            .expect("Unable to delete file");
    }
}
