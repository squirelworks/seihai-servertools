//! `seihai-servertools setup`: the go-public wizard on the command line, for
//! servers with no screen. The same steps as the window, as prompts; every
//! rule and every write goes through `setup-core`.
//!
//! ```text
//! setup status [--world DIR] [SERVER]
//! setup public [--world DIR] [SERVER] [answers...] [--yes]   (also: --public)
//! setup lab    [--world DIR] [SERVER] [--yes]                (also: --lab)
//! setup undo   [--world DIR] [SERVER]
//! ```
//!
//! A value not given as an option is asked for on standard input. With
//! `--yes` nothing is asked: a missing required value is an error, and the
//! review is saved without asking (a blocking finding still stops it).

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use setup_core::checks::{check_server_folder, Finding, Severity};
use setup_core::folder::{Preview, ServerFolder};
use setup_core::profile::{Profile, PublicAnswers};

pub const USAGE: &str = "\
seihai-servertools setup: take a gserver folder from the lab to the public and back

  setup status [--world DIR] [SERVER]
  setup public [--world DIR] [SERVER] [--listserver HOST[:PORT]] [--account NAME]
               [--name TEXT] [--description TEXT] [--url TEXT]
               [--interface AUTO|ADDR] [--server-ip AUTO|ADDR] [--upnp] [--yes]
  setup lab    [--world DIR] [SERVER] [--yes]      Back to lab
  setup undo   [--world DIR] [SERVER]              restore the last backup
  setup start  [--world DIR] [SERVER] [--gserver PATH] [--for SECS]
               start the server and show the three lights; Ctrl+C stops it
  setup test-connection HOST[:PORT]               one TCP connect to that address

SERVER defaults to `default` (servers/default). --world defaults to the
current folder. Values not given are asked for. Every save keeps a dated
backup of serveroptions.txt next to it.";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Args {
    world: Option<PathBuf>,
    server: Option<String>,
    listserver: Option<String>,
    account: Option<String>,
    name: Option<String>,
    description: Option<String>,
    url: Option<String>,
    interface: Option<String>,
    server_ip: Option<String>,
    upnp: bool,
    yes: bool,
    gserver: Option<PathBuf>,
    for_secs: Option<u64>,
}

fn parse(args: &[&str]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = args.iter();
    while let Some(&w) = it.next() {
        let mut value = || {
            it.next()
                .map(|v| v.to_string())
                .ok_or(format!("{w} needs a value"))
        };
        match w {
            "--world" => a.world = Some(value()?.into()),
            "--listserver" => a.listserver = Some(value()?),
            "--account" => a.account = Some(value()?),
            "--name" => a.name = Some(value()?),
            "--description" => a.description = Some(value()?),
            "--url" => a.url = Some(value()?),
            "--interface" => a.interface = Some(value()?),
            "--server-ip" => a.server_ip = Some(value()?),
            "--upnp" => a.upnp = true,
            "--gserver" => a.gserver = Some(value()?.into()),
            "--for" => {
                a.for_secs = Some(value()?.parse().map_err(|_| "--for needs whole seconds")?)
            }
            "--yes" | "-y" => a.yes = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s if a.server.is_none() => a.server = Some(s.to_string()),
            s => return Err(format!("unexpected {s}")),
        }
    }
    Ok(a)
}

/// Prompts on the given input and output, so tests can drive them.
struct Io<'a> {
    input: &'a mut dyn BufRead,
    out: &'a mut dyn Write,
    yes: bool,
}

impl Io<'_> {
    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }

    /// Ask for a value; `default` is used for an empty answer.
    fn ask(&mut self, question: &str, default: Option<&str>) -> Result<String, String> {
        if self.yes {
            return default.map(str::to_string).ok_or(format!(
                "{question}: no value given (and --yes asks nothing)"
            ));
        }
        match default {
            Some(d) if !d.is_empty() => write!(self.out, "{question} [{d}]: "),
            _ => write!(self.out, "{question}: "),
        }
        .and_then(|_| self.out.flush())
        .map_err(|e| e.to_string())?;
        let mut line = String::new();
        if self.input.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err(format!("{question}: no answer (end of input)"));
        }
        let line = line.trim();
        if line.is_empty() {
            default
                .map(str::to_string)
                .ok_or(format!("{question}: an answer is needed"))
        } else {
            Ok(line.to_string())
        }
    }

    fn confirm(&mut self, question: &str) -> Result<bool, String> {
        if self.yes {
            return Ok(true);
        }
        let a = self.ask(&format!("{question} (yes/no)"), Some("no"))?;
        Ok(matches!(a.to_ascii_lowercase().as_str(), "y" | "yes"))
    }
}

