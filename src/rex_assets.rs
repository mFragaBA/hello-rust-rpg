use rltk::rex::XpFile;

const SMALL_DUNGEON_PATH: &'static str = "../resources/SmallDungeon_80x50.xp";
const WFC_DEMO_IMAGE1_PATH: &'static str = "../resources/wfc-demo1.xp";
const WFC_DEMO_IMAGE2_PATH: &'static str = "../resources/wfc-demo2.xp";

rltk::embedded_resource!(SMALL_DUNGEON, "../resources/SmallDungeon_80x50.xp");
rltk::embedded_resource!(WFC_DEMO_IMAGE1, "../resources/wfc-demo1.xp");
rltk::embedded_resource!(WFC_DEMO_IMAGE2, "../resources/wfc-demo2.xp");

pub struct RexAssets {
    pub menu: XpFile,
}

impl RexAssets {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RexAssets {
        rltk::link_resource!(SMALL_DUNGEON, SMALL_DUNGEON_PATH);
        rltk::link_resource!(WFC_DEMO_IMAGE1, WFC_DEMO_IMAGE1_PATH);
        rltk::link_resource!(WFC_DEMO_IMAGE2, WFC_DEMO_IMAGE2_PATH);

        let menu = XpFile::from_resource(SMALL_DUNGEON_PATH).expect(&format!(
            "could not initialize xp file at {}",
            SMALL_DUNGEON_PATH
        ));

        RexAssets { menu }
    }
}
