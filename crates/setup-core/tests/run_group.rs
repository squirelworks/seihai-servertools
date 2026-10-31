//! Start, watch and stop processes with `setup_core::run`. Uses the
//! system shell as a stand-in server, so it runs on every CI OS.

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use setup_core::run::{start, tail, Group, ProcessSpec};

fn shell(label: &str, script: &str, dir: &std::path::Path) -> ProcessSpec {
    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    ProcessSpec {
        label: label.into(),
        program: program.into(),
        args: vec![flag.into(), script.into()],
        cwd: dir.to_path_buf(),
        console: dir.join("logs").join(format!("{label}.txt")),
    }
}

#[test]
fn early_exit_is_seen_with_its_console() {
    let dir = tempfile::tempdir().unwrap();
    let spec = shell("boom", "echo port in use&& exit 3", dir.path());
    let mut group = Group::default();
    group.push(start(&spec).unwrap());
    let stop = AtomicBool::new(false);
    assert!(!group.wait(&stop), "wait ends when every process stopped");
    let stopped = group.stopped();
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0].1.code(), Some(3));
    assert!(tail(&spec.console, 5).contains("port in use"));
}

#[test]
fn stop_all_ends_long_running_processes() {
    let dir = tempfile::tempdir().unwrap();
    let long = if cfg!(windows) {
        "ping -n 60 127.0.0.1 > nul"
    } else {
        "sleep 60"
    };
    let mut group = Group::default();
    group.push(start(&shell("a", long, dir.path())).unwrap());
    group.push(start(&shell("b", long, dir.path())).unwrap());
    assert!(group.stopped().is_empty());
    let t = Instant::now();
    group.stop_all(Duration::from_secs(5));
    assert_eq!(group.stopped().len(), 2);
    assert!(t.elapsed() < Duration::from_secs(20));
}

#[test]
fn missing_program_is_an_error_naming_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = shell("x", "", dir.path());
    spec.program = dir.path().join("no-such-program");
    let err = start(&spec).err().expect("cannot start").to_string();
    assert!(err.contains("no-such-program"), "{err}");
}