fn print_findings(io: &mut Io, findings: &[Finding]) {
    for f in findings {
        let level = match f.severity {
            Severity::Block => "BLOCKED",
            Severity::Warn => "warning",
            Severity::Info => "note",
        };
        let key = f
            .key
            .as_deref()
            .map(|k| format!(" [{k}]"))
            .unwrap_or_default();
        io.say(&format!("  {level}{key}: {}", f.text));
    }
}

fn open_folder(a: &Args) -> Result<ServerFolder, String> {
    let world = a.world.clone().unwrap_or_else(|| PathBuf::from("."));
    let name = a.server.clone().unwrap_or_else(|| "default".into());
    ServerFolder::open(&world, &name)
}

fn profile_name(p: Profile) -> &'static str {
    match p {
        Profile::Lab => "Lab (loopback only)",
        Profile::Public => "Public",
        Profile::HostListserver => "Hosting a listserver",
    }
}

/// Show the before/after, check, confirm and save. Returns the exit code.
fn review_and_save(
    io: &mut Io,
    f: &mut ServerFolder,
    preview: &Preview,
) -> Result<ExitCode, String> {
    io.say("");
    io.say("Review:");
    if preview.diff.is_empty() {
        io.say("  nothing to change.");
        return Ok(ExitCode::SUCCESS);
    }
    for (key, before, now) in &preview.diff {
        io.say(&format!(
            "  {key}: {} -> {}",
            before.as_deref().unwrap_or("(not set)"),
            now.as_deref().unwrap_or("(removed)")
        ));
    }
    if !preview.findings.is_empty() {
        io.say("Checks:");
        print_findings(io, &preview.findings);
    }
    if !preview.may_save() {
        io.say("Not saved: fix the BLOCKED items first.");
        return Ok(ExitCode::FAILURE);
    }
    if !io.confirm("Save these changes")? {
        io.say("Not saved.");
        return Ok(ExitCode::FAILURE);
    }
    let backup = f.save(preview)?;
    io.say(&format!("Saved {}.", f.options_path.display()));
    if let Some(b) = backup {
        io.say(&format!(
            "Backup: {} (undo with `setup undo`).",
            b.display()
        ));
    }
    Ok(ExitCode::SUCCESS)
}

fn status(io: &mut Io, a: &Args) -> Result<ExitCode, String> {
    let f = open_folder(a)?;
    io.say(&format!(
        "{}: {}",
        f.dir.display(),
        profile_name(f.profile())
    ));
    for key in [
        "serverinterface",
        "serverip",
        "localip",
        "serverport",
        "listip",
        "listport",
        "upnp",
        "localhost",
        "name",
    ] {
        io.say(&format!(
            "  {key} = {}",
            f.options.get(key).unwrap_or("(not set)")
        ));
    }
    print_findings(io, &f.status_findings());
    Ok(ExitCode::SUCCESS)
}

/// Split `HOST[:PORT]`; the port defaults to 14900. IPv6 needs brackets.
fn split_host_port(s: &str) -> Result<(String, u16), String> {
    let s = s.trim();
    let (host, port) = if let Some(rest) = s.strip_prefix('[') {
        let (h, p) = rest.split_once(']').ok_or("missing ] in the address")?;
        (h.to_string(), p.strip_prefix(':'))
    } else if s.matches(':').count() == 1 {
        let (h, p) = s.split_once(':').unwrap_or((s, ""));
        (h.to_string(), Some(p))
    } else {
        (s.to_string(), None)
    };
    let port = match port {
        Some(p) => p.parse().map_err(|_| format!("bad port {p}"))?,
        None => 14900,
    };
    if host.is_empty() {
        return Err("the listserver address is empty".into());
    }
    Ok((host, port))
}

