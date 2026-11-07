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
//! | [`check`] | The world and config checker | parse, links, images and sounds, NPC scripts |
//!
//! Parsers come from `seihai-core` and painting from `seihai-draw`, both
//! pinned in the workspace `Cargo.toml`. Do not write a second level or
//! gani parser, and do not copy the drawing (`docs/ARCHITECTURE.md`).

pub mod check;
pub mod code_cache;
pub mod diff;
pub mod gani;
pub mod gpak;
pub mod level;
pub mod npc;
pub mod preview;
pub mod remote;
