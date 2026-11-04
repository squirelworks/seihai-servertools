//! `setup_core::lab_run`: a lab plan on a worker thread. The system shell
//! stands in for the servers, so it runs on every CI OS.

use std::time::{Duration, Instant};

use setup_core::lab::{GserverPlan, Plan};
use setup_core::lab_run::{LabRun, Phase};
use setup_core::run::ProcessSpec;

fn shell(label: &str, script: &str, dir: &std::path::Path) -> GserverPlan {
    let (program, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    GserverPlan {
        name: label.into(),
        interface: None,
        port: None,
        spec: ProcessSpec {
            label: label.into(),
            program: program.into(),
            args: vec![flag.into(), script.into()],
            cwd: dir.to_path_buf(),
            console: dir.join("logs").join(format!("{label}.txt")),
        },
    }
}

const LONG: &str = if cfg!(windows) {
    "echo up&& ping -n 60 127.0.0.1 > nul"
} else {
    "echo up; sleep 60"
};

fn wait_for(run: &LabRun, limit: Duration, done: impl Fn(&Phase) -> bool) -> Phase {
    let deadline = Instant::now() + limit;
    loop {
        let phase = run.status().phase;
        if done(&phase) || Instant::now() > deadline {
            return phase;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn starts_runs_shows_consoles_and_stops() {
    let dir = tempfile::tempdir().unwrap();
    let plan = Plan {
        listserver: None,
        gservers: vec![
            shell("default", LONG, dir.path()),
            shell("classic", LONG, dir.path()),
        ],
        notes: vec!["a note".into()],
    };
    let run = LabRun::start(plan);
    let phase = wait_for(&run, Duration::from_secs(15), |p| *p != Phase::Starting);
    assert_eq!(phase, Phase::Running);
    let s = run.status();
    assert_eq!(s.procs.len(), 2);
    assert!(s.procs.iter().all(|p| p.exited.is_none()));
    assert_eq!(s.log[0], "note: a note");
    assert!(s.log.iter().any(|l| l.starts_with("started classic, pid ")));
    let tails = run.tails(5);
    assert_eq!(tails[0].0, "default");
    assert!(tails.iter().all(|(_, t)| t.contains("up")), "{tails:?}");

    let t = Instant::now();
    run.stop();
    let phase = wait_for(&run, Duration::from_secs(20), |p| *p == Phase::Stopped);
    assert_eq!(phase, Phase::Stopped);
    assert!(t.elapsed() < Duration::from_secs(20));
    assert!(run.status().procs.iter().all(|p| p.exited.is_some()));
    std::thread::sleep(Duration::from_millis(100));
    assert!(run.is_finished());
}

#[test]
fn a_server_that_dies_at_once_fails_the_start_and_stops_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let plan = Plan {
        listserver: None,
        gservers: vec![
            shell("default", LONG, dir.path()),
            shell("classic", "echo port in use&& exit 3", dir.path()),
        ],
        notes: Vec::new(),
    };
    let run = LabRun::start(plan);
    let phase = wait_for(&run, Duration::from_secs(20), |p| {
        matches!(p, Phase::Failed(_))
    });
    let Phase::Failed(why) = phase else {
        panic!("expected a failed start, got {phase:?}");
    };
    assert!(why.contains("classic stopped"), "{why}");
    assert!(why.contains("port in use"), "{why}");
    assert!(
        run.status().procs.iter().all(|p| p.exited.is_some()),
        "default was stopped too"
    );
}

#[test]
fn a_program_that_cannot_start_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut g = shell("default", LONG, dir.path());
    g.spec.program = dir.path().join("no-such-gserver");
    let run = LabRun::start(Plan {
        listserver: None,
        gservers: vec![g],
        notes: Vec::new(),
    });
    let phase = wait_for(&run, Duration::from_secs(10), |p| {
        matches!(p, Phase::Failed(_))
    });
    assert!(
        matches!(&phase, Phase::Failed(w) if w.contains("no-such-gserver")),
        "{phase:?}"
    );
}

#[test]
fn dropping_the_run_stops_the_servers() {
    let dir = tempfile::tempdir().unwrap();
    let run = LabRun::start(Plan {
        listserver: None,
        gservers: vec![shell("default", LONG, dir.path())],
        notes: Vec::new(),
    });
    wait_for(&run, Duration::from_secs(15), |p| *p == Phase::Running);
    let t = Instant::now();
    drop(run);
    assert!(t.elapsed() < Duration::from_secs(20));
}
