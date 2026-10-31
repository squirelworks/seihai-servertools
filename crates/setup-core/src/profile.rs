//! The three setups a server folder can be in, and which keys each one
//! sets, as data. The tables come from `docs/design/go-public.md`; key
//! names are facts about the gserver's config format.
//!
//! Besides the tables: [`detect`] says what a server folder is set up for,
//! [`back_to_lab`] restores the lab values, and [`apply_public`] writes the
//! owner's answers from the Join a listserver wizard. They only change an
//! [`OptionsTxt`] in memory; saving (with a backup) is the caller's job, after
//! [`crate::checks::check_gserver`] allows it.

use crate::config::options_txt::OptionsTxt;

/// What a server folder is set up for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Loopback only, logins not checked: the lab default.
    Lab,
    /// Listens outside, registers with a listserver the owner names.
    Public,
    /// This machine also hosts a listserver for other machines.
    HostListserver,
}

/// Where a public value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicValue {
    /// A fixed value.
    Fixed(&'static str),
    /// The owner types or picks it in the wizard.
    Owner,
    /// Left as it is.
    Unchanged,
}

/// One gserver key the wizard manages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyRule {
    pub key: &'static str,
    /// The lab value, restored by "Back to lab". `None`: Back to lab leaves it.
    pub lab: Option<&'static str>,
    pub public: PublicValue,
    /// The wizard step that changes it.
    pub step: &'static str,
}

/// The gserver (`serveroptions.txt`) keys, in the order "Back to lab" writes
/// them. `localhost` comes last on purpose: it is only turned back on after
/// `serverinterface` is loopback again.
pub const GSERVER_KEYS: &[KeyRule] = &[
    KeyRule {
        key: "serverinterface",
        lab: Some("127.0.0.1"),
        public: PublicValue::Owner,
        step: "Network",
    },
    KeyRule {
        key: "serverip",
        lab: Some("127.0.0.1"),
        public: PublicValue::Owner,
        step: "Network",
    },
    KeyRule {
        key: "localip",
        lab: Some("127.0.0.1"),
        public: PublicValue::Fixed("AUTO"),
        step: "Network",
    },
    KeyRule {
        key: "serverport",
        lab: None,
        public: PublicValue::Unchanged,
        step: "Network",
    },
    KeyRule {
        key: "listip",
        lab: Some("127.0.0.1"),
        public: PublicValue::Owner,
        step: "Listserver",
    },
    KeyRule {
        key: "listport",
        lab: Some("14900"),
        public: PublicValue::Owner,
        step: "Listserver",
    },
    KeyRule {
        key: "upnp",
        lab: Some("false"),
        public: PublicValue::Owner,
        step: "Router",
    },
    KeyRule {
        key: "name",
        lab: None,
        public: PublicValue::Owner,
        step: "Server details",
    },
    KeyRule {
        key: "description",
        lab: None,
        public: PublicValue::Owner,
        step: "Server details",
    },
    KeyRule {
        key: "url",
        lab: None,
        public: PublicValue::Owner,
        step: "Server details",
    },
    KeyRule {
        key: "staff",
        lab: None,
        public: PublicValue::Owner,
        step: "Safety check",
    },
    KeyRule {
        key: "localhost",
        lab: Some("true"),
        public: PublicValue::Fixed("false"),
        step: "Listserver",
    },
];

/// The listserver (`seihai-listserver.toml`) keys the host wizard manages.
pub const LISTSERVER_KEYS: &[KeyRule] = &[
    KeyRule {
        key: "bind",
        lab: Some("127.0.0.1"),
        public: PublicValue::Owner,
        step: "Network",
    },
    KeyRule {
        key: "gserver_port",
        lab: Some("14900"),
        public: PublicValue::Unchanged,
        step: "Network",
    },
    KeyRule {
        key: "client_ports",
        lab: Some("[14921, 14922, 14923]"),
        public: PublicValue::Unchanged,
        step: "Network",
    },
    KeyRule {
        key: "accounts",
        lab: Some(""),
        public: PublicValue::Owner,
        step: "Safety check",
    },
    KeyRule {
        key: "gserver_ip",
        lab: Some("peer"),
        public: PublicValue::Owner,
        step: "Addresses",
    },
    KeyRule {
        key: "site_url",
        lab: Some(""),
        public: PublicValue::Owner,
        step: "Site and upgrade URLs",
    },
    KeyRule {
        key: "upgrade_url",
        lab: Some(""),
        public: PublicValue::Owner,
        step: "Site and upgrade URLs",
    },
];

