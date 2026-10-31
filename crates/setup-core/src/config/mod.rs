//! Config files the setup tools read and write.
//!
//! Key names and file formats are facts; nothing here is copied from the
//! gserver fork's code (`AGENTS.md`). Every writer must keep comments, key
//! order and unknown keys: the owner's file is edited line by line, never
//! regenerated from a template.

pub mod backup;
pub mod listserver_toml;
pub mod options_txt;
