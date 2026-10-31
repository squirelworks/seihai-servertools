//! Dated backups and "Undo last change". **Planned** (`docs/NEXT.md`, setup
//! step 1).
//!
//! Contract for whoever builds it:
//!
//! - Before any write, copy the file to `<name>.bak-YYYYMMDD-HHMMSS` in the
//!   same folder.
//! - Write through a temporary file in the same folder and a rename, so an
//!   interrupted save never leaves half a file.
//! - "Undo last change" restores the newest backup and keeps the current file
//!   as a backup of its own, so undo can itself be undone.
