//! lat-core: the LAT (Level Administration Team) toolkit's logic, with no UI.
//!
//! LAT staff edit a *live* server's world: they open a level from the
//! server, change it and upload it again with their LAT rights. That is the
//! difference from the level editor in the Seihai client, which edits files
//! on disk in offline mode and stays there.
//!
//! | Module | What | State |
//! |---|---|---|
//! | [`remote`] | The file link: list, download, upload. A local folder today; the RC file browser later | trait + local folder |
//! | [`level`] | Open levels through `seihai-core`'s parser | open only |
//! | [`gani`] | Open ganis through `seihai-core`'s parser | open only |
//! | [`npc`] | NPC scripts in levels and on the NPC-server | planned |
//! | [`check`] | The world and config checker | first checks |
//!
//! Parsers come from `seihai-core`, pinned by tag in the workspace
//! `Cargo.toml`. Do not write a second level or gani parser here: fix or
//! extend the shared one in `squirelworks/seihai` and bump the tag
//! (`docs/ARCHITECTURE.md`, "Code shared with Seihai").

pub mod check;
pub mod gani;
pub mod level;
pub mod npc;
pub mod remote;
