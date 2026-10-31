//! Levels for the server-side level editor.
//!
//! **Today:** open a level's bytes (from [`crate::remote::RemoteFiles`]) with
//! `seihai-core`'s parser. **Planned** (`docs/NEXT.md`, LAT track):
//!
//! 1. A shared, line-preserving level *document* (paint, objects, NPC script
//!    pane, save byte-for-byte when unchanged). It already exists in the
//!    Seihai repo as `tools/level-editor`; it moves into a library crate
//!    there (with the tile drawing from `seihai-mae`) so both editors use
//!    one copy. See `docs/ARCHITECTURE.md`, "Code shared with Seihai".
//! 2. Open from a live server, keep the downloaded bytes, show a diff
//!    against them before upload, refuse to upload when the server's copy
//!    changed in the meantime (download again and compare).
//! 3. Upload with the LAT account through `RcFiles`.

pub use seihai_core::assets::Level;

/// Parse a `.nw` (`GLEVNW01`) or `.graal` (`GR-V1`, `Z3-V1`) level.
pub fn open(bytes: &[u8]) -> Result<Level, String> {
    Level::from_bytes(bytes)
}
