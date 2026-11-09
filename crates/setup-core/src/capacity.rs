//! Gserver player limits. Config names and keys are wire-independent facts:
//! the fork uses `maxplayers` in `serveroptions.txt`, Rust uses
//! `max_players` in `config/server.txt`. Neither is the listserver's TOML.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::backup;
use crate::config::options_txt::{Line, OptionsTxt, CHANGE_COMMENT};
use crate::stamp::Stamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Fork,
    Rust,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Fork => "maxplayers",
            Self::Rust => "max_players",
        }
    }

    fn matches(self, key: &str) -> bool {
        match self {
            Self::Fork => key.eq_ignore_ascii_case(self.key()),
            Self::Rust => key == self.key(),
        }
    }

    pub fn path(self, world: &Path, server: Option<&str>) -> Result<PathBuf, String> {
        match self {
            Self::Fork => {
                let name = server.unwrap_or("default");
                if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':'])
                {
                    return Err("bad server folder name".into());
                }
                Ok(world
                    .join("servers")
                    .join(name)
                    .join("config/serveroptions.txt"))
            }
            Self::Rust if server.is_some() => {
                Err("Rust capacity uses --world, not a SERVER name".into())
            }
            Self::Rust => Ok(world.join("config/server.txt")),
        }
    }
}

/// An existing config and the proposed edit. Save checks that the original
/// bytes still match, then uses the common backup and atomic-write helper.
pub struct Edit {
    pub path: PathBuf,
    pub key: &'static str,
    pub before: Option<String>,
    pub after: String,
    original: String,
    rendered: String,
}

impl Edit {
    pub fn preview(path: PathBuf, kind: Kind, players: u32) -> Result<Self, String> {
        if players == 0 {
            return Err("max players must be at least 1".into());
        }
        // Strict UTF-8 avoids silently replacing any bytes in the owner's file.
        let original = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut doc = OptionsTxt::parse(&original);
        let found = doc
            .lines
            .iter()
            .rposition(|line| matches!(line, Line::Setting { key, .. } if kind.matches(key)));
        let before = found.and_then(|i| match &doc.lines[i] {
            Line::Setting { value, .. } => {
                Some(value.split('#').next().unwrap_or("").trim().to_string())
            }
            _ => None,
        });
        let after = players.to_string();
        if before.as_deref() != Some(after.as_str()) {
            let comment = format!("{CHANGE_COMMENT}, {}", Stamp::now().date());
            if let Some(i) = found {
                if let Line::Setting { value, raw, .. } = &mut doc.lines[i] {
                    let eq = raw.find('=').expect("setting has equals");
                    let tail = &raw[eq + 1..];
                    let pad = &tail[..tail.len() - tail.trim_start().len()];
                    // Keep the inline comment, its preceding whitespace and all
                    // unrelated lines. Rust's config reader strips # comments.
                    let suffix = tail
                        .find('#')
                        .map(|n| {
                            let start = tail[..n].trim_end().len();
                            &tail[start..]
                        })
                        .unwrap_or("");
                    *raw = format!("{}={pad}{after}{suffix}", &raw[..eq]);
                    *value = after.clone();
                }
                match i.checked_sub(1).and_then(|j| doc.lines.get_mut(j)) {
                    Some(Line::Other(c)) if c.starts_with(CHANGE_COMMENT) => *c = comment,
                    _ => doc.lines.insert(i, Line::Other(comment)),
                }
            } else {
                // Append the exact Rust key; an uppercase spelling is an
                // unrelated unknown setting, not an alias as in the fork.
                doc.trailing_newline = true;
                doc.lines.push(Line::Other(comment));
                doc.lines.push(Line::Setting {
                    key: kind.key().into(),
                    value: after.clone(),
                    raw: format!("{} = {after}", kind.key()),
                });
            }
        }
        let rendered = if before.as_deref() == Some(after.as_str()) {
            original.clone()
        } else {
            doc.render()
        };
        Ok(Self {
            path,
            key: kind.key(),
            before,
            after,
            original,
            rendered,
        })
    }

