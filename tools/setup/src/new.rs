//! `setup new`: a server folder from scratch, asked for step by step or
//! given as options or a `layout.toml`. Every rule and file is in
//! `setup_core::layout`; this only asks, shows and calls it.

use std::path::PathBuf;
use std::process::ExitCode;

use setup_core::layout::{self, Basic, Bins, GserverPlan, ListRef, ListserverPlan, NpcMode, Plan};

use super::{print_findings, Io};

/// The help for `setup new`.
pub const NEW_USAGE: &str = "\
setup new [DIR] [--plan FILE] [--listservers N] [--gservers N]
          [--npcserver auto|external|off] [--public HOST]
          [--remote-listserver HOST:PORT] [--staff NAME]... [--rc-from RANGE]...
          [--bin-dir DIR]... [--no-port-check] [--print-plan] [--yes]

Writes a new server folder DIR (default seihai-server; it must not exist or
be empty) for the Seihai Rust stack: listservers, gservers each pointed at
a listserver, an NPC-server for each gserver that wants one, staff accounts
for RC with random passwords (printed once, saved nowhere), and start and
stop scripts (start.sh / stop.sh, start.bat / stop.bat).

Without --yes it asks for each choice, with the options as the defaults.
With --yes it asks nothing: the layout comes from --plan FILE (a
layout.toml, as every folder made gets one) or from the options:

  --listservers N      listservers run here (default 1; 0: none, or --remote-listserver)
  --gservers N         gservers (default 1), all on the first listserver
  --npcserver MODE     auto: each gserver starts its NPC-server (default);
                       external: start.sh starts it, with a token file; off
  --public HOST        listen on every address and list the gservers at HOST
                       (default: a lab on 127.0.0.1, this machine only)
  --remote-listserver HOST:PORT   register with a listserver run elsewhere
  --staff NAME         a staff account for RC on every gserver (default admin;
                       repeat for more; --staff none for no account)
  --rc-from RANGE      where staff RCs may log in from (default 127.0.0.1;
                       repeatable; an address, ADDR/BITS or *)
  --bin-dir DIR        where seihai-gserver, -listserver, -npcserver are
                       (repeatable; then next to this program, then PATH).
                       Without seihai-gserver the accounts are not made and
                       the commands to make them are printed instead
  --no-port-check      do not check whether the ports are free on this machine
  --print-plan         print the layout.toml and write nothing

`seihai-servertools guide setup` explains every choice.";

#[derive(Debug, Default)]
struct NewArgs {
    dir: Option<PathBuf>,
    plan: Option<PathBuf>,
    listservers: Option<usize>,
    gservers: Option<usize>,
    npc: Option<NpcMode>,
    public: Option<String>,
    remote: Option<String>,
    staff: Vec<String>,
    rc_from: Vec<String>,
    bin_dirs: Vec<PathBuf>,
    no_port_check: bool,
    print_plan: bool,
    yes: bool,
}

fn parse_new(args: &[&str]) -> Result<NewArgs, String> {
    let mut a = NewArgs::default();
    let mut it = args.iter();
    while let Some(&w) = it.next() {
        let mut value = || {
            it.next()
                .map(|v| v.to_string())
                .ok_or(format!("{w} needs a value"))
        };
        let count = |v: String| {
            v.parse::<usize>()
                .ok()
                .filter(|&n| n <= 32)
                .ok_or(format!("{w} needs a whole number, 0 to 32"))
        };
        match w {
            "--plan" => a.plan = Some(value()?.into()),
            "--listservers" => a.listservers = Some(count(value()?)?),
            "--gservers" => a.gservers = Some(count(value()?)?),
            "--npcserver" => a.npc = Some(NpcMode::parse(&value()?)?),
            "--public" => a.public = Some(value()?),
            "--remote-listserver" => a.remote = Some(value()?),
            "--staff" => a.staff.push(value()?),
            "--rc-from" => a.rc_from.push(value()?),
            "--bin-dir" => a.bin_dirs.push(value()?.into()),
            "--no-port-check" => a.no_port_check = true,
            "--print-plan" => a.print_plan = true,
            "--yes" | "-y" => a.yes = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s if a.dir.is_none() => a.dir = Some(s.into()),
            s => return Err(format!("unexpected {s}")),
        }
    }
    Ok(a)
}

fn staff_names(a: &NewArgs) -> Vec<String> {
    if a.staff.is_empty() {
        vec!["admin".into()]
    } else if a.staff.iter().any(|s| s.eq_ignore_ascii_case("none")) {
        Vec::new()
    } else {
        a.staff.clone()
    }
}