/// True for `true`, `1`, `yes`, `on` (any case).
pub fn is_true(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("true" | "1" | "yes" | "on")
    )
}

/// True when the address only reaches this machine.
pub fn is_loopback(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        Some(v) => v.starts_with("127.") || v == "::1" || v.eq_ignore_ascii_case("localhost"),
        None => false,
    }
}

/// What a gserver folder is set up for: [`Profile::Lab`] when it listens on
/// loopback only and does not ask the listserver about logins, else
/// [`Profile::Public`]. (Hosting a listserver is a property of the machine,
/// not of a gserver folder.)
pub fn detect(options: &OptionsTxt) -> Profile {
    if is_loopback(options.get("serverinterface")) && is_true(options.get("localhost")) {
        Profile::Lab
    } else {
        Profile::Public
    }
}

/// Write every lab value from [`GSERVER_KEYS`], in table order, so
/// `localhost = true` is written last, after `serverinterface` is loopback.
/// Returns the keys that changed.
pub fn back_to_lab(options: &mut OptionsTxt, date: &str) -> Vec<&'static str> {
    GSERVER_KEYS
        .iter()
        .filter_map(|k| k.lab.map(|v| (k.key, v)))
        .filter(|(key, value)| options.set(key, value, date))
        .map(|(key, _)| key)
        .collect()
}

/// The owner's answers in the Join a listserver wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicAnswers {
    /// "Listen on": `AUTO` or one of this machine's addresses.
    pub interface: String,
    /// "Address players use": `AUTO` or what the owner typed.
    pub server_ip: String,
    /// The listserver the owner named. There is no default.
    pub list_host: String,
    pub list_port: u16,
    pub upnp: bool,
    pub name: String,
    pub description: String,
    pub url: String,
    /// The owner's account, put in `staff` in place of `YOURACCOUNT`.
    pub staff_account: String,
}

/// Write the public values. `localhost = false` goes last, after the
/// listserver address is set. The `staff` line has `YOURACCOUNT` replaced
/// by the owner's account (or the account added when it is not listed).
/// Returns the keys that changed.
pub fn apply_public(options: &mut OptionsTxt, a: &PublicAnswers, date: &str) -> Vec<&'static str> {
    let staff = staff_with(options.get("staff").unwrap_or(""), &a.staff_account);
    let port = a.list_port.to_string();
    let values: [(&'static str, &str); 11] = [
        ("serverinterface", &a.interface),
        ("serverip", &a.server_ip),
        ("localip", "AUTO"),
        ("listip", &a.list_host),
        ("listport", &port),
        ("upnp", if a.upnp { "true" } else { "false" }),
        ("name", &a.name),
        ("description", &a.description),
        ("url", &a.url),
        ("staff", &staff),
        ("localhost", "false"),
    ];
    values
        .into_iter()
        // An empty optional text (description, url) is not added as a new key.
        .filter(|(key, value)| {
            !(value.is_empty() && options.get(key).is_none()) && options.set(key, value, date)
        })
        .map(|(key, _)| key)
        .collect()
}

