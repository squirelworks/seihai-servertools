//! The layout plan: which listservers, gservers, NPC-servers and staff
//! accounts a new server folder gets, read from and written to
//! `layout.toml`.

use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

/// The lab listserver's ports (seihai-listserver's defaults).
pub const LIST_GSERVER_PORT: u16 = 14900;
pub const LIST_CLIENT_PORTS: [u16; 3] = [14921, 14922, 14923];
/// The lab gserver port and NPC-server link port (seihai-gserver's defaults).
pub const GSERVER_PORT: u16 = 14802;
pub const NPC_PORT: u16 = 14810;
/// Port steps between the second, third ... server of a kind.
pub const GSERVER_STEP: u16 = 10;
pub const LIST_STEP: u16 = 50;

/// How a gserver's NPC-server runs (`npcserver` in its `config/server.txt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcMode {
    /// No NPC-server.
    Off,
    /// The gserver starts `seihai-npcserver` itself with a fresh token.
    Auto,
    /// The start script starts `seihai-npcserver`; the two share a token
    /// file written by the wizard.
    External,
}

impl NpcMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "no" => Ok(NpcMode::Off),
            "auto" | "yes" | "on" => Ok(NpcMode::Auto),
            "external" => Ok(NpcMode::External),
            o => Err(format!("npcserver: `{o}` is not off, auto or external")),
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            NpcMode::Off => "off",
            NpcMode::Auto => "auto",
            NpcMode::External => "external",
        }
    }
}

/// Which listserver a gserver registers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListRef {
    /// Not listed anywhere: players connect to its address directly.
    None,
    /// A listserver in this layout, by name.
    Local(String),
    /// A listserver run elsewhere: its gserver port, `HOST:PORT`.
    Remote(String),
}

impl ListRef {
    /// `""` or `none`: none; a name without `:`: a local listserver;
    /// `HOST:PORT`: a remote one.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("none") || s.eq_ignore_ascii_case("off") {
            ListRef::None
        } else if s.contains(':') {
            ListRef::Remote(s.to_string())
        } else {
            ListRef::Local(s.to_string())
        }
    }

    pub fn word(&self) -> String {
        match self {
            ListRef::None => "none".into(),
            ListRef::Local(n) | ListRef::Remote(n) => n.clone(),
        }
    }
}

/// One listserver in the layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListserverPlan {
    /// Folder name under `listservers/`, and how gservers name it.
    pub name: String,
    /// The list's name in the status text.
    pub title: String,
    /// The port gservers register on.
    pub gserver_port: u16,
    /// The ports clients ask for the list on.
    pub client_ports: Vec<u16>,
    /// An accounts file (only its accounts get in) instead of open mode
    /// (every account and password gets in).
    pub accounts_file: bool,
    /// Let only this layout's gservers register: a `gservers` allow-list
    /// with a random HQ password per gserver, which each gserver sends
    /// (`listserver_hq_password`). Off: any link to `gserver_port` may
    /// register.
    pub allow_list: bool,
}

/// One gserver in the layout (seihai-gserver).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GserverPlan {
    /// Folder name under `gservers/`.
    pub name: String,
    /// The server's name in the list.
    pub title: String,
    pub description: String,
    pub port: u16,
    pub listserver: ListRef,
    /// Check passwords with the listserver (`listserver_accounts`). Only
    /// with a listserver that has an accounts file.
    pub listserver_accounts: bool,
    pub npcserver: NpcMode,
    pub npc_port: u16,
    /// Make an account on its first login (lab only).
    pub create_accounts: bool,
}

/// A staff account: RC login on the gservers it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffPlan {
    pub account: String,
    /// Gserver names; empty: every gserver in the layout.
    pub gservers: Vec<String>,
    /// Where its RC may log in from (`IPRANGE`); empty: loopback only.
    pub ip_ranges: Vec<String>,
    /// File-browser rights (`FOLDER`), such as `rw levels/*`.
    pub folders: Vec<String>,
}