/// The plan from the options alone.
fn plan_from_args(a: &NewArgs) -> Result<Plan, String> {
    if let Some(p) = &a.plan {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        return Plan::from_toml(&text);
    }
    let mut plan = Plan::basic(&Basic {
        listservers: a
            .listservers
            .unwrap_or(if a.remote.is_some() { 0 } else { 1 }),
        gservers: a.gservers.unwrap_or(1),
        npcserver: a.npc.unwrap_or(NpcMode::Auto),
        public_host: a.public.clone(),
        remote_listserver: a.remote.clone(),
        staff: staff_names(a),
    });
    if !a.rc_from.is_empty() {
        for s in &mut plan.staff {
            s.ip_ranges = a.rc_from.clone();
        }
    }
    Ok(plan)
}

fn list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn ask_num(io: &mut Io, q: &str, default: usize) -> Result<usize, String> {
    loop {
        let v = io.ask(q, Some(&default.to_string()))?;
        match v.parse::<usize>() {
            Ok(n) if n <= 32 => return Ok(n),
            _ => io.say("  A whole number, 0 to 32, please."),
        }
    }
}

fn ask_port(io: &mut Io, q: &str, default: u16) -> Result<u16, String> {
    loop {
        let v = io.ask(q, Some(&default.to_string()))?;
        match v.parse::<u16>() {
            Ok(p) if p > 0 => return Ok(p),
            _ => io.say("  A port, 1 to 65535, please."),
        }
    }
}

fn ask_npc(io: &mut Io, default: NpcMode) -> Result<NpcMode, String> {
    loop {
        let v = io.ask("  NPC-server: auto, external or off", Some(default.word()))?;
        match NpcMode::parse(&v) {
            Ok(m) => return Ok(m),
            Err(e) => io.say(&format!("  {e}")),
        }
    }
}

