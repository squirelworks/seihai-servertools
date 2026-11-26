//! The checks a layout must pass before it is written: names, links
//! between servers, and ports (taken twice in the plan, or busy on this
//! machine).

use std::net::{IpAddr, Ipv4Addr, TcpListener};

use super::plan::{ListRef, NpcMode, Plan};
use crate::checks::{Finding, Severity};

fn f(severity: Severity, key: &str, text: impl Into<String>) -> Finding {
    Finding {
        severity,
        key: (!key.is_empty()).then(|| key.to_string()),
        text: text.into(),
    }
}

/// True for `127.0.0.1`, `localhost`, `::1` and the rest of 127/8.
pub fn is_loopback(bind: &str) -> bool {
    let b = bind.trim();
    b.eq_ignore_ascii_case("localhost") || b.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn is_any(bind: &str) -> bool {
    matches!(bind.trim(), "0.0.0.0" | "::" | "")
}

/// Two listen addresses on one port clash: the same address, or either
/// one being every address.
fn overlaps(a: &str, b: &str) -> bool {
    is_any(a) || is_any(b) || a.trim().eq_ignore_ascii_case(b.trim())
}

fn good_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 40
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One port the layout listens on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortUse {
    /// `gserver world`, `listserver list (clients)`, ...
    pub owner: String,
    pub bind: String,
    pub port: u16,
}

/// Every port the layout listens on, in start order.
pub fn ports(plan: &Plan) -> Vec<PortUse> {
    let mut v = Vec::new();
    for l in &plan.listservers {
        v.push(PortUse {
            owner: format!("listserver {} (gservers register)", l.name),
            bind: plan.bind.clone(),
            port: l.gserver_port,
        });
        for &p in &l.client_ports {
            v.push(PortUse {
                owner: format!("listserver {} (clients)", l.name),
                bind: plan.bind.clone(),
                port: p,
            });
        }
    }
    for g in &plan.gservers {
        v.push(PortUse {
            owner: format!("gserver {} (players and RC)", g.name),
            bind: plan.bind.clone(),
            port: g.port,
        });
        if g.npcserver != NpcMode::Off {
            v.push(PortUse {
                owner: format!("gserver {} (NPC-server link)", g.name),
                bind: "127.0.0.1".into(),
                port: g.npc_port,
            });
        }
    }
    v
}

/// Whether something on this machine already listens on `bind:port`
/// (binding it fails). Nothing is sent on the network.
pub fn port_busy(bind: &str, port: u16) -> bool {
    let ip: IpAddr = if is_any(bind) {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    } else if bind.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        match bind.trim().parse() {
            Ok(ip) => ip,
            Err(_) => return false,
        }
    };
    TcpListener::bind((ip, port)).is_err()
}