fn public(io: &mut Io, a: &Args) -> Result<ExitCode, String> {
    let mut f = open_folder(a)?;
    io.say(&format!(
        "Join a listserver: {} is now {}.",
        f.dir.display(),
        profile_name(f.profile())
    ));

    io.say("");
    io.say("1. Safety check");
    let folder = check_server_folder(&f.dir);
    if folder.is_empty() {
        io.say("  nothing to report.");
    }
    print_findings(io, &folder);
    let current_staff = f.options.get("staff").unwrap_or("").to_string();
    let account = match &a.account {
        Some(v) => v.clone(),
        None => io.ask(
            &format!("Your account, for the staff list (now: {current_staff})"),
            None,
        )?,
    };

    io.say("");
    io.say("2. Server details");
    let cur = |k: &str| f.options.get(k).unwrap_or("").to_string();
    let name = match &a.name {
        Some(v) => v.clone(),
        None => io.ask("Server name", Some(&cur("name")))?,
    };
    let description = match &a.description {
        Some(v) => v.clone(),
        None => io.ask("Description", Some(&cur("description")))?,
    };
    let url = match &a.url {
        Some(v) => v.clone(),
        None => io.ask("Web page (optional)", Some(&cur("url")))?,
    };

    io.say("");
    io.say("3. Network");
    let interface = match &a.interface {
        Some(v) => v.clone(),
        None => io.ask(
            "Listen on: AUTO (every interface) or one of this machine's addresses",
            Some("AUTO"),
        )?,
    };
    let server_ip = match &a.server_ip {
        Some(v) => v.clone(),
        None => io.ask(
            "Address players use: AUTO (the listserver decides) or an address",
            Some("AUTO"),
        )?,
    };
    io.say(&format!(
        "  Port {} must be free and reachable from outside.",
        f.options.get("serverport").unwrap_or("(serverport)")
    ));

    io.say("");
    io.say("4. Listserver");
    let list = match &a.listserver {
        Some(v) => v.clone(),
        None => io.ask(
            "Listserver address, HOST or HOST:PORT (<listserver-host>)",
            None,
        )?,
    };
    let (list_host, list_port) = split_host_port(&list)?;
    io.say("  localhost will be set to false: logins are then checked by the listserver.");
    io.say("  Leaving it on would let anyone log in as anyone.");

    io.say("");
    io.say("5. Router");
    let upnp =
        a.upnp || (!io.yes && io.confirm("Open the port on the router automatically (UPnP)")?);
    if !upnp {
        io.say(&format!(
            "  Forward TCP port {} on your router to this machine's LAN address, and allow it in the firewall.",
            f.options.get("serverport").unwrap_or("(serverport)")
        ));
    }

    let answers = PublicAnswers {
        interface,
        server_ip,
        list_host,
        list_port,
        upnp,
        name,
        description,
        url,
        staff_account: account,
    };
    let preview = f.preview_public(&answers);
    io.say("");
    io.say("6. Review");
    let code = review_and_save(io, &mut f, &preview)?;
    if code == ExitCode::SUCCESS {
        io.say("Next: `seihai-servertools setup start` starts it and shows whether it registers.");
    }
    Ok(code)
}

fn lab(io: &mut Io, a: &Args) -> Result<ExitCode, String> {
    let mut f = open_folder(a)?;
    io.say(&format!("Back to lab: {}", f.dir.display()));
    let preview = f.preview_lab();
    review_and_save(io, &mut f, &preview)
}

fn undo(io: &mut Io, a: &Args) -> Result<ExitCode, String> {
    let mut f = open_folder(a)?;
    let restored = f.undo()?;
    io.say(&format!(
        "Restored {} from {} (the replaced file was kept as a backup).",
        f.options_path.display(),
        restored.display()
    ));
    Ok(ExitCode::SUCCESS)
}