/// The questions, with the options' answers as the defaults.
fn interview(io: &mut Io, a: &NewArgs) -> Result<(PathBuf, Plan), String> {
    io.say("Set up a Seihai server from scratch.");
    io.say("");
    io.say("This writes one folder with everything the servers need: a listserver (the");
    io.say("server list players pick from), one or more gservers (the game worlds), each");
    io.say("pointed at a listserver, an NPC-server for each gserver that runs server-side");
    io.say("scripts, staff accounts for the RC (remote control), and start/stop scripts.");
    io.say("Press Enter to take the answer in [brackets].");
    io.say("");
    let dir = a
        .dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("seihai-server"));
    let dir = PathBuf::from(io.ask("Folder to create", Some(&dir.display().to_string()))?);

    io.say("");
    io.say("Who can join: `lab` listens on 127.0.0.1 (this machine only); `public` listens");
    io.say("on every address, for other machines (open the ports in your firewall).");
    let public = loop {
        let d = if a.public.is_some() { "public" } else { "lab" };
        match io
            .ask("Lab or public", Some(d))?
            .to_ascii_lowercase()
            .as_str()
        {
            "lab" => break None,
            "public" => {
                let host = io.ask(
                    "  The address players use to reach this machine (a name or IP)",
                    a.public.as_deref().or(Some("")),
                )?;
                break Some(host);
            }
            _ => io.say("  lab or public, please."),
        }
    };

    io.say("");
    io.say("Listservers keep the server list. One is enough for most; run none to use a");
    io.say("listserver someone else runs, or to have players connect to a gserver directly.");
    let n_lists = ask_num(
        io,
        "How many listservers to run here",
        a.listservers
            .unwrap_or(if a.remote.is_some() { 0 } else { 1 }),
    )?;
    let remote = if n_lists == 0 {
        let r = io.ask(
            "  A listserver elsewhere, its gserver port as HOST:PORT (empty: none)",
            Some(a.remote.as_deref().unwrap_or("")),
        )?;
        (!r.trim().is_empty()).then_some(r)
    } else {
        None
    };
    let n_gs = loop {
        let n = ask_num(io, "How many gservers", a.gservers.unwrap_or(1))?;
        if n > 0 {
            break n;
        }
        io.say("  At least one, please.");
    };
    let npc = a.npc.unwrap_or(NpcMode::Auto);
    io.say("");
    io.say("Staff accounts log in with seihai-rc to run the server. Each gets a random");
    io.say("password, shown once at the end.");
    let staff_default = staff_names(a).join(",");
    let staff = list(&io.ask(
        "Staff accounts, comma-separated (none for no account)",
        Some(if staff_default.is_empty() {
            "none"
        } else {
            &staff_default
        }),
    )?);
    let staff: Vec<String> = staff
        .into_iter()
        .filter(|s| !s.eq_ignore_ascii_case("none"))
        .collect();
    let rc_default = if a.rc_from.is_empty() {
        "127.0.0.1".to_string()
    } else {
        a.rc_from.join(",")
    };
    let rc_from = if staff.is_empty() {
        Vec::new()
    } else {
        list(&io.ask(
            "  Where their RC may log in from (addresses or ADDR/BITS, comma-separated)",
            Some(&rc_default),
        )?)
    };

    let mut plan = Plan::basic(&Basic {
        listservers: n_lists,
        gservers: n_gs,
        npcserver: npc,
        public_host: public,
        remote_listserver: remote,
        staff,
    });
    for s in &mut plan.staff {
        s.ip_ranges = rc_from.clone();
    }

    io.say("");
    io.say("Each server gets a name and ports; the defaults are the lab ports, stepped up");
    io.say("for each extra server. The gservers take the listservers in turn, and each starts");
    io.say(&format!(
        "its own NPC-server ({}).",
        if npc == NpcMode::Off {
            "none: off"
        } else {
            npc.word()
        }
    ));
    let keep = io.ask("Keep those defaults (yes/no)", Some("yes"))?;
    let detail = !matches!(keep.to_ascii_lowercase().as_str(), "y" | "yes");
    if detail {
        for (i, l) in plan.listservers.iter_mut().enumerate() {
            io.say(&format!("Listserver {}:", i + 1));
            *l = ListserverPlan {
                name: io.ask(
                    "  Name (its folder, and how gservers name it)",
                    Some(&l.name),
                )?,
                title: io.ask("  Title in the status text", Some(&l.title))?,
                gserver_port: ask_port(io, "  Port gservers register on", l.gserver_port)?,
                client_ports: loop {
                    let d = l
                        .client_ports
                        .iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(",");
                    let v = io.ask("  Client ports, comma-separated", Some(&d))?;
                    match list(&v).iter().map(|p| p.parse::<u16>()).collect() {
                        Ok(p) => break p,
                        Err(_) => io.say("  Ports, 1 to 65535, comma-separated, please."),
                    }
                },
                accounts_file: {
                    let d = if l.accounts_file { "file" } else { "open" };
                    io.ask("  Accounts: open (any account gets in) or file", Some(d))?
                        .eq_ignore_ascii_case("file")
                },
                allow_list: {
                    let d = if l.allow_list { "yes" } else { "no" };
                    let a = io.ask(
                        "  Only this layout's gservers may register, each with an HQ password (yes/no)",
                        Some(d),
                    )?;
                    matches!(a.to_ascii_lowercase().as_str(), "y" | "yes")
                },
            };
        }
        let names: Vec<String> = plan.listservers.iter().map(|l| l.name.clone()).collect();
        for (i, g) in plan.gservers.iter_mut().enumerate() {
            io.say(&format!("Gserver {}:", i + 1));
            // The listservers may have been renamed above: the same one in turn.
            let list_default = match &g.listserver {
                ListRef::Local(_) if !names.is_empty() => names[i % names.len()].clone(),
                other => other.word(),
            };
            *g = GserverPlan {
                name: io.ask("  Name (its folder)", Some(&g.name))?,
                title: io.ask("  Name in the server list", Some(&g.title))?,
                port: ask_port(io, "  Port players and RCs connect to", g.port)?,
                listserver: ListRef::parse(&io.ask(
                    &format!(
                        "  Listserver: {}HOST:PORT, or none",
                        if names.is_empty() {
                            String::new()
                        } else {
                            format!("{}, ", names.join(", "))
                        }
                    ),
                    Some(&list_default),
                )?),
                npcserver: ask_npc(io, g.npcserver)?,
                ..g.clone()
            };
            if g.npcserver != NpcMode::Off {
                g.npc_port = ask_port(io, "  NPC-server link port (on 127.0.0.1)", g.npc_port)?;
            }
        }
    }
    Ok((dir, plan))
}

fn show_review(io: &mut Io, dir: &std::path::Path, plan: &Plan) {
    io.say("");
    io.say(&format!("Review: {}", dir.display()));
    for l in &plan.listservers {
        io.say(&format!(
            "  listserver {:<10} gservers register on {}, clients on {}{}{}",
            l.name,
            l.gserver_port,
            l.client_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(","),
            if l.accounts_file {
                ", accounts file"
            } else {
                ", open mode"
            },
            if l.allow_list { ", allow-list" } else { "" }
        ));
    }
    for g in &plan.gservers {
        io.say(&format!(
            "  gserver    {:<10} port {}, listserver {}, NPC-server {}{}",
            g.name,
            g.port,
            g.listserver.word(),
            g.npcserver.word(),
            if g.npcserver == NpcMode::Off {
                String::new()
            } else {
                format!(" (link {})", g.npc_port)
            }
        ));
    }
    for s in &plan.staff {
        let on: Vec<_> = plan.staff_gservers(s).map(|g| g.name.as_str()).collect();
        io.say(&format!(
            "  staff      {:<10} on {}, RC from {}",
            s.account,
            on.join(","),
            if s.ip_ranges.is_empty() {
                "loopback".to_string()
            } else {
                s.ip_ranges.join(",")
            }
        ));
    }
    io.say(&format!(
        "  listening on {}{}",
        plan.bind,
        if plan.public_host.is_empty() {
            String::new()
        } else {
            format!(", listed at {}", plan.public_host)
        }
    ));
}

