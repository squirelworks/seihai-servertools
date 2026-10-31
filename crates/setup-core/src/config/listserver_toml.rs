//! `seihai-listserver.toml`: a quick line scan for the lab start
//! ([`ListserverLab`]) and a format-preserving editor for the Host a
//! listserver wizard ([`ListserverToml`], on `toml_edit`), which keeps
//! comments, order and unknown keys.
//!
//! The keys and their defaults are documented in
//! `seihai-listserver.sample.toml` in `squirelworks/seihai-listserver`.

use toml_edit::{value, Array, DocumentMut, Item};

/// The keys the host wizard manages, in the order it writes them.
pub const MANAGED_KEYS: &[&str] = &[
    "bind",
    "gserver_port",
    "client_ports",
    "accounts",
    "gserver_ip",
    "site_url",
    "upgrade_url",
];

/// Default client ports.
pub const DEFAULT_CLIENT_PORTS: [u16; 3] = [14921, 14922, 14923];

/// Default port gservers register on.
pub const DEFAULT_GSERVER_PORT: u16 = 14900;

/// The settings the lab start and the safety check need, read with a plain line scan so a
/// half-edited file still starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListserverLab {
    /// `bind`, or `127.0.0.1` when missing.
    pub bind: String,
    /// `gserver_port`, or [`DEFAULT_GSERVER_PORT`] when missing or not a number.
    pub gserver_port: u16,
    /// `accounts`: empty means open mode (every account gets in).
    pub accounts: String,
    /// `gserver_ip`: `peer` (the default) or `sent`.
    pub gserver_ip: String,
}

impl Default for ListserverLab {
    fn default() -> Self {
        ListserverLab {
            bind: "127.0.0.1".into(),
            gserver_port: DEFAULT_GSERVER_PORT,
            accounts: String::new(),
            gserver_ip: "peer".into(),
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
                "accounts" => out.accounts = value.trim_matches('"').to_string(),
                "gserver_ip" => out.gserver_ip = value.trim_matches('"').to_string(),
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

/// A `seihai-listserver.toml` being edited.
#[derive(Debug, Clone)]
pub struct ListserverToml {
    doc: DocumentMut,
}

impl ListserverToml {
    /// Parse the file's text (empty text is an empty file: every default).
    pub fn parse(text: &str) -> Result<Self, String> {
        text.parse::<DocumentMut>()
            .map(|doc| ListserverToml { doc })
            .map_err(|e| format!("not valid TOML: {e}"))
    }

    /// The text again; unchanged when nothing was set.
    pub fn render(&self) -> String {
        self.doc.to_string()
    }

    /// A top-level value shown as text (`"x"` without quotes, arrays as
    /// `[a, b]`), or `None` when the key is missing.
    pub fn get(&self, key: &str) -> Option<String> {
        let v = self.doc.get(key)?.as_value()?;
        Some(match v {
            toml_edit::Value::String(s) => s.value().clone(),
            other => other.to_string().trim().to_string(),
        })
    }

    /// The settings as the listserver will read them.
    pub fn settings(&self) -> ListserverLab {
        ListserverLab::scan(&self.render())
    }

    fn set_item(&mut self, key: &str, item: Item, shown: &str, date: &str) -> bool {
        if self.get(key).as_deref() == Some(shown) {
            return false;
        }
        let existed = self.doc.contains_key(key);
        let first = self.doc.is_empty();
        self.doc[key] = item;
        if let Some(mut k) = self.doc.as_table_mut().key_mut(key) {
            let prefix = k
                .leaf_decor()
                .prefix()
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            let comment = format!("# Changed by Seihai server setup, {date}\n");
            let kept: String = prefix
                .lines()
                .filter(|l| !l.starts_with("# Changed by Seihai server setup"))
                .map(|l| format!("{l}\n"))
                .collect();
            let lead = if existed || first { "" } else { "\n" };
            k.leaf_decor_mut()
                .set_prefix(format!("{lead}{kept}{comment}"));
        }
        true
    }

    /// Set a string key. Returns false when it already had that value.
    pub fn set_str(&mut self, key: &str, v: &str, date: &str) -> bool {
        self.set_item(key, value(v), v, date)
    }

    /// Set a whole-number key.
    pub fn set_port(&mut self, key: &str, port: u16, date: &str) -> bool {
        self.set_item(key, value(i64::from(port)), &port.to_string(), date)
    }

    /// Set a list of ports.
    pub fn set_ports(&mut self, key: &str, ports: &[u16], date: &str) -> bool {
        let mut a = Array::new();
        for p in ports {
            a.push(i64::from(*p));
        }
        let shown = a.to_string();
        let current = self.get(key).map(|s| s.replace(' ', ""));
        if current.as_deref() == Some(shown.replace(' ', "").as_str()) {
            return false;
        }
        self.set_item(key, value(a), &shown, date)
    }

    /// `(key, before, after)` for every managed key that differs.
    pub fn diff(&self, after: &ListserverToml) -> Vec<(String, Option<String>, Option<String>)> {
        MANAGED_KEYS
            .iter()
            .filter_map(|k| {
                let a = self.get(k);
                let b = after.get(k);
                (a != b).then(|| (k.to_string(), a, b))
            })
            .collect()
    }
}

/// The owner's answers in the Host a listserver wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAnswers {
    /// `0.0.0.0`, `::` or one interface's address.
    pub bind: String,
    pub gserver_port: u16,
    pub client_ports: Vec<u16>,
    /// Accounts file, relative to the TOML's folder. Required once public.
    pub accounts: String,
    /// `peer` or `sent`.
    pub gserver_ip: String,
    pub site_url: String,
    pub upgrade_url: String,
}

impl Default for HostAnswers {
    fn default() -> Self {
        HostAnswers {
            bind: "0.0.0.0".into(),
            gserver_port: DEFAULT_GSERVER_PORT,
            client_ports: DEFAULT_CLIENT_PORTS.to_vec(),
            accounts: "accounts.txt".into(),
            gserver_ip: "sent".into(),
            site_url: String::new(),
            upgrade_url: String::new(),
        }
    }
}

/// Write the host answers. Empty URLs are only written when the key exists.
pub fn apply_host(t: &mut ListserverToml, a: &HostAnswers, date: &str) -> Vec<&'static str> {
    let mut changed = Vec::new();
    let mut note = |k: &'static str, c: bool| {
        if c {
            changed.push(k)
        }
    };
    note("bind", t.set_str("bind", &a.bind, date));
    note(
        "gserver_port",
        t.set_port("gserver_port", a.gserver_port, date),
    );
    note(
        "client_ports",
        t.set_ports("client_ports", &a.client_ports, date),
    );
    note("accounts", t.set_str("accounts", &a.accounts, date));
    note("gserver_ip", t.set_str("gserver_ip", &a.gserver_ip, date));
    for (k, v) in [("site_url", &a.site_url), ("upgrade_url", &a.upgrade_url)] {
        if !(v.is_empty() && t.get(k).is_none()) {
            note(k, t.set_str(k, v, date));
        }
    }
    changed
}