    pub fn changed(&self) -> bool {
        self.original != self.rendered
    }

    pub fn save(&self) -> Result<Option<PathBuf>, String> {
        if fs::read_to_string(&self.path).map_err(|e| e.to_string())? != self.original {
            return Err("config changed since preview; review it again".into());
        }
        if !self.changed() {
            return Ok(None);
        }
        backup::save(&self.path, self.rendered.as_bytes(), Stamp::now()).map_err(|e| e.to_string())
    }
}

pub fn undo(path: &Path) -> Result<PathBuf, String> {
    backup::undo(path, Stamp::now()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_select_different_paths_and_keys() {
        let w = Path::new("world");
        assert_eq!(
            Kind::Fork.path(w, None).unwrap(),
            w.join("servers/default/config/serveroptions.txt")
        );
        assert_eq!(
            Kind::Rust.path(w, None).unwrap(),
            w.join("config/server.txt")
        );
        assert!(Kind::Rust.path(w, Some("default")).is_err());
        assert!(Kind::Fork.path(w, Some("../default")).is_err());
    }

    #[test]
    fn fork_changes_last_case_insensitive_key_and_undo_restores_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("serveroptions.txt");
        let original = "# owner's config\r\nmaxplayers=12\r\nunknown = keep\r\n MaxPlayers = 128  # operator note";
        fs::write(&path, original).unwrap();
        let edit = Edit::preview(path.clone(), Kind::Fork, 1200).unwrap();
        assert_eq!(edit.before.as_deref(), Some("128"));
        // Preview does not write, and only the last occurrence counts.
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        let backup = edit.save().unwrap().unwrap();
        assert_eq!(fs::read_to_string(&backup).unwrap(), original);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("maxplayers=12\r\nunknown = keep\r\n"));
        assert!(text.ends_with(" MaxPlayers = 1200  # operator note"));
        assert!(!text.replace("\r\n", "").contains('\n'));
        assert!(!Edit::preview(path.clone(), Kind::Fork, 1200)
            .unwrap()
            .changed());
        undo(&path).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }

    #[test]
    fn rust_uses_exact_keys_preserves_inline_comments_and_unknown_case() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.txt");
        let original =
            "MAX_PLAYERS = 42\nmax_players = 64\nmax_players\t=\t256 # note\nmaxplayers = 17\n";
        fs::write(&path, original).unwrap();
        let edit = Edit::preview(path.clone(), Kind::Rust, 1000).unwrap();
        edit.save().unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("MAX_PLAYERS = 42\nmax_players = 64\n"));
        assert!(text.ends_with("max_players\t=\t1000 # note\nmaxplayers = 17\n"));
        let before = backup::backups(&path).unwrap();
        Edit::preview(path.clone(), Kind::Rust, 1000)
            .unwrap()
            .save()
            .unwrap();
        assert_eq!(backup::backups(&path).unwrap(), before);
        fs::write(&path, "MAX_PLAYERS = 99\n").unwrap();
        Edit::preview(path.clone(), Kind::Rust, 1200)
            .unwrap()
            .save()
            .unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.starts_with("MAX_PLAYERS = 99\n"));
        assert!(text.ends_with("max_players = 1200\n"));
    }

    #[test]
    fn rejects_zero_and_stale_previews_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.txt");
        fs::write(&path, "max_players = 256\n").unwrap();
        assert!(Edit::preview(path.clone(), Kind::Rust, 0).is_err());
        let edit = Edit::preview(path.clone(), Kind::Rust, 1000).unwrap();
        fs::write(&path, "max_players = 500\n").unwrap();
        assert!(edit.save().is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "max_players = 500\n");
        assert!(backup::backups(&path).unwrap().is_empty());
    }
}