/// The default folder rights for a new staff account.
pub fn default_folders() -> Vec<String> {
    ["rw levels/*", "rw files/*", "rw npcs/*", "r config/*"]
        .map(String::from)
        .to_vec()
}

/// The whole layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Where the servers listen: `127.0.0.1` (this machine only, a lab) or
    /// `0.0.0.0` / one address (other machines too).
    pub bind: String,
    /// The address players elsewhere use for this machine. Empty: the
    /// listserver lists each gserver at the address its link comes from.
    pub public_host: String,
    pub listservers: Vec<ListserverPlan>,
    pub gservers: Vec<GserverPlan>,
    pub staff: Vec<StaffPlan>,
}

/// What [`Plan::basic`] builds.
#[derive(Debug, Clone)]
pub struct Basic {
    pub listservers: usize,
    pub gservers: usize,
    pub npcserver: NpcMode,
    /// `None`: a lab on 127.0.0.1. `Some(host)`: listen on every address
    /// and list the gservers at `host`.
    pub public_host: Option<String>,
    /// `HOST:PORT` of a listserver run elsewhere, used when `listservers`
    /// is 0.
    pub remote_listserver: Option<String>,
    pub staff: Vec<String>,
}

impl Default for Basic {
    fn default() -> Self {
        Basic {
            listservers: 1,
            gservers: 1,
            npcserver: NpcMode::Auto,
            public_host: None,
            remote_listserver: None,
            staff: vec!["admin".into()],
        }
    }
}

/// Default name of the `i`th server of a kind (from 0).
fn nth(base: &str, i: usize) -> String {
    if i == 0 {
        base.to_string()
    } else {
        format!("{base}{}", i + 1)
    }
}

impl ListserverPlan {
    /// The `i`th listserver's defaults: the lab ports, then 50 up each.
    pub fn nth(i: usize, lab: bool) -> Self {
        let step = LIST_STEP * i as u16;
        ListserverPlan {
            name: nth("list", i),
            title: format!(
                "{} listserver",
                if i == 0 {
                    "Main".into()
                } else {
                    format!("List {}", i + 1)
                }
            ),
            gserver_port: LIST_GSERVER_PORT + step,
            client_ports: LIST_CLIENT_PORTS.iter().map(|p| p + step).collect(),
            accounts_file: !lab,
            allow_list: !lab,
        }
    }
}

impl GserverPlan {
    /// The `i`th gserver's defaults: 14802 and NPC link 14810, then 10 up each.
    pub fn nth(i: usize, listserver: ListRef, npc: NpcMode, lab: bool) -> Self {
        let step = GSERVER_STEP * i as u16;
        GserverPlan {
            name: nth("world", i),
            title: if i == 0 {
                "My Server".into()
            } else {
                format!("My Server {}", i + 1)
            },
            description: String::new(),
            port: GSERVER_PORT + step,
            listserver,
            listserver_accounts: false,
            npcserver: npc,
            npc_port: NPC_PORT + step,
            create_accounts: lab,
        }
    }
}

impl Plan {
    /// A layout from a few answers: the gservers take the listservers in
    /// turn (or all use the remote one), every staff account on every gserver.
    pub fn basic(b: &Basic) -> Plan {
        let lab = b.public_host.is_none();
        let listservers: Vec<_> = (0..b.listservers)
            .map(|i| ListserverPlan::nth(i, lab))
            .collect();
        // Gservers take the listservers in turn: with two of each, one each.
        let list = |i: usize| match (
            listservers.get(i % listservers.len().max(1)),
            &b.remote_listserver,
        ) {
            (Some(l), _) => ListRef::Local(l.name.clone()),
            (None, Some(r)) => ListRef::Remote(r.clone()),
            (None, None) => ListRef::None,
        };
        let gservers = (0..b.gservers)
            .map(|i| GserverPlan::nth(i, list(i), b.npcserver, lab))
            .collect();
        Plan {
            bind: if lab {
                "127.0.0.1".into()
            } else {
                "0.0.0.0".into()
            },
            public_host: b.public_host.clone().unwrap_or_default(),
            listservers,
            gservers,
            staff: b
                .staff
                .iter()
                .map(|a| StaffPlan {
                    account: a.clone(),
                    gservers: Vec::new(),
                    ip_ranges: vec!["127.0.0.1".into()],
                    folders: default_folders(),
                })
                .collect(),
        }
    }

