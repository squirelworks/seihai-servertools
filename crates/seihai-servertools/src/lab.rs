//! `seihai-servertools lab start|plan`: the one-step lab start.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => opts.servers.push(s.to_string()),
        }
    }
    Ok(opts)
}

/// Wait until something accepts on `127.0.0.1:port`, up to `limit`.
fn wait_for_port(port: u16, limit: Duration) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
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

pub fn run(args: &[&str], really: bool) -> ExitCode {
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
        assert!(parse(&["--bogus"]).is_err());
    }
}
