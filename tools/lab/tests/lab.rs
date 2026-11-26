//! `seihai-lab`, run as a program on its own.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-lab"))
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
        .args(["plan", "--no-listserver", "--world"])
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

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-lab"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-lab "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-lab"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("start"), "{text}");
}