/// `setup new ...`.
pub fn new(args: &[&str], io: &mut Io) -> Result<ExitCode, String> {
    if matches!(args, ["--help" | "-h"]) {
        io.say(NEW_USAGE);
        return Ok(ExitCode::SUCCESS);
    }
    let a = parse_new(args)?;
    io.yes = a.yes || a.print_plan;
    let (dir, plan) = if io.yes || a.plan.is_some() {
        (
            a.dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("seihai-server")),
            plan_from_args(&a)?,
        )
    } else {
        interview(io, &a)?
    };
    if a.print_plan {
        io.say(&plan.to_toml());
        return Ok(ExitCode::SUCCESS);
    }
    show_review(io, &dir, &plan);
    let findings = layout::check(&plan, !a.no_port_check);
    if !findings.is_empty() {
        io.say("Checks:");
        print_findings(io, &findings);
    }
    if !layout::may_write(&findings) {
        io.say("Nothing written: fix the BLOCKED items first.");
        return Ok(ExitCode::FAILURE);
    }
    let mut dirs = a.bin_dirs.clone();
    if let Some(d) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        dirs.push(d);
    }
    let bins = Bins::find(&dirs);
    let missing = bins.missing(&plan);
    if !missing.is_empty() {
        io.say(&format!(
            "  note: not found here: {} (give --bin-dir, or put them next to this program or on PATH). The folder is written anyway; the start scripts look on PATH or in SEIHAI_BIN_DIR.",
            missing.join(", ")
        ));
    }
    if !io.confirm("Write this layout")? {
        io.say("Nothing written.");
        return Ok(ExitCode::FAILURE);
    }
    let passwords: Vec<String> = plan
        .staff
        .iter()
        .map(|_| layout::random_password())
        .collect::<Result<_, _>>()?;
    let w = layout::write(&plan, &dir, &bins, &passwords)?;
    io.say("");
    io.say(&format!(
        "Wrote {} ({} files).",
        dir.display(),
        w.files.len()
    ));
    for m in &w.made {
        io.say(&format!("  account made: {m}"));
    }
    if !plan.staff.is_empty() {
        io.say("");
        io.say("Staff passwords. They are shown this once and saved nowhere: keep them");
        io.say("in your password manager now. A lost one is reset with `seihai-gserver account`.");
        for (s, p) in plan.staff.iter().zip(&passwords) {
            io.say(&format!("  {:<16} {p}", s.account));
        }
    }
    if !w.todo.is_empty() {
        io.say("");
        io.say("Still to do (a program was not found), with the password above:");
        for t in &w.todo {
            io.say(&format!("  {t}"));
        }
    }
    io.say("");
    io.say("Next:");
    if cfg!(windows) {
        io.say(&format!("  cd {} && start.bat", dir.display()));
    } else {
        io.say(&format!("  cd {} && ./start.sh", dir.display()));
    }
    io.say("  then read README.txt there, and `seihai-servertools guide rc` for the RC.");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_make_the_plan() {
        let a = parse_new(&[
            "out",
            "--gservers",
            "2",
            "--npcserver",
            "external",
            "--staff",
            "boss",
            "--rc-from",
            "10.0.0.0/8",
            "--yes",
        ])
        .unwrap();
        assert_eq!(a.dir, Some(PathBuf::from("out")));
        let p = plan_from_args(&a).unwrap();
        assert_eq!(p.gservers.len(), 2);
        assert!(p.gservers.iter().all(|g| g.npcserver == NpcMode::External));
        assert_eq!(p.staff[0].account, "boss");
        assert_eq!(p.staff[0].ip_ranges, ["10.0.0.0/8"]);
        let r = plan_from_args(
            &parse_new(&["--remote-listserver", "<listserver-host>:14900"]).unwrap(),
        )
        .unwrap();
        assert!(r.listservers.is_empty());
        assert_eq!(
            r.gservers[0].listserver,
            ListRef::Remote("<listserver-host>:14900".into())
        );
        assert!(plan_from_args(&parse_new(&["--staff", "none"]).unwrap())
            .unwrap()
            .staff
            .is_empty());
        assert!(parse_new(&["--gservers", "x"]).is_err());
        assert!(parse_new(&["--npcserver", "x"]).is_err());
    }
}