    /// True when every server listens on loopback only.
    pub fn is_lab(&self) -> bool {
        crate::layout::is_loopback(&self.bind)
    }

    pub fn listserver(&self, name: &str) -> Option<&ListserverPlan> {
        self.listservers
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
    }

    /// The gservers a staff account is made on.
    pub fn staff_gservers<'a>(&'a self, s: &'a StaffPlan) -> impl Iterator<Item = &'a GserverPlan> {
        self.gservers.iter().filter(move |g| {
            s.gservers.is_empty() || s.gservers.iter().any(|n| n.eq_ignore_ascii_case(&g.name))
        })
    }

    /// The listservers with an accounts file that list one of the staff
    /// account's gservers: the account is added there too.
    pub fn staff_listservers<'a>(
        &'a self,
        s: &'a StaffPlan,
    ) -> impl Iterator<Item = &'a ListserverPlan> {
        self.listservers.iter().filter(move |l| {
            l.accounts_file
                && self.staff_gservers(s).any(
                    |g| matches!(&g.listserver, ListRef::Local(n) if n.eq_ignore_ascii_case(&l.name)),
                )
        })
    }

    /// The plan as `layout.toml`.
    pub fn to_toml(&self) -> String {
        let mut doc = DocumentMut::new();
        doc["bind"] = value(&self.bind);
        doc["public_host"] = value(&self.public_host);
        let mut ls = ArrayOfTables::new();
        for l in &self.listservers {
            let mut t = Table::new();
            t["name"] = value(&l.name);
            t["title"] = value(&l.title);
            t["gserver_port"] = value(i64::from(l.gserver_port));
            t["client_ports"] = value(ports_array(&l.client_ports));
            t["accounts"] = value(if l.accounts_file { "file" } else { "open" });
            t["allow_list"] = value(l.allow_list);
            ls.push(t);
        }
        if !ls.is_empty() {
            doc["listserver"] = Item::ArrayOfTables(ls);
        }
        let mut gs = ArrayOfTables::new();
        for g in &self.gservers {
            let mut t = Table::new();
            t["name"] = value(&g.name);
            t["title"] = value(&g.title);
            t["description"] = value(&g.description);
            t["port"] = value(i64::from(g.port));
            t["listserver"] = value(g.listserver.word());
            t["listserver_accounts"] = value(g.listserver_accounts);
            t["npcserver"] = value(g.npcserver.word());
            t["npcserver_port"] = value(i64::from(g.npc_port));
            t["create_accounts"] = value(g.create_accounts);
            gs.push(t);
        }
        if !gs.is_empty() {
            doc["gserver"] = Item::ArrayOfTables(gs);
        }
        let mut st = ArrayOfTables::new();
        for s in &self.staff {
            let mut t = Table::new();
            t["account"] = value(&s.account);
            t["gservers"] = value(strings_array(&s.gservers));
            t["ip_ranges"] = value(strings_array(&s.ip_ranges));
            t["folders"] = value(strings_array(&s.folders));
            st.push(t);
        }
        if !st.is_empty() {
            doc["staff"] = Item::ArrayOfTables(st);
        }
        format!(
            "# seihai-servertools layout: what `setup new --plan` builds.\n\
             # Edit it and run `seihai-servertools setup new DIR --plan layout.toml`\n\
             # to make another folder like it. `seihai-servertools guide setup`\n\
             # explains each key.\n{doc}"
        )
    }

    /// Read a `layout.toml`. Keys left out take the defaults of
    /// [`ListserverPlan::nth`] and [`GserverPlan::nth`].
    pub fn from_toml(text: &str) -> Result<Plan, String> {
        let doc: DocumentMut = text.parse().map_err(|e| format!("layout: {e}"))?;
        let bind = str_key(doc.as_table(), "bind")?.unwrap_or_else(|| "127.0.0.1".into());
        let lab = crate::layout::is_loopback(&bind);
        let mut plan = Plan {
            public_host: str_key(doc.as_table(), "public_host")?.unwrap_or_default(),
            bind,
            listservers: Vec::new(),
            gservers: Vec::new(),
            staff: Vec::new(),
        };
        for (i, t) in tables(&doc, "listserver")?.into_iter().enumerate() {
            let mut l = ListserverPlan::nth(i, lab);
            if let Some(v) = str_key(t, "name")? {
                l.name = v;
            }
            if let Some(v) = str_key(t, "title")? {
                l.title = v;
            }
            if let Some(v) = port_key(t, "gserver_port")? {
                l.gserver_port = v;
            }
            if let Some(v) = t.get("client_ports") {
                l.client_ports = port_list(v, "client_ports")?;
            }
            if let Some(v) = bool_key(t, "allow_list")? {
                l.allow_list = v;
            }
            if let Some(v) = str_key(t, "accounts")? {
                l.accounts_file = match v.as_str() {
                    "file" => true,
                    "open" => false,
                    o => return Err(format!("listserver accounts: `{o}` is not file or open")),
                };
            }
            plan.listservers.push(l);
        }
        let first = plan
            .listservers
            .first()
            .map(|l| ListRef::Local(l.name.clone()))
            .unwrap_or(ListRef::None);
        for (i, t) in tables(&doc, "gserver")?.into_iter().enumerate() {
            let mut g = GserverPlan::nth(i, first.clone(), NpcMode::Auto, lab);
            if let Some(v) = str_key(t, "name")? {
                g.name = v;
            }
            if let Some(v) = str_key(t, "title")? {
                g.title = v;
            }
            if let Some(v) = str_key(t, "description")? {
                g.description = v;
            }
            if let Some(v) = port_key(t, "port")? {
                g.port = v;
            }
            if let Some(v) = str_key(t, "listserver")? {
                g.listserver = ListRef::parse(&v);
            }
            if let Some(v) = bool_key(t, "listserver_accounts")? {
                g.listserver_accounts = v;
            }
            if let Some(v) = str_key(t, "npcserver")? {
                g.npcserver = NpcMode::parse(&v)?;
            }
            if let Some(v) = port_key(t, "npcserver_port")? {
                g.npc_port = v;
            }
            if let Some(v) = bool_key(t, "create_accounts")? {
                g.create_accounts = v;
            }
            plan.gservers.push(g);
        }
        for t in tables(&doc, "staff")? {
            let account = str_key(t, "account")?.ok_or("staff: every [[staff]] needs account")?;
            plan.staff.push(StaffPlan {
                account,
                gservers: str_list(t, "gservers")?.unwrap_or_default(),
                ip_ranges: str_list(t, "ip_ranges")?.unwrap_or_else(|| vec!["127.0.0.1".into()]),
                folders: str_list(t, "folders")?.unwrap_or_else(default_folders),
            });
        }
        Ok(plan)
    }
}

