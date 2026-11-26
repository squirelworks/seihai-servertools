//! The text of every file a layout gets: the listserver TOML, each
//! gserver's `config/server.txt`, the start and stop scripts and the
//! folder's README.

use std::path::Path;

use super::check::{is_loopback, ports};
use super::plan::{GserverPlan, ListRef, ListserverPlan, NpcMode, Plan};

/// Folder of listserver `name`, relative to the layout.
pub fn listserver_dir(name: &str) -> String {
    format!("listservers/{name}")
}

/// Folder (the world) of gserver `name`, relative to the layout.
pub fn gserver_dir(name: &str) -> String {
    format!("gservers/{name}")
}

/// The NPC-server token file of an `external` gserver, relative to the layout.
pub fn token_file(gserver: &str) -> String {
    format!("{}/config/npcserver-token.txt", gserver_dir(gserver))
}

/// The address a program on this machine uses to reach a server that
/// listens on `bind`.
pub fn local_addr(bind: &str) -> String {
    match bind.trim() {
        "0.0.0.0" | "" => "127.0.0.1".into(),
        "::" => "::1".into(),
        b => b.to_string(),
    }
}

fn toml_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' | '\r' | '\t' => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// One line of `key = value` text: no newlines, no `#` (a comment there).
fn line_value(s: &str) -> String {
    s.replace(['\n', '\r'], " ").replace('#', "")
}

/// `seihai-listserver.toml` for one listserver.
/// The address a gserver of this layout registers from: the one it
/// connects to the listserver on, as both run on this machine.
pub fn register_ip(plan: &Plan) -> String {
    local_addr(&plan.bind)
}

/// `seihai-listserver.toml` for one listserver. `allowed` is its
/// allow-list, `(gserver name, HQ password)` (empty unless `allow_list`).
pub fn listserver_toml(plan: &Plan, l: &ListserverPlan, allowed: &[(String, String)]) -> String {
    let ports = l
        .client_ports
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let sent = !plan.public_host.trim().is_empty();
    format!(
        "# Written by `seihai-servertools setup new`. Every setting, with its\n\
         # default: `seihai-listserver sample-config`. The listserver reads this\n\
         # file again when it changes; bind and ports take a restart.\n\
         \n\
         bind = {bind}\n\
         # Gservers register here. When other machines can reach it, open this\n\
         # port only to your own gservers.\n\
         gserver_port = {gp}\n\
         # Clients and RCs ask for the server list on these.\n\
         client_ports = [{ports}]\n\
         list_name = {title}\n\
         \n\
         # Empty: open mode, every account and password gets in (a lab).\n\
         # A file: only its accounts (`seihai-listserver add-account FILE NAME`).\n\
         accounts = {accounts}\n\
         log_file = \"\"\n\
         \n\
         # Where the list says each gserver is: \"peer\" (the address its link\n\
         # comes from) or \"sent\" (the listserver_ip the gserver sends).\n\
         gserver_ip = {gip}\n\
         \n\
         status_log_secs = 60\n\
         secure_login_key = \"seihai-login.key\"\n\
         \n\
         {allow}",
        bind = toml_str(&plan.bind),
        gp = l.gserver_port,
        title = toml_str(&l.title),
        accounts = toml_str(if l.accounts_file { "accounts.txt" } else { "" }),
        gip = toml_str(if sent { "sent" } else { "peer" }),
        allow = allow_list_toml(plan, allowed),
    )
}

fn allow_list_toml(plan: &Plan, allowed: &[(String, String)]) -> String {
    if allowed.is_empty() {
        return "# `gservers = [...]` would let only those addresses register, each\n\
                # with its HQ password (listserver_hq_password on the gserver).\n\
                # Empty: any link to gserver_port may register.\n"
            .into();
    }
    let ip = toml_str(&register_ip(plan));
    let rows: Vec<String> = allowed
        .iter()
        .map(|(g, pw)| format!("  {{ ip = {ip}, hq_password = {} }}, # {g}", toml_str(pw)))
        .collect();
    format!(
        "# Only these may register: each of this layout's gservers, with the HQ\n\
         # password it sends (listserver_hq_password in its config/server.txt).\n\
         # Keep this file private. Another gserver needs its own row.\n\
         gservers = [\n{}\n]\n",
        rows.join("\n")
    )
}