/// Start the server folder's gserver and show the lights until Ctrl+C or
/// `--for` seconds, then stop it.
fn start_and_check(io: &mut Io, a: &Args) -> Result<ExitCode, String> {
    use setup_core::lab::exe_name;
    use setup_core::lights::Lights;
    use setup_core::run::{start, tail, Group, ProcessSpec};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let f = open_folder(a)?;
    let world = a.world.clone().unwrap_or_else(|| PathBuf::from("."));
    let name = a.server.clone().unwrap_or_else(|| "default".into());
    let program = a
        .gserver
        .clone()
        .unwrap_or_else(|| world.join(exe_name("gs2emu")));
    if !program.is_file() {
        return Err(format!(
            "no gserver at {} (give --gserver)",
            program.display()
        ));
    }
    let public = f.profile() != Profile::Lab;
    let spec = ProcessSpec {
        label: name.clone(),
        program,
        args: vec!["--server".into(), name.clone()],
        cwd: world,
        console: f.console_path(),
    };
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        let _ = ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst));
    }
    let mut group = Group::default();
    group.push(start(&spec).map_err(|e| e.to_string())?);
    io.say(&format!(
        "Started {name}; console in {}. Stop with Ctrl+C.",
        spec.console.display()
    ));
    let deadline = a.for_secs.map(|s| Instant::now() + Duration::from_secs(s));
    let mut shown = String::new();
    let mut code = ExitCode::SUCCESS;
    loop {
        let text = std::fs::read_to_string(&spec.console).unwrap_or_default();
        let lights = Lights::read(&text, public).to_string();
        if lights != shown {
            io.say("");
            io.say(&lights);
            shown = lights;
        }
        if let Some((_, status)) = group.stopped().into_iter().next() {
            io.say(&format!(
                "{name} stopped ({status}). The end of its console:"
            ));
            io.say(&tail(&spec.console, 15));
            code = ExitCode::FAILURE;
            break;
        }
        if stop.load(Ordering::SeqCst) || deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    io.say("Stopping...");
    group.stop_all(Duration::from_secs(5));
    Ok(code)
}

fn test_connection(io: &mut Io, target: &str) -> Result<ExitCode, String> {
    let (host, port) = split_host_port(target)?;
    let reach = setup_core::net::test_connection(&host, port, std::time::Duration::from_secs(5));
    io.say(&format!("{host}:{port}: {reach}"));
    Ok(if reach == setup_core::net::Reach::Open {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Run `setup` with the process's standard input and output.
pub fn run(args: &[&str]) -> ExitCode {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut out = io::stdout();
    run_with(args, &mut input, &mut out)
}

fn run_with(args: &[&str], input: &mut dyn BufRead, out: &mut dyn Write) -> ExitCode {
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (*c, r),
        None => {
            let _ = writeln!(out, "{USAGE}");
            return ExitCode::from(2);
        }
    };
    if cmd == "test-connection" {
        let mut io = Io {
            input,
            out,
            yes: true,
        };
        return match rest {
            [target] => test_connection(&mut io, target).unwrap_or_else(|e| {
                eprintln!("setup: {e}");
                ExitCode::FAILURE
            }),
            _ => {
                eprintln!("setup test-connection HOST[:PORT]");
                ExitCode::from(2)
            }
        };
    }
    let a = match parse(rest) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("setup: {e}");
            return ExitCode::from(2);
        }
    };
    let mut io = Io {
        input,
        out,
        yes: a.yes,
    };
    let result = match cmd {
        "status" => status(&mut io, &a),
        "public" | "--public" => public(&mut io, &a),
        "lab" | "--lab" => lab(&mut io, &a),
        "undo" => undo(&mut io, &a),
        "start" => start_and_check(&mut io, &a),
        "--host-listserver" | "host-listserver" => {
            Err("Host a listserver comes in setup step 6 (docs/NEXT.md)".into())
        }
        "--help" | "-h" | "help" => {
            io.say(USAGE);
            Ok(ExitCode::SUCCESS)
        }
        other => Err(format!(
            "unknown setup command {other}; see `seihai-servertools setup --help`"
        )),
    };
    result.unwrap_or_else(|e| {
        eprintln!("setup: {e}");
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_port() {
        assert_eq!(
            split_host_port("127.0.0.1").unwrap(),
            ("127.0.0.1".into(), 14900)
        );
        assert_eq!(
            split_host_port("127.0.0.1:15000").unwrap(),
            ("127.0.0.1".into(), 15000)
        );
        assert_eq!(
            split_host_port("[::1]:15000").unwrap(),
            ("::1".into(), 15000)
        );
        assert_eq!(split_host_port("::1").unwrap(), ("::1".into(), 14900));
        assert!(split_host_port("127.0.0.1:x").is_err());
        assert!(split_host_port("").is_err());
    }

    #[test]
    fn options() {
        let a = parse(&["classic", "--world", "w", "--yes", "--upnp"]).unwrap();
        assert_eq!(a.server.as_deref(), Some("classic"));
        assert!(a.yes && a.upnp);
        assert!(parse(&["a", "b"]).is_err());
    }
}
