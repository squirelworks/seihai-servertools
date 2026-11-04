//! `seihai-listserver.toml`: a quick line scan for the lab start
//! ([`ListserverLab`]) and a format-preserving editor for the Host a
//! listserver wizard ([`ListserverToml`], on `toml_edit`), which keeps
//! comments, order and unknown keys.
//!
//! The keys and their defaults are documented in
//! `seihai-listserver.sample.toml` in `squirelworks/seihai-listserver`.

use std::fmt;

use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Value};

/// The keys the host wizard manages, in the order it writes them.
pub const MANAGED_KEYS: &[&str] = &[
    "bind",
    "gserver_port",
    "client_ports",
    "accounts",
    "gserver_ip",
    "gservers",
    "max_connections",
    "silent_list_after_ms",
    "site_url",
    "upgrade_url",
];

/// The listserver's own `max_connections` default.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 256;

/// What Host a listserver suggests for `silent_list_after_ms`. Real 1.41
/// rev 1 connects and waits for the list without sending a byte; with the
/// listserver's default (0, off) it never gets one. A client that speaks
/// first is never answered early, so this does not change other clients.
pub const SUGGESTED_SILENT_LIST_MS: u64 = 500;

/// One row of the listserver's `gservers` allow-list: an address that may
/// register, and the HQ password (`SERVERHQPASS`) its gserver must send
/// before it is listed. The gserver side keeps the same password as
/// `hq_password` in its `config/adminconfig.txt`.
///
/// The listserver reads the password as plain text, so it is stored that
/// way; [`fmt::Debug`] and the review diff never show it.
#[derive(Clone, PartialEq, Eq)]
pub struct AllowedGserver {
    /// Address the gserver's link comes from, such as `127.0.0.1`.
    pub ip: String,
    pub hq_password: String,
}

impl fmt::Debug for AllowedGserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AllowedGserver")
            .field("ip", &self.ip)
            .field(
                "hq_password",
                &if self.hq_password.is_empty() {
                    "(empty)"
                } else {
                    "(set)"
                },
            )
            .finish()
    }
}