/// The `listserver =` address for a gserver.
pub fn listserver_addr(plan: &Plan, g: &GserverPlan) -> Option<String> {
    match &g.listserver {
        ListRef::None => None,
        ListRef::Remote(a) => Some(a.clone()),
        ListRef::Local(n) => plan
            .listserver(n)
            .map(|l| format!("{}:{}", local_addr(&plan.bind), l.gserver_port)),
    }
}

/// `config/server.txt` for one gserver. `npc_bin` is the NPC-server
/// program for `auto` (empty: next to seihai-gserver); `token` the shared
/// token for `external`; `hq` the HQ password its listserver's allow-list
/// expects (empty: none).
pub fn gserver_txt(plan: &Plan, g: &GserverPlan, npc_bin: &str, token: &str, hq: &str) -> String {
    let mut s = format!(
        "# Written by `seihai-servertools setup new`. Every setting, with its\n\
         # default and a comment: seihai-gserver's sample world/config/server.txt.\n\
         \n\
         name = {}\n\
         description = {}\n\
         # Where players and RCs connect. 127.0.0.1: this machine only.\n\
         bind = {}\n\
         port = {}\n\
         # Make an account on the first login with the password given (lab only).\n\
         create_accounts = {}\n\
         heartbeat = 5\n\
         \n",
        line_value(&g.title),
        line_value(&g.description),
        line_value(&plan.bind),
        g.port,
        g.create_accounts,
    );
    s.push_str("# The listserver this gserver registers with (its gserver port).\n");
    match listserver_addr(plan, g) {
        Some(a) => {
            s.push_str(&format!("listserver = {}\n", line_value(&a)));
            s.push_str(&format!(
                "# Check passwords with the listserver's accounts instead of this world's.\nlistserver_accounts = {}\n",
                g.listserver_accounts
            ));
            if !plan.public_host.trim().is_empty() {
                s.push_str(&format!(
                    "# The address the list shows for this server (gserver_ip = \"sent\").\nlistserver_ip = {}\n",
                    line_value(plan.public_host.trim())
                ));
            }
            if !hq.is_empty() {
                s.push_str(&format!(
                    "# Sent before registering; the listserver's gservers allow-list has\n# the same text for this server. Keep it private.\nlistserver_hq_password = {}\n",
                    line_value(hq)
                ));
            } else if matches!(g.listserver, ListRef::Remote(_)) {
                s.push_str("# If that listserver has an allow-list, its operator gives you an HQ\n# password: listserver_hq_password = ...\n");
            }
            if matches!(g.listserver, ListRef::Remote(_)) {
                s.push_str("# The link is encrypted; set listserver_secure = false only for a\n# listserver that does not know the handshake (GServer-v2's).\n");
            }
        }
        None => s.push_str("listserver =\n"),
    }
    s.push_str(&format!(
        "\n# The NPC-server: off, auto (this gserver starts seihai-npcserver) or\n\
         # external (start.sh starts it with the token file).\n\
         npcserver = {}\n",
        g.npcserver.word()
    ));
    if g.npcserver != NpcMode::Off {
        s.push_str(&format!(
            "npcserver_bind = 127.0.0.1\nnpcserver_port = {}\n",
            g.npc_port
        ));
    }
    match g.npcserver {
        NpcMode::Auto if !npc_bin.is_empty() => {
            s.push_str(&format!("npcserver_path = {}\n", line_value(npc_bin)))
        }
        NpcMode::External => s.push_str(&format!(
            "# The same text as {}; keep both private.\nnpcserver_token = {token}\n",
            Path::new(&token_file(&g.name))
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default()
        )),
        _ => {}
    }
    s
}