/// The plan's findings. `machine` also checks each port against this
/// machine (a busy port is a warning: it may be this layout, running).
pub fn check(plan: &Plan, machine: bool) -> Vec<Finding> {
    use Severity::*;
    let mut out = Vec::new();
    let bind = plan.bind.trim();
    if !(is_any(bind) || bind.eq_ignore_ascii_case("localhost") || bind.parse::<IpAddr>().is_ok()) {
        out.push(f(Block, "bind", format!("`{bind}` is not an address to listen on (127.0.0.1, 0.0.0.0 or one of this machine's addresses)")));
    }
    if plan.gservers.is_empty() {
        out.push(f(Block, "gserver", "the layout has no gserver"));
    }
    let mut names: Vec<(String, &str)> = Vec::new();
    for (kind, n) in plan
        .listservers
        .iter()
        .map(|l| ("listserver", &l.name))
        .chain(plan.gservers.iter().map(|g| ("gserver", &g.name)))
    {
        if !good_name(n) {
            out.push(f(
                Block,
                kind,
                format!("{kind} name `{n}`: use letters, digits, - and _ only (up to 40)"),
            ));
        }
        if names
            .iter()
            .any(|(m, k)| m.eq_ignore_ascii_case(n) && *k == kind)
        {
            out.push(f(Block, kind, format!("two {kind}s are named `{n}`")));
        }
        names.push((n.clone(), kind));
    }
    for l in &plan.listservers {
        if l.client_ports.is_empty() {
            out.push(f(
                Block,
                "client_ports",
                format!("listserver {} has no client port", l.name),
            ));
        }
        if !plan.is_lab() && !l.accounts_file {
            out.push(f(Warn, "accounts", format!("listserver {} is open to other machines in open mode: every account and password gets in. Use accounts = \"file\" unless that is meant", l.name)));
        }
        if !plan.is_lab() && !l.allow_list {
            out.push(f(Warn, "allow_list", format!("listserver {} lets any program that reaches its gserver port register a server; use allow_list = true unless the port is firewalled", l.name)));
        }
        if !plan
            .gservers
            .iter()
            .any(|g| matches!(&g.listserver, ListRef::Local(n) if n.eq_ignore_ascii_case(&l.name)))
        {
            out.push(f(
                Info,
                "listserver",
                format!("no gserver registers with listserver {}", l.name),
            ));
        }
    }
    for g in &plan.gservers {
        match &g.listserver {
            ListRef::Local(n) => match plan.listserver(n) {
                None => out.push(f(Block, "listserver", format!("gserver {} names listserver `{n}`, which is not in the layout (give a name from [[listserver]], HOST:PORT for one elsewhere, or none)", g.name))),
                Some(l) => {
                    if g.listserver_accounts && !l.accounts_file {
                        out.push(f(Block, "listserver_accounts", format!("gserver {} checks passwords with listserver {}, which is in open mode and would let any password in", g.name, l.name)));
                    }
                }
            },
            ListRef::Remote(addr) => {
                let ok = addr
                    .rsplit_once(':')
                    .is_some_and(|(h, p)| !h.is_empty() && p.parse::<u16>().is_ok_and(|p| p > 0));
                if !ok {
                    out.push(f(Block, "listserver", format!("gserver {}: listserver `{addr}` is not HOST:PORT", g.name)));
                }
                if is_loopback(addr.rsplit_once(':').map_or("", |(h, _)| h)) {
                    out.push(f(Warn, "listserver", format!("gserver {}: listserver {addr} is on this machine but not in the layout; it must already be running", g.name)));
                }
            }
            ListRef::None => out.push(f(Info, "listserver", format!("gserver {} is not on any server list; players connect to its address directly", g.name))),
        }
        if g.create_accounts && !plan.is_lab() {
            out.push(f(Warn, "create_accounts", format!("gserver {} makes an account for anyone who logs in from another machine; leave create_accounts off outside a lab", g.name)));
        }
    }
    if !plan.is_lab() && plan.public_host.trim().is_empty() && !plan.listservers.is_empty() {
        out.push(f(Warn, "public_host", "no public host: the list shows each gserver at the address its link comes from, which is 127.0.0.1 for a gserver on this machine. Set public_host to the address players use"));
    }
    for s in &plan.staff {
        if !good_name(&s.account) {
            out.push(f(
                Block,
                "staff",
                format!(
                    "staff account `{}`: use letters, digits, - and _ only",
                    s.account
                ),
            ));
        }
        for n in &s.gservers {
            if !plan.gservers.iter().any(|g| g.name.eq_ignore_ascii_case(n)) {
                out.push(f(
                    Block,
                    "staff",
                    format!(
                        "staff account {} names gserver `{n}`, which is not in the layout",
                        s.account
                    ),
                ));
            }
        }
        if s.ip_ranges.iter().any(|r| r.trim() == "*") && !plan.is_lab() {
            out.push(f(
                Warn,
                "ip_ranges",
                format!(
                    "staff account {} may log in with an RC from any address",
                    s.account
                ),
            ));
        }
    }
    if plan.staff.is_empty() {
        out.push(f(Info, "staff", "no staff account: nobody can log in with an RC until one is made (`seihai-gserver account`, then STAFF 1 in its file)"));
    }
    // Ports: taken twice in the plan, then busy on this machine.
    let used = ports(plan);
    for (i, a) in used.iter().enumerate() {
        if let Some(b) = used[..i]
            .iter()
            .find(|b| b.port == a.port && overlaps(&a.bind, &b.bind))
        {
            out.push(f(
                Block,
                "port",
                format!("port {} is used twice: {} and {}", a.port, b.owner, a.owner),
            ));
        } else if machine && port_busy(&a.bind, a.port) {
            out.push(f(Warn, "port", format!("port {} ({}) is already in use on this machine. If it is not this layout running, pick another", a.port, a.owner)));
        }
        if a.port < 1024 {
            out.push(f(
                Warn,
                "port",
                format!(
                    "port {} ({}) is below 1024 and needs admin rights on most systems",
                    a.port, a.owner
                ),
            ));
        }
    }
    out
}

