use rltk::rex::XpFile;

const SMALL_DUNGEON_PATH: &'static str = "../resources/SmallDungeon_80x50.xp";

rltk::embedded_resource!(SMALL_DUNGEON, "../resources/SmallDungeon_80x50.xp");

pub struct RexAssets {
    pub menu: XpFile,
}

impl RexAssets {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RexAssets {
        rltk::link_resource!(SMALL_DUNGEON, SMALL_DUNGEON_PATH);

        let menu = XpFile::from_resource(SMALL_DUNGEON_PATH).expect(&format!(
            "could not initialize xp file at {}",
            SMALL_DUNGEON_PATH
        ));

        RexAssets { menu }
    }
}