/// One program a start script runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// `listserver-list`, `gserver-world`, `npcserver-world`: the log
    /// and pid file names.
    pub label: String,
    /// `seihai-listserver`, `seihai-gserver`, `seihai-npcserver`.
    pub program: &'static str,
    /// Arguments, with paths relative to the layout and `/` separators.
    pub args: Vec<String>,
}

/// What the start scripts run, in order: listservers, gservers, then
/// the external NPC-servers.
pub fn launches(plan: &Plan) -> Vec<Launch> {
    let mut v = Vec::new();
    for l in &plan.listservers {
        v.push(Launch {
            label: format!("listserver-{}", l.name),
            program: "seihai-listserver",
            args: vec![
                "--config".into(),
                format!("{}/seihai-listserver.toml", listserver_dir(&l.name)),
            ],
        });
    }
    for g in &plan.gservers {
        v.push(Launch {
            label: format!("gserver-{}", g.name),
            program: "seihai-gserver",
            args: vec!["--world".into(), gserver_dir(&g.name)],
        });
    }
    for g in plan
        .gservers
        .iter()
        .filter(|g| g.npcserver == NpcMode::External)
    {
        v.push(Launch {
            label: format!("npcserver-{}", g.name),
            program: "seihai-npcserver",
            args: vec![
                "--gserver".into(),
                format!("127.0.0.1:{}", g.npc_port),
                "--token-file".into(),
                token_file(&g.name),
                "--world".into(),
                gserver_dir(&g.name),
            ],
        });
    }
    v
}

fn sh_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:=".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// `start.sh` (Linux and macOS). `bin_dir` is where the server programs
/// are (empty: on PATH); `SEIHAI_BIN_DIR` overrides it.
pub fn start_sh(plan: &Plan, bin_dir: &str) -> String {
    let mut s = format!(
        "#!/bin/sh\n\
         # Starts this layout's servers in the background; ./stop.sh stops them.\n\
         # Written by `seihai-servertools setup new`. Logs go to logs/, process\n\
         # ids to run/. SEIHAI_BIN_DIR overrides where the programs are.\n\
         cd \"$(dirname \"$0\")\" || exit 1\n\
         BIN=\"${{SEIHAI_BIN_DIR:-{}}}\"\n\
         mkdir -p logs run\n\
         \n\
         prog() {{ if [ -n \"$BIN\" ]; then echo \"$BIN/$1\"; else echo \"$1\"; fi; }}\n\
         \n\
         launch() {{\n\
         \tname=$1; shift\n\
         \tif [ -f \"run/$name.pid\" ] && kill -0 \"$(cat \"run/$name.pid\")\" 2>/dev/null; then\n\
         \t\techo \"$name is already running (pid $(cat \"run/$name.pid\"))\"\n\
         \t\treturn\n\
         \tfi\n\
         \tp=$(prog \"$1\"); shift\n\
         \t\"$p\" \"$@\" >\"logs/$name.log\" 2>&1 &\n\
         \techo $! >\"run/$name.pid\"\n\
         \techo \"started $name (pid $!), log: logs/$name.log\"\n\
         }}\n\n",
        bin_dir.replace('"', "")
    );
    let mut last = "";
    for l in launches(plan) {
        if !last.is_empty() && l.program != last {
            s.push_str("# Give the servers above a moment to open their ports.\nsleep 1\n");
        }
        last = l.program;
        let args: Vec<String> = l.args.iter().map(|a| sh_quote(a)).collect();
        s.push_str(&format!(
            "launch {} {} {}\n",
            l.label,
            l.program,
            args.join(" ")
        ));
    }
    s.push_str("echo \"All started. Check with: tail logs/*.log   Stop with: ./stop.sh\"\n");
    s
}