/// True when no finding blocks writing the layout.
pub fn may_write(findings: &[Finding]) -> bool {
    !findings.iter().any(|f| f.severity == Severity::Block)
}

#[cfg(test)]
mod tests {
    use super::super::plan::{Basic, GserverPlan};
    use super::*;

    fn blocks(p: &Plan) -> Vec<String> {
        check(p, false)
            .into_iter()
            .filter(|f| f.severity == Severity::Block)
            .map(|f| f.text)
            .collect()
    }

    #[test]
    fn default_layouts_pass() {
        for n in 1..=4 {
            let p = Plan::basic(&Basic {
                gservers: n,
                listservers: 2,
                ..Default::default()
            });
            assert!(blocks(&p).is_empty(), "{:?}", blocks(&p));
        }
    }

    #[test]
    fn port_clash_is_caught() {
        let mut p = Plan::basic(&Basic {
            gservers: 2,
            ..Default::default()
        });
        p.gservers[1].port = 14900;
        let b = blocks(&p);
        assert!(
            b.iter().any(|t| t.contains("port 14900 is used twice")),
            "{b:?}"
        );
        // The NPC link is on 127.0.0.1; it clashes with a gserver on 0.0.0.0.
        let mut q = Plan::basic(&Basic {
            gservers: 2,
            public_host: Some("<public-host>".into()),
            ..Default::default()
        });
        q.gservers[1].port = q.gservers[0].npc_port;
        assert!(!blocks(&q).is_empty());
    }

    #[test]
    fn bad_links_are_caught() {
        let mut p = Plan::basic(&Basic::default());
        p.gservers[0].listserver = ListRef::Local("nope".into());
        assert!(!blocks(&p).is_empty());
        p.gservers[0].listserver = ListRef::Remote("host:notaport".into());
        assert!(!blocks(&p).is_empty());
        p.gservers[0].listserver = ListRef::Local("list".into());
        p.gservers[0].listserver_accounts = true;
        assert!(blocks(&p).iter().any(|t| t.contains("open mode")));
        p.gservers[0].listserver_accounts = false;
        p.staff[0].gservers = vec!["nope".into()];
        assert!(!blocks(&p).is_empty());
        p.staff[0].gservers.clear();
        p.gservers.push(GserverPlan {
            name: "world".into(),
            ..p.gservers[0].clone()
        });
        assert!(blocks(&p).iter().any(|t| t.contains("two gservers")));
    }

    #[test]
    fn busy_port_is_a_warning() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert!(port_busy("127.0.0.1", port));
        let mut p = Plan::basic(&Basic::default());
        p.gservers[0].port = port;
        let fs = check(&p, true);
        assert!(fs
            .iter()
            .any(|f| f.severity == Severity::Warn && f.text.contains("already in use")));
        assert!(may_write(&fs));
    }

    #[test]
    fn public_layout_warnings() {
        let mut p = Plan::basic(&Basic {
            public_host: Some(String::new()),
            ..Default::default()
        });
        p.public_host.clear();
        p.listservers[0].accounts_file = false;
        p.listservers[0].allow_list = false;
        p.gservers[0].create_accounts = true;
        let text: Vec<_> = check(&p, false).into_iter().map(|f| f.text).collect();
        assert!(text.iter().any(|t| t.contains("open mode")));
        assert!(text.iter().any(|t| t.contains("no public host")));
        assert!(text.iter().any(|t| t.contains("create_accounts")));
        assert!(text.iter().any(|t| t.contains("allow_list = true")));
    }
}
