//! lat-core: the LAT (Level Administration Team) toolkit's logic, with no UI.
//!
//! LAT staff edit a *live* server's world: they open a level from the
//! server, change it and upload it again with their LAT rights. That is the
//! difference from the level editor in the Seihai client, which edits files
//! on disk in offline mode and stays there.
//!
//! | Module | What | State |
//! |---|---|---|
//! | [`remote`] | The file link: list, download, upload. A local folder, or [`remote::RcFiles`] over `seihai-rc` | working |
//! | [`diff`] | The line diff the editors show before an upload, and when the server's copy changed | working |
//! | [`level`] | Open a level, change a tile, write `GLEVNW01` text, paint with `seihai-draw` | editing |
//! | [`gani`] | Open a gani, change a sprite frame, write it back, paint one frame | editing |
//! | [`gpak`] | Resource packages: build a `.gpak` or encrypted `.gpake`, keep the gserver's key file | working |
//! | [`code_cache`] | `GWEBL001` level caches and `GRESCR01` script caches | working |
//! | [`preview`] | The RGBA buffers those editors hand the window | working |
//! | [`npc`] | NPC scripts: the GS1 check of level NPCs, used by [`check`] and the NPC editor page; NPC-server objects are planned | level NPCs working |
//! | [`gifcopy`] | GIF copies of PNG art for pre-2.1 clients (`check --make-gif`, opt-in) | working |
//! | [`worldmap`] | Build a `.gmap` from a level list, generator-style names, or a linked world's edge links | working |
//! | [`generate`] | Terrain, image-to-levels and dungeon worlds (TerrainGenerator, GraalLevelGen, the NW Dungeon Generator) | working |
//! | [`anim`] | gif2mng: a GIF as a PNG (one frame) or an MNG animation | working |
//! | [`levelfill`] | LevelFiller: scatter a `.nwt` theme's objects over levels; LFThemeMaker's patch themes | working, with borders and neighbouring levels |
//! | [`lfscript`] | LevelFiller's object language for complex theme objects | working |
//! | [`movie`] | GraalShop movie mode (`GANI0002`): actors at a frame, SPRITE, CHAR and TEXT actors painted | working |
//! | [`tmx`] | Levels to and from Tiled `.tmx` maps (NW2TMX, nw-converter) | working |
//! | [`render`] | Levels and whole gmaps as PNG pictures (NW2PNG's job): board, chests, signs, and with loaders NPC images and ganis | working |
//! | [`files`] | A world folder's files by name, and the level, picture and gani loaders the offline tools hand the painters | working |
//! | [`asset3d`] | 3D assets for a playerworld: glTF, OBJ, STL, 3DS and pictures as `.3ds` shapes and textures a 5.12 draws | working |
//! | [`check`] | The world and config checker | parse, links, images and sounds, NPC scripts, packages |
//!
//! Parsers come from `seihai-core` and painting from `seihai-draw`, both
//! pinned in the workspace `Cargo.toml`. Do not write a second level or
//! gani parser, and do not copy the drawing (`docs/ARCHITECTURE.md`).

pub mod anim;
pub mod asset3d;
pub mod check;
pub mod code_cache;
pub mod diff;
pub mod files;
pub mod gani;
pub mod generate;
pub mod gifcopy;
pub mod gpak;
pub mod level;
pub mod levelfill;
pub mod lfscript;
pub mod movie;
pub mod ncdb;
pub mod npc;
pub mod preview;
pub mod pw3d;
pub mod remote;
pub mod render;
pub mod terrain3d;
pub mod terrain_chunks;
pub mod tmx;
pub mod world3d;
pub mod worldmap;