/// Back to lab for the listserver: listen on `127.0.0.1` only. Accounts,
/// ports and URLs are left alone.
pub fn listserver_back_to_lab(t: &mut ListserverToml, date: &str) -> bool {
    t.set_str("bind", "127.0.0.1", date)
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

    const SAMPLE: &str = "# seihai-listserver settings\n\n# Address to listen on.\nbind = \"127.0.0.1\"\n\n# Ports.\nclient_ports = [14921, 14922, 14923]\nlist_name = \"Lab listserver\"\naccounts = \"\"\n";

    #[test]
    fn untouched_file_round_trips() {
        let t = ListserverToml::parse(SAMPLE).unwrap();
        assert_eq!(t.render(), SAMPLE);
        assert_eq!(t.get("bind").as_deref(), Some("127.0.0.1"));
        assert_eq!(
            t.get("client_ports").as_deref(),
            Some("[14921, 14922, 14923]")
        );
    }

    #[test]
    fn host_then_back_to_lab_keeps_comments_and_unknown_keys() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        let changed = apply_host(&mut t, &HostAnswers::default(), "2026-10-31");
        assert_eq!(changed, ["bind", "gserver_port", "accounts", "gserver_ip"]);
        let text = t.render();
        assert!(text.contains("# Address to listen on.\n# Changed by Seihai server setup, 2026-10-31\nbind = \"0.0.0.0\"\n"), "{text}");
        assert!(text.contains("list_name = \"Lab listserver\""), "{text}");
        assert!(!text.contains("site_url"), "empty URLs not added: {text}");
        let s = t.settings();
        assert_eq!(
            (s.bind.as_str(), s.accounts.as_str(), s.gserver_ip.as_str()),
            ("0.0.0.0", "accounts.txt", "sent")
        );
        assert!(ListserverToml::parse(&text).is_ok());

        assert!(listserver_back_to_lab(&mut t, "2026-11-01"));
        let text = t.render();
        assert_eq!(
            text.matches("# Changed by Seihai server setup").count(),
            4,
            "{text}"
        );
        assert!(text.contains("2026-11-01\nbind = \"127.0.0.1\""), "{text}");
        assert!(t.settings().is_loopback());
    }

    #[test]
    fn empty_file_gets_the_keys() {
        let mut t = ListserverToml::parse("").unwrap();
        apply_host(&mut t, &HostAnswers::default(), "2026-10-31");
        let again = ListserverToml::parse(&t.render()).unwrap();
        assert_eq!(
            again.get("client_ports").as_deref(),
            Some("[14921, 14922, 14923]")
        );
        assert_eq!(again.get("gserver_port").as_deref(), Some("14900"));
        let before = ListserverToml::parse("").unwrap();
        assert_eq!(before.diff(&again).len(), 5);
    }

    #[test]
    fn same_ports_are_no_change() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        assert!(!t.set_ports("client_ports", &DEFAULT_CLIENT_PORTS, "2026-10-31"));
        assert_eq!(t.render(), SAMPLE);
    }

    #[test]
    fn stops_at_the_first_table() {
        let lab = ListserverLab::scan("[other]\ngserver_port = 1\n");
        assert_eq!(lab.gserver_port, DEFAULT_GSERVER_PORT);
    }
}