fn ports_array(p: &[u16]) -> Array {
    p.iter().map(|&p| i64::from(p)).collect()
}

fn strings_array(s: &[String]) -> Array {
    s.iter().map(String::as_str).collect()
}

fn tables<'a>(doc: &'a DocumentMut, key: &str) -> Result<Vec<&'a Table>, String> {
    match doc.get(key) {
        None => Ok(Vec::new()),
        Some(Item::ArrayOfTables(a)) => Ok(a.iter().collect()),
        Some(_) => Err(format!("layout: `{key}` must be [[{key}]] tables")),
    }
}

fn str_key(t: &Table, key: &str) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or(format!("layout: `{key}` must be a string")),
    }
}

fn bool_key(t: &Table, key: &str) -> Result<Option<bool>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_bool()
            .map(Some)
            .ok_or(format!("layout: `{key}` must be true or false")),
    }
}

fn to_port(n: Option<i64>, key: &str) -> Result<u16, String> {
    n.and_then(|n| u16::try_from(n).ok())
        .filter(|&p| p > 0)
        .ok_or(format!("layout: `{key}` must be a port, 1 to 65535"))
}

fn port_key(t: &Table, key: &str) -> Result<Option<u16>, String> {
    t.get(key).map(|v| to_port(v.as_integer(), key)).transpose()
}

