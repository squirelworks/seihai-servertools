//! `seihai-listserver.toml`. **Read-only bits for the lab start today**; the
//! editor for the "Host a listserver" wizard comes after the listserver's
//! public-mode settings exist (`docs/NEXT.md`, setup step 6).
//!
//! The keys and their defaults are documented in
//! `seihai-listserver.sample.toml` in `squirelworks/seihai-listserver`. The
//! editor must keep comments and unknown keys (use a format-preserving TOML
//! crate such as `toml_edit`, MIT/Apache).

/// Default port gservers register on.
pub const DEFAULT_GSERVER_PORT: u16 = 14900;

/// The two settings the lab start needs, read with a plain line scan so a
/// half-edited file still starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListserverLab {
    /// `bind`, or `127.0.0.1` when missing.
    pub bind: String,
    /// `gserver_port`, or [`DEFAULT_GSERVER_PORT`] when missing or not a number.
    pub gserver_port: u16,
}

impl Default for ListserverLab {
    fn default() -> Self {
        ListserverLab {
            bind: "127.0.0.1".into(),
            gserver_port: DEFAULT_GSERVER_PORT,
        }
    }
}

impl ListserverLab {
    /// Read `bind` and `gserver_port` from TOML text. Only top-level
    /// `key = value` lines are looked at; tables and comments are skipped.
    pub fn scan(text: &str) -> Self {
        let mut out = ListserverLab::default();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.split('#').next().unwrap_or("").trim();
            match key.trim() {
                "bind" => out.bind = value.trim_matches('"').to_string(),
                "gserver_port" => {
                    if let Ok(port) = value.parse() {
                        out.gserver_port = port;
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// True when the listserver only listens on this machine.
    pub fn is_loopback(&self) -> bool {
        self.bind == "127.0.0.1" || self.bind == "::1" || self.bind == "localhost"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_empty() {
        assert_eq!(ListserverLab::scan(""), ListserverLab::default());
    }

    #[test]
    fn reads_bind_and_port_and_skips_comments() {
        let text = "# lab\nbind = \"0.0.0.0\" # every interface\ngserver_port = 15000\n";
        let lab = ListserverLab::scan(text);
        assert_eq!(lab.bind, "0.0.0.0");
        assert_eq!(lab.gserver_port, 15000);
        assert!(!lab.is_loopback());
    }

    #[test]
    fn stops_at_the_first_table() {
        let lab = ListserverLab::scan("[other]\ngserver_port = 1\n");
        assert_eq!(lab.gserver_port, DEFAULT_GSERVER_PORT);
    }
}
