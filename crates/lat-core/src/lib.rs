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
//! | [`level`] | Open a level, change a tile, write `GLEVNW01` text, paint with `seihai-draw` | editing |
//! | [`gani`] | Open a gani, change a sprite frame, write it back, paint one frame | editing |
//! | [`preview`] | The RGBA buffers those editors hand the window | working |
//! | [`npc`] | NPC scripts in levels and on the NPC-server | planned |
//! | [`check`] | The world and config checker | parse, links, images and sounds |
//!
//! Parsers come from `seihai-core` and painting from `seihai-draw`, both
//! pinned in the workspace `Cargo.toml`. Do not write a second level or
//! gani parser, and do not copy the drawing (`docs/ARCHITECTURE.md`).

pub mod check;
pub mod gani;
pub mod level;
pub mod npc;
pub mod preview;
pub mod remote;