/// The `staff` list with `YOURACCOUNT` replaced by `account`, or `account`
/// added when neither is in it. Entries like `(Manager)` are kept. An empty
/// `account` leaves the list as it is (so the safety check still blocks a
/// `YOURACCOUNT`).
pub fn staff_with(staff: &str, account: &str) -> String {
    let account = account.trim();
    if account.is_empty() {
        return staff.to_string();
    }
    let mut parts: Vec<String> = staff
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut found = false;
    for p in &mut parts {
        if p.eq_ignore_ascii_case("YOURACCOUNT") {
            *p = account.to_string();
            found = true;
        } else if p.eq_ignore_ascii_case(account) {
            found = true;
        }
    }
    if !found && !account.is_empty() {
        parts.push(account.to_string());
    }
    let mut seen = Vec::new();
    parts.retain(|p| {
        let new = !seen.iter().any(|s: &String| s.eq_ignore_ascii_case(p));
        seen.push(p.clone());
        new
    });
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAB: &str = "name = My Server\nserverinterface = 127.0.0.1\nserverip = 127.0.0.1\nlocalip = 127.0.0.1\nserverport = 14802\nlistip = 127.0.0.1\nlistport = 14900\nupnp = false\nstaff = (Manager),YOURACCOUNT\nlocalhost = true\n";

    fn answers() -> PublicAnswers {
        PublicAnswers {
            interface: "AUTO".into(),
            server_ip: "AUTO".into(),
            list_host: "<listserver-host>".into(),
            list_port: 14900,
            upnp: false,
            name: "Owner's World".into(),
            description: "A test world".into(),
            url: String::new(),
            staff_account: "owner".into(),
        }
    }

    #[test]
    fn empty_optional_texts_are_not_added() {
        let mut o = OptionsTxt::parse(LAB);
        apply_public(&mut o, &answers(), "2026-10-31");
        assert_eq!(o.get("url"), None);
        assert_eq!(o.get("description"), Some("A test world"));
    }

    #[test]
    fn detects_lab_and_public() {
        let lab = OptionsTxt::parse(LAB);
        assert_eq!(detect(&lab), Profile::Lab);
        let mut public = lab.clone();
        apply_public(&mut public, &answers(), "2026-10-31");
        assert_eq!(detect(&public), Profile::Public);
        // Loopback but logins checked by the listserver: not the lab profile.
        let mut half = lab.clone();
        half.set("localhost", "false", "2026-10-31");
        assert_eq!(detect(&half), Profile::Public);
    }

    #[test]
    fn apply_public_then_back_to_lab_restores_the_lab_values() {
        let mut o = OptionsTxt::parse(LAB);
        let changed = apply_public(&mut o, &answers(), "2026-10-31");
        assert_eq!(changed.last(), Some(&"localhost"));
        assert_eq!(o.get("serverinterface"), Some("AUTO"));
        assert_eq!(o.get("listip"), Some("<listserver-host>"));
        assert_eq!(o.get("staff"), Some("(Manager),owner"));
        assert_eq!(o.get("localhost"), Some("false"));

        let changed = back_to_lab(&mut o, "2026-10-31");
        assert_eq!(changed.last(), Some(&"localhost"));
        for k in GSERVER_KEYS {
            if let Some(v) = k.lab {
                assert_eq!(o.get(k.key), Some(v), "{}", k.key);
            }
        }
        assert_eq!(detect(&o), Profile::Lab);
        // The owner's name and staff are not lab-managed: they stay.
        assert_eq!(o.get("name"), Some("Owner's World"));
    }

    #[test]
    fn back_to_lab_on_a_lab_folder_changes_nothing() {
        let mut o = OptionsTxt::parse(LAB);
        assert!(back_to_lab(&mut o, "2026-10-31").is_empty());
        assert_eq!(o.render(), LAB);
    }

    #[test]
    fn staff_list_editing() {
        assert_eq!(
            staff_with("(Manager),YOURACCOUNT", "owner"),
            "(Manager),owner"
        );
        assert_eq!(staff_with("(Manager),owner", "owner"), "(Manager),owner");
        assert_eq!(staff_with("a, b", "owner"), "a,b,owner");
        assert_eq!(staff_with("", "owner"), "owner");
        assert_eq!(staff_with("YOURACCOUNT,owner", "owner"), "owner");
        assert_eq!(
            staff_with("(Manager),YOURACCOUNT", " "),
            "(Manager),YOURACCOUNT"
        );
    }

    #[test]
    fn localhost_is_written_last() {
        assert_eq!(GSERVER_KEYS.last().map(|k| k.key), Some("localhost"));
    }

    #[test]
    fn no_real_hosts_in_the_tables() {
        // Rule (AGENTS.md): only loopback or placeholders ship.
        for k in GSERVER_KEYS.iter().chain(LISTSERVER_KEYS) {
            for v in [
                k.lab,
                if let PublicValue::Fixed(v) = k.public {
                    Some(v)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                let looks_like_host = v.contains('.') && v.chars().any(|c| c.is_ascii_alphabetic());
                assert!(!looks_like_host, "{}: {v}", k.key);
                if v.chars().filter(|c| *c == '.').count() == 3 {
                    assert!(v.starts_with("127."), "{}: {v}", k.key);
                }
            }
        }
    }
}
