//! `seihai-lab start|plan` (also `seihai-servertools lab`): the one-step lab start.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use setup_core::lab::{plan, LabOptions, ListserverChoice};
use setup_core::run::{start, tail, Group};

fn parse(args: &[&str]) -> Result<LabOptions, String> {
    let mut opts = LabOptions {
        world: PathBuf::from("."),
        ..Default::default()
    };
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .map(|v| v.to_string())
                .ok_or(format!("{name} needs a value"))
        };
        match a {
            "--world" => opts.world = value(a)?.into(),
            "--listserver" => opts.listserver = ListserverChoice::Path(value(a)?.into()),
            "--no-listserver" => opts.listserver = ListserverChoice::Off,
            "--gserver" => opts.gserver = Some(value(a)?.into()),
            "--rust" => opts.rust_worlds.push(value(a)?.into()),
            "--rust-gserver" => opts.rust_gserver = Some(value(a)?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => opts.servers.push(s.to_string()),
        }
    }
    Ok(opts)
}

/// Wait until something accepts on `127.0.0.1:port`, up to `limit`.
fn wait_for_port(port: u16, limit: Duration) -> bool {
    setup_core::lab_run::wait_for_port(port, limit, &AtomicBool::new(false))
}

fn report_stopped(group: &mut Group) -> bool {
    let mut any = false;
    for r in &mut group.procs {
        if let Ok(Some(status)) = r.poll() {
            any = true;
            eprintln!(
                "{} stopped ({status}). The end of {}:",
                r.spec.label,
                r.spec.console.display()
            );
            eprintln!("{}", tail(&r.spec.console, 15));
        }
    }
    any
}

/// Help for `seihai-lab` (`seihai-servertools lab`).
pub const USAGE: &str = "\
lab start [--world DIR] [--listserver PATH | --no-listserver]
          [--gserver PATH] [--rust DIR]... [--rust-gserver PATH] [SERVER...]
lab plan  (same options; prints what start would run)

start: starts the listserver (if one is found) and then the gservers of a
lab world (default: servers/default, classic and modern, those that exist),
each console in a log file, and stops them all on Ctrl+C. --world defaults
to the current folder. --rust DIR also starts seihai-gserver on that world
folder (config/server.txt; relative to --world), found beside the world,
this tool or on PATH unless --rust-gserver is given. With --rust and no
SERVER named, the fork's folders start too when gs2emu is there.";

/// Runs one command line: the words after `seihai-lab` (or after
/// `seihai-servertools lab`): `start ...` or `plan ...`.
pub fn run(args: &[&str]) -> ExitCode {
    match args {
        ["start", rest @ ..] => start_or_plan(rest, true),
        ["plan", rest @ ..] => start_or_plan(rest, false),
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn start_or_plan(args: &[&str], really: bool) -> ExitCode {
    let opts = match parse(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("lab: {e}");
            return ExitCode::from(2);
        }
    };
    let plan = match plan(&opts) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("lab: {e}");
            return ExitCode::FAILURE;
        }
    };
    print!("{plan}");
    if !really {
        return ExitCode::SUCCESS;
    }

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        if let Err(e) = ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst)) {
            eprintln!("lab: cannot catch Ctrl+C: {e}");
            return ExitCode::FAILURE;
        }
    }

    let mut group = Group::default();
    if let Some(l) = &plan.listserver {
        match start(&l.spec) {
            Ok(r) => {
                println!("started listserver, pid {}", r.id());
                group.push(r);
            }
            Err(e) => {
                eprintln!("lab: {e}");
                return ExitCode::FAILURE;
            }
        }
        if l.settings.is_loopback()
            && !wait_for_port(l.settings.gserver_port, Duration::from_secs(5))
        {
            eprintln!(
                "lab: the listserver is not accepting on 127.0.0.1:{} after 5 s",
                l.settings.gserver_port
            );
            report_stopped(&mut group);
            return ExitCode::FAILURE;
        }
    }
    for g in &plan.gservers {
        match start(&g.spec) {
            Ok(r) => {
                println!("started {}, pid {}", g.name, r.id());
                group.push(r);
            }
            Err(e) => {
                eprintln!("lab: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    // A server that dies straight away (port in use, bad config) is
    // reported with the end of its console, like start.sh does.
    thread::sleep(Duration::from_secs(2));
    if report_stopped(&mut group) {
        eprintln!("lab: stopping the rest");
        return ExitCode::FAILURE;
    }
    println!("Lab world running. Stop with Ctrl+C.");
    let by_signal = group.wait(&stop);
    if !by_signal {
        report_stopped(&mut group);
    }
    println!("Stopping...");
    group.stop_all(Duration::from_secs(5));
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options() {
        let o = parse(&["--world", "w", "--no-listserver", "classic"]).unwrap();
        assert_eq!(o.world, PathBuf::from("w"));
        assert_eq!(o.listserver, ListserverChoice::Off);
        assert_eq!(o.servers, ["classic"]);
        assert!(parse(&["--world"]).is_err());
        let o = parse(&["--rust", "rw", "--rust", "/abs/rw2", "--rust-gserver", "sg"]).unwrap();
        assert_eq!(
            o.rust_worlds,
            [PathBuf::from("rw"), PathBuf::from("/abs/rw2")]
        );
        assert_eq!(o.rust_gserver, Some(PathBuf::from("sg")));
        assert!(o.servers.is_empty());
        assert!(parse(&["--bogus"]).is_err());
    }
}