/// `stop.sh`: stops what start.sh started, last first.
pub fn stop_sh(plan: &Plan) -> String {
    let mut labels: Vec<String> = launches(plan).into_iter().map(|l| l.label).collect();
    labels.reverse();
    format!(
        "#!/bin/sh\n\
         # Stops the servers ./start.sh started (SIGTERM, then SIGKILL after 5 s).\n\
         cd \"$(dirname \"$0\")\" || exit 1\n\
         for name in {}; do\n\
         \tf=\"run/$name.pid\"\n\
         \t[ -f \"$f\" ] || continue\n\
         \tpid=$(cat \"$f\")\n\
         \tif kill \"$pid\" 2>/dev/null; then\n\
         \t\ti=0\n\
         \t\twhile kill -0 \"$pid\" 2>/dev/null && [ $i -lt 50 ]; do sleep 0.1; i=$((i + 1)); done\n\
         \t\tkill -9 \"$pid\" 2>/dev/null\n\
         \t\techo \"stopped $name\"\n\
         \tfi\n\
         \trm -f \"$f\"\n\
         done\n",
        labels.join(" ")
    )
}

fn crlf(s: String) -> String {
    s.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// `start.bat` (Windows): each server in its own minimised window, titled
/// `seihai-LABEL`, so stop.bat can find it.
pub fn start_bat(plan: &Plan, bin_dir: &str) -> String {
    let mut s = format!(
        "@echo off\n\
         rem Starts this layout's servers, each in a minimised window; stop.bat stops them.\n\
         rem Written by `seihai-servertools setup new`. Logs go to logs\\.\n\
         rem SEIHAI_BIN_DIR overrides where the programs are.\n\
         cd /d \"%~dp0\"\n\
         if \"%SEIHAI_BIN_DIR%\"==\"\" set \"SEIHAI_BIN_DIR={}\"\n\
         if not exist logs mkdir logs\n",
        bin_dir.replace('"', "")
    );
    let mut last = "";
    for l in launches(plan) {
        if !last.is_empty() && l.program != last {
            s.push_str("timeout /t 1 /nobreak >nul\n");
        }
        last = l.program;
        let args: Vec<String> = l
            .args
            .iter()
            .map(|a| format!("\"{}\"", a.replace('/', "\\")))
            .collect();
        s.push_str(&format!(
            "start \"seihai-{label}\" /min cmd /c \"\"%SEIHAI_BIN_DIR%\\{prog}.exe\" {args} > logs\\{label}.log 2>&1\"\n\
             echo started {label}, log: logs\\{label}.log\n",
            label = l.label,
            prog = l.program,
            args = args.join(" ")
        ));
    }
    crlf(s)
}

/// `stop.bat`: closes the windows start.bat opened.
pub fn stop_bat(plan: &Plan) -> String {
    let mut s = String::from("@echo off\nrem Stops the servers start.bat started.\n");
    let mut labels: Vec<String> = launches(plan).into_iter().map(|l| l.label).collect();
    labels.reverse();
    for l in labels {
        s.push_str(&format!(
            "taskkill /fi \"WINDOWTITLE eq seihai-{l}*\" /t /f >nul 2>&1 && echo stopped {l}\n"
        ));
    }
    crlf(s)
}

/// The layout's README.txt: what is where, how to start it, how to log in
/// with an RC.
pub fn readme(plan: &Plan) -> String {
    let mut s = String::from(
        "This server folder was written by `seihai-servertools setup new`.\n\
         `seihai-servertools guide` explains how the pieces fit together.\n\n",
    );
    s.push_str("WHAT IS HERE\n\n");
    for l in &plan.listservers {
        s.push_str(&format!(
            "  listservers/{}/   listserver \"{}\": gservers register on {}, clients ask on {}\n",
            l.name,
            l.title,
            l.gserver_port,
            l.client_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for g in &plan.gservers {
        let list = match &g.listserver {
            ListRef::None => "not listed".to_string(),
            ListRef::Local(n) => format!("listed on {n}"),
            ListRef::Remote(a) => format!("listed on {a}"),
        };
        let npc = match g.npcserver {
            NpcMode::Off => "no NPC-server".to_string(),
            NpcMode::Auto => format!("NPC-server started by the gserver (link {})", g.npc_port),
            NpcMode::External => format!("NPC-server started by start.sh (link {})", g.npc_port),
        };
        s.push_str(&format!(
            "  gservers/{}/   gserver \"{}\" on port {}, {list}, {npc}\n",
            g.name, g.title, g.port
        ));
    }
    s.push_str("\nPORTS\n\n");
    for p in ports(plan) {
        s.push_str(&format!("  {:>5}  {}  {}\n", p.port, p.bind, p.owner));
    }
    s.push_str(
        "\nSTART AND STOP\n\n  ./start.sh   and   ./stop.sh        (Linux, macOS)\n  start.bat    and   stop.bat         (Windows)\n\n  Logs are in logs/. A gserver's log says `listserver link to ...: registered`\n  once it is on the list, and `NPC-server connected` once its NPC-server is in.\n",
    );
    let host = if is_loopback(&plan.bind) || plan.public_host.trim().is_empty() {
        local_addr(&plan.bind)
    } else {
        plan.public_host.trim().to_string()
    };
    s.push_str("\nPLAYERS\n\n");
    if let Some(l) = plan.listservers.first() {
        s.push_str(&format!(
            "  Point a client's server list at {host}:{}. Or connect straight to a\n  gserver, e.g. {host}:{}.\n",
            l.client_ports.first().copied().unwrap_or(0),
            plan.gservers.first().map_or(0, |g| g.port)
        ));
    } else {
        s.push_str(&format!(
            "  Connect straight to a gserver, e.g. {host}:{}.\n",
            plan.gservers.first().map_or(0, |g| g.port)
        ));
    }
    if plan.gservers.iter().any(|g| g.create_accounts) {
        s.push_str("  create_accounts is on: a new account is made at its first login.\n");
    }
    s.push_str(
        "  Otherwise make player accounts with\n    seihai-gserver account NAME --password-env VAR --world gservers/NAME\n",
    );
    if plan.listservers.iter().any(|l| l.accounts_file) {
        s.push_str("  and, for a listserver with an accounts file,\n    seihai-listserver add-account listservers/LIST/accounts.txt NAME --password-env VAR\n");
    }
    s.push_str("\nRC (REMOTE CONTROL)\n\n");
    if let (Some(st), Some(g)) = (plan.staff.first(), plan.gservers.first()) {
        let addr = format!("{}:{}", local_addr(&plan.bind), g.port);
        s.push_str(&format!(
            "  export SEIHAI_RC_PASSWORD=...      (the password printed when this was made)\n\
             \x20 seihai-rc players list --server {addr} --account {a}\n\
             \x20 seihai-rc login --server {addr} --account {a} --for 30\n",
            a = st.account
        ));
        if let Some(l) = plan.listservers.first() {
            s.push_str(&format!(
                "  seihai-rc servers --listserver {}:{} --account {}\n",
                local_addr(&plan.bind),
                l.client_ports.first().copied().unwrap_or(0),
                st.account
            ));
        }
        if plan.gservers.iter().any(|g| g.npcserver != NpcMode::Off) {
            s.push_str(&format!(
                "  seihai-rc nc npcs --server {addr} --account {}      (NPC Control)\n",
                st.account
            ));
        }
        s.push_str("  `seihai-servertools guide rc` has the rest.\n");
    } else {
        s.push_str("  No staff account was made. `seihai-servertools guide rc` says how.\n");
    }
    s.push_str(
        "\nTROUBLE\n\n  `seihai-servertools guide troubleshooting`, and the end of each log in logs/.\n\n\
         layout.toml is the plan this folder was made from (no passwords). Run\n\
         `seihai-servertools setup new OTHERDIR --plan layout.toml` to make another.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::super::plan::Basic;
    use super::*;

    #[test]
    fn gserver_points_at_its_listserver() {
        let mut p = Plan::basic(&Basic {
            listservers: 2,
            gservers: 2,
            ..Default::default()
        });
        p.gservers[1].listserver = ListRef::Local("list2".into());
        let a = gserver_txt(&p, &p.gservers[0], "/bin/seihai-npcserver", "", "");
        assert!(a.contains("listserver = 127.0.0.1:14900\n"), "{a}");
        assert!(a.contains("npcserver = auto\n"));
        assert!(a.contains("npcserver_port = 14810\n"));
        assert!(a.contains("npcserver_path = /bin/seihai-npcserver\n"));
        let b = gserver_txt(&p, &p.gservers[1], "", "", "");
        assert!(b.contains("listserver = 127.0.0.1:14950\n"), "{b}");
        assert!(b.contains("port = 14812\n"));
    }

    #[test]
    fn public_layout_lists_at_public_host() {
        let mut p = Plan::basic(&Basic {
            public_host: Some("<public-host>".into()),
            npcserver: NpcMode::External,
            ..Default::default()
        });
        p.gservers[0].title = "Bad # name\nx".into();
        let g = gserver_txt(&p, &p.gservers[0], "", "tok-0123456789abcdef", "hq-secret");
        assert!(g.contains("bind = 0.0.0.0\n"));
        assert!(g.contains("listserver = 127.0.0.1:14900\n"));
        assert!(g.contains("listserver_ip = <public-host>\n"));
        assert!(g.contains("npcserver_token = tok-0123456789abcdef\n"));
        assert!(g.contains("name = Bad  name x\n"));
        assert!(g.contains("listserver_hq_password = hq-secret\n"));
        let l = listserver_toml(
            &p,
            &p.listservers[0],
            &[("world".into(), "hq-secret".into())],
        );
        assert!(
            l.contains("{ ip = \"127.0.0.1\", hq_password = \"hq-secret\" }, # world"),
            "{l}"
        );
        let open = listserver_toml(&p, &p.listservers[0], &[]);
        assert!(!open.contains("\ngservers ="));
        assert!(l.contains("gserver_ip = \"sent\""));
        assert!(l.contains("accounts = \"accounts.txt\""));
        assert!(l.contains("bind = \"0.0.0.0\""));
        l.parse::<toml_edit::DocumentMut>().unwrap();
    }

    #[test]
    fn scripts_start_in_order() {
        let mut p = Plan::basic(&Basic {
            gservers: 2,
            npcserver: NpcMode::External,
            ..Default::default()
        });
        p.gservers[1].npcserver = NpcMode::Off;
        let order: Vec<_> = launches(&p).into_iter().map(|l| l.label).collect();
        assert_eq!(
            order,
            [
                "listserver-list",
                "gserver-world",
                "gserver-world2",
                "npcserver-world"
            ]
        );
        let sh = start_sh(&p, "/opt/seihai bin");
        assert!(sh.starts_with("#!/bin/sh\n"));
        assert!(sh.contains("BIN=\"${SEIHAI_BIN_DIR:-/opt/seihai bin}\""));
        assert!(sh.contains("launch npcserver-world seihai-npcserver --gserver 127.0.0.1:14810 --token-file gservers/world/config/npcserver-token.txt --world gservers/world\n"));
        assert!(stop_sh(&p)
            .contains("for name in npcserver-world gserver-world2 gserver-world listserver-list;"));
        let bat = start_bat(&p, "C:\\seihai");
        assert!(bat.contains("\r\n") && !bat.replace("\r\n", "").contains('\n'));
        assert!(bat
            .contains("\"%SEIHAI_BIN_DIR%\\seihai-gserver.exe\" \"--world\" \"gservers\\world\""));
        assert!(stop_bat(&p).contains("seihai-listserver-list*"));
    }

    #[test]
    fn readme_names_every_port() {
        let p = Plan::basic(&Basic {
            gservers: 2,
            ..Default::default()
        });
        let r = readme(&p);
        for port in ports(&p) {
            assert!(r.contains(&port.port.to_string()));
        }
        assert!(r.contains("seihai-rc players list --server 127.0.0.1:14802 --account admin"));
        assert!(r.contains("seihai-rc nc npcs --server 127.0.0.1:14802"));
    }
}
