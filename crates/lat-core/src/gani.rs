//! Ganis (animations) for the gani editor.
//!
//! **Today:** open with `seihai-core`'s parser, which the client uses to
//! play them. **Planned** (`docs/NEXT.md`, LAT track): a line-preserving gani
//! document (like the level one), a frame list and sprite table editor,
//! preview with the client's drawing code from the shared crate, upload
//! through [`crate::remote::RemoteFiles`].

pub use seihai_core::assets::Gani;

/// Parse a gani's text.
pub fn open(bytes: &[u8]) -> Result<Gani, String> {
    Gani::parse(&String::from_utf8_lossy(bytes))
}
