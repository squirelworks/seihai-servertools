//! The command line, run as a program.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
}

#[test]
fn version_and_help() {
    let out = bin().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-servertools "));
    let out = bin().arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("lab start"));
}

#[test]
fn lab_plan_prints_the_servers() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::write(
        w.join(format!("gs2emu{}", std::env::consts::EXE_SUFFIX)),
        b"",
    )
    .unwrap();
    let cfg = w.join("servers/default/config");
    fs::create_dir_all(&cfg).unwrap();
    fs::write(
        cfg.join("serveroptions.txt"),
        "serverinterface = 127.0.0.1\nserverport = 14802\n",
    )
    .unwrap();
    let out = bin()
        .args(["lab", "plan", "--no-listserver", "--world"])
        .arg(w)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("gserver     default on 127.0.0.1:14802"),
        "{text}"
    );
}

#[test]
fn check_fails_on_a_broken_level() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bad.nw"), "nope").unwrap();
    let out = bin().arg("check").arg(dir.path()).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("error: bad.nw"));
}

#[test]
fn setup_is_not_built_yet() {
    let out = bin().arg("setup").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