fn port_list(v: &Item, key: &str) -> Result<Vec<u16>, String> {
    let a = v
        .as_array()
        .ok_or(format!("layout: `{key}` must be a list of ports"))?;
    a.iter().map(|p| to_port(p.as_integer(), key)).collect()
}

fn str_list(t: &Table, key: &str) -> Result<Option<Vec<String>>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(v) => {
            let a = v
                .as_array()
                .ok_or(format!("layout: `{key}` must be a list of strings"))?;
            a.iter()
                .map(|s| {
                    s.as_str()
                        .map(str::to_string)
                        .ok_or(format!("layout: `{key}` must be a list of strings"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lab_layout() {
        let p = Plan::basic(&Basic {
            gservers: 3,
            ..Default::default()
        });
        assert!(p.is_lab());
        assert_eq!(p.listservers.len(), 1);
        assert_eq!(p.listservers[0].gserver_port, 14900);
        assert!(!p.listservers[0].accounts_file);
        let ports: Vec<_> = p.gservers.iter().map(|g| (g.port, g.npc_port)).collect();
        assert_eq!(ports, [(14802, 14810), (14812, 14820), (14822, 14830)]);
        assert!(p
            .gservers
            .iter()
            .all(|g| g.listserver == ListRef::Local("list".into())));
        assert_eq!(p.staff_gservers(&p.staff[0]).count(), 3);
    }

    #[test]
    fn toml_round_trip() {
        let mut p = Plan::basic(&Basic {
            listservers: 2,
            gservers: 2,
            npcserver: NpcMode::External,
            public_host: Some("<public-host>".into()),
            ..Default::default()
        });
        assert_eq!(p.gservers[1].listserver, ListRef::Local("list2".into()));
        p.gservers[1].npcserver = NpcMode::Off;
        p.staff[0].gservers = vec!["world2".into()];
        let back = Plan::from_toml(&p.to_toml()).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn short_toml_takes_defaults() {
        let p = Plan::from_toml(
            "[[listserver]]\n[[gserver]]\nname = \"hub\"\n[[gserver]]\nlistserver = \"none\"\nnpcserver = \"off\"\n[[staff]]\naccount = \"boss\"\n",
        )
        .unwrap();
        assert_eq!(p.bind, "127.0.0.1");
        assert_eq!(p.gservers[0].name, "hub");
        assert_eq!(p.gservers[0].listserver, ListRef::Local("list".into()));
        assert_eq!(p.gservers[1].port, 14812);
        assert_eq!(p.gservers[1].listserver, ListRef::None);
        assert_eq!(p.staff[0].folders, default_folders());
        assert!(Plan::from_toml("[[gserver]]\nport = 70000\n").is_err());
        assert!(Plan::from_toml("[[gserver]]\nnpcserver = \"maybe\"\n").is_err());
        assert!(Plan::from_toml("[[staff]]\n").is_err());
    }

    #[test]
    fn list_refs() {
        assert_eq!(ListRef::parse(""), ListRef::None);
        assert_eq!(ListRef::parse("main"), ListRef::Local("main".into()));
        assert_eq!(
            ListRef::parse("<listserver-host>:14900"),
            ListRef::Remote("<listserver-host>:14900".into())
        );
    }
}
