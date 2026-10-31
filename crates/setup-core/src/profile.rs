//! The three setups a server folder can be in, and which keys each one
//! sets, as data. The tables come from `docs/design/go-public.md`; key
//! names are facts about the gserver's config format.
//!
//! **Today:** the tables. **Setup step 2** (`docs/NEXT.md`) adds detecting
//! which profile a folder is in and "Back to lab".

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

#[cfg(test)]
mod tests {
    use super::*;

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