/// The allow-list as the review shows it: addresses only, never passwords.
pub fn shown_gservers(list: &[AllowedGserver]) -> String {
    if list.is_empty() {
        return "[] (any gserver may register)".into();
    }
    list.iter()
        .map(|g| {
            if g.hq_password.is_empty() {
                format!("{} (no HQ password)", g.ip)
            } else {
                format!("{} (HQ password set)", g.ip)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Read `gservers` from a parsed file: an inline array of tables or
/// `[[gservers]]` tables. Rows without an `ip` are skipped.
fn read_gservers(doc: &DocumentMut) -> Vec<AllowedGserver> {
    let row = |ip: Option<&str>, pw: Option<&str>| {
        ip.map(|ip| AllowedGserver {
            ip: ip.to_string(),
            hq_password: pw.unwrap_or("").to_string(),
        })
    };
    match doc.get("gservers") {
        Some(Item::Value(Value::Array(a))) => a
            .iter()
            .filter_map(|v| v.as_inline_table())
            .filter_map(|t| {
                row(
                    t.get("ip").and_then(Value::as_str),
                    t.get("hq_password").and_then(Value::as_str),
                )
            })
            .collect(),
        Some(Item::ArrayOfTables(a)) => a
            .iter()
            .filter_map(|t| {
                row(
                    t.get("ip").and_then(Item::as_str),
                    t.get("hq_password").and_then(Item::as_str),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

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
    /// `gservers`: who may register. Empty: any gserver that reaches the
    /// gserver port.
    pub gservers: Vec<AllowedGserver>,
    /// `max_connections`, or [`DEFAULT_MAX_CONNECTIONS`] when missing.
    pub max_connections: u32,
    /// `silent_list_after_ms`: 0 (the default) leaves real 1.41 rev 1
    /// without a list.
    pub silent_list_after_ms: u64,
}

impl Default for ListserverLab {
    fn default() -> Self {
        ListserverLab {
            bind: "127.0.0.1".into(),
            gserver_port: DEFAULT_GSERVER_PORT,
            accounts: String::new(),
            gserver_ip: "peer".into(),
            gservers: Vec::new(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            silent_list_after_ms: 0,
        }
    }
}

impl ListserverLab {
    /// Read `bind` and `gserver_port` from TOML text. Only top-level
    /// `key = value` lines are looked at; tables and comments are skipped.
    /// `gservers` needs the whole file, so it stays empty when the file is
    /// not valid TOML.
    pub fn scan(text: &str) -> Self {
        let mut out = ListserverLab::default();
        if let Ok(doc) = text.parse::<DocumentMut>() {
            out.gservers = read_gservers(&doc);
        }
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
                "max_connections" => {
                    if let Ok(n) = value.parse() {
                        out.max_connections = n;
                    }
                }
                "silent_list_after_ms" => {
                    if let Ok(n) = value.parse() {
                        out.silent_list_after_ms = n;
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

    /// The `gservers` allow-list (passwords included).
    pub fn gservers(&self) -> Vec<AllowedGserver> {
        read_gservers(&self.doc)
    }

    /// A managed key as the review shows it: `gservers` without passwords.
    fn shown(&self, key: &str) -> Option<String> {
        if key == "gservers" {
            return self
                .doc
                .contains_key(key)
                .then(|| shown_gservers(&self.gservers()));
        }
        self.get(key)
    }

    /// Set the `gservers` allow-list, written as one inline array. Returns
    /// false when it already held exactly these rows.
    pub fn set_gservers(&mut self, list: &[AllowedGserver], date: &str) -> bool {
        if self.doc.contains_key("gservers") && self.gservers() == list {
            return false;
        }
        let mut a = Array::new();
        for g in list {
            let mut t = InlineTable::new();
            t.insert("ip", g.ip.as_str().into());
            t.insert("hq_password", g.hq_password.as_str().into());
            a.push(t);
        }
        // `[[gservers]]` tables would clash with a plain key of that name.
        if matches!(self.doc.get("gservers"), Some(Item::ArrayOfTables(_))) {
            self.doc.remove("gservers");
        }
        // Never shown; `set_item` only compares it with `get`.
        self.set_item("gservers", value(a), "\0", date)
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

    /// Set a whole-number key that is not a port.
    pub fn set_int(&mut self, key: &str, n: u64, date: &str) -> bool {
        let v = i64::try_from(n).unwrap_or(i64::MAX);
        self.set_item(key, value(v), &v.to_string(), date)
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
                let a = self.shown(k);
                let b = after.shown(k);
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
    /// Who may register. Empty: any gserver. Written when not empty, or
    /// when the file already has the key (so emptying it opens the list).
    pub gservers: Vec<AllowedGserver>,
    /// Most links open at once, clients and gservers together. Written when
    /// it is not the listserver's default or the key is already there.
    pub max_connections: u32,
    /// Milliseconds before a client that sent nothing gets the list (real
    /// 1.41 rev 1). Written when not 0 or the key is already there.
    pub silent_list_after_ms: u64,
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
            gservers: Vec::new(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            silent_list_after_ms: SUGGESTED_SILENT_LIST_MS,
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
    if !a.gservers.is_empty() || t.doc.contains_key("gservers") {
        note("gservers", t.set_gservers(&a.gservers, date));
    }
    if a.max_connections != DEFAULT_MAX_CONNECTIONS || t.doc.contains_key("max_connections") {
        note(
            "max_connections",
            t.set_int("max_connections", a.max_connections.into(), date),
        );
    }
    if a.silent_list_after_ms != 0 || t.doc.contains_key("silent_list_after_ms") {
        note(
            "silent_list_after_ms",
            t.set_int("silent_list_after_ms", a.silent_list_after_ms, date),
        );
    }
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
        assert_eq!(
            changed,
            [
                "bind",
                "gserver_port",
                "accounts",
                "gserver_ip",
                "silent_list_after_ms"
            ]
        );
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
            5,
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
        assert_eq!(before.diff(&again).len(), 6);
    }

    #[test]
    fn same_ports_are_no_change() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        assert!(!t.set_ports("client_ports", &DEFAULT_CLIENT_PORTS, "2026-10-31"));
        assert_eq!(t.render(), SAMPLE);
    }

    fn allow(ip: &str, pw: &str) -> AllowedGserver {
        AllowedGserver {
            ip: ip.into(),
            hq_password: pw.into(),
        }
    }

    #[test]
    fn allow_list_is_written_read_back_and_never_shown() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        let before = t.clone();
        let answers = HostAnswers {
            gservers: vec![allow("127.0.0.1", "s3cret"), allow("::1", "")],
            ..HostAnswers::default()
        };
        let changed = apply_host(&mut t, &answers, "2026-11-04");
        assert!(changed.contains(&"gservers"), "{changed:?}");
        let text = t.render();
        assert!(
            text.contains("# Changed by Seihai server setup, 2026-11-04\ngservers = [{ ip = \"127.0.0.1\", hq_password = \"s3cret\" }, { ip = \"::1\", hq_password = \"\" }]\n"),
            "{text}"
        );
        let again = ListserverToml::parse(&text).unwrap();
        assert_eq!(again.gservers(), answers.gservers);
        assert_eq!(again.settings().gservers, answers.gservers);

        let diff = before.diff(&again);
        let row = diff.iter().find(|(k, _, _)| k == "gservers").unwrap();
        assert_eq!(row.1, None);
        assert_eq!(
            row.2.as_deref(),
            Some("127.0.0.1 (HQ password set), ::1 (no HQ password)")
        );
        assert!(!format!("{diff:?}{:?}", again.settings()).contains("s3cret"));

        // Same rows again: no change. An empty list then opens it up.
        let mut t = again;
        assert!(!t.set_gservers(&answers.gservers, "2026-11-05"));
        let open = HostAnswers::default();
        assert!(apply_host(&mut t, &open, "2026-11-05").contains(&"gservers"));
        assert!(t.render().contains("gservers = []"), "{}", t.render());
        assert_eq!(
            ListserverToml::parse(&t.render())
                .unwrap()
                .shown("gservers")
                .as_deref(),
            Some("[] (any gserver may register)")
        );
    }

    #[test]
    fn no_allow_list_is_not_written_when_none_is_asked_for() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        apply_host(&mut t, &HostAnswers::default(), "2026-11-04");
        assert!(!t.render().contains("gservers"), "{}", t.render());
    }

    #[test]
    fn reads_and_replaces_array_of_tables() {
        let text = "bind = \"0.0.0.0\"\n\n[[gservers]]\nip = \"10.0.0.2\"\nhq_password = \"pw\"\n";
        let mut t = ListserverToml::parse(text).unwrap();
        assert_eq!(t.gservers(), [allow("10.0.0.2", "pw")]);
        assert_eq!(
            ListserverLab::scan(text).gservers,
            [allow("10.0.0.2", "pw")]
        );
        assert!(t.set_gservers(&[allow("127.0.0.1", "pw")], "2026-11-04"));
        let out = t.render();
        assert!(!out.contains("[[gservers]]"), "{out}");
        assert_eq!(
            ListserverToml::parse(&out).unwrap().gservers(),
            [allow("127.0.0.1", "pw")]
        );
    }

    #[test]
    fn silent_list_and_max_connections() {
        let mut t = ListserverToml::parse(SAMPLE).unwrap();
        let a = HostAnswers {
            max_connections: 1200,
            ..HostAnswers::default()
        };
        apply_host(&mut t, &a, "2026-11-04");
        let text = t.render();
        assert!(text.contains("max_connections = 1200\n"), "{text}");
        assert!(text.contains("silent_list_after_ms = 500\n"), "{text}");
        let s = ListserverLab::scan(&text);
        assert_eq!((s.max_connections, s.silent_list_after_ms), (1200, 500));

        // Back to the listserver defaults: the keys are there, so they are
        // written; nothing new is added to a file that never had them.
        let off = HostAnswers {
            silent_list_after_ms: 0,
            ..HostAnswers::default()
        };
        let changed = apply_host(&mut t, &off, "2026-11-05");
        assert_eq!(changed, ["max_connections", "silent_list_after_ms"]);
        assert_eq!(
            (
                t.settings().max_connections,
                t.settings().silent_list_after_ms
            ),
            (DEFAULT_MAX_CONNECTIONS, 0)
        );
        let mut fresh = ListserverToml::parse(SAMPLE).unwrap();
        apply_host(&mut fresh, &off, "2026-11-05");
        let text = fresh.render();
        assert!(
            !text.contains("max_connections") && !text.contains("silent_list"),
            "{text}"
        );
        assert_eq!(ListserverLab::default().max_connections, 256);
    }

    #[test]
    fn stops_at_the_first_table() {
        let lab = ListserverLab::scan("[other]\ngserver_port = 1\n");
        assert_eq!(lab.gserver_port, DEFAULT_GSERVER_PORT);
    }
}
