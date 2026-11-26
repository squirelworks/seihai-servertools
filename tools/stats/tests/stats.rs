//! `seihai-stats`, run as a program on its own.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-stats"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

#[test]
fn stats_pages_rank_accounts_without_passwords() {
    let d = tempfile::tempdir().unwrap();
    let acc = d.path().join("accounts");
    fs::create_dir(&acc).unwrap();
    fs::write(
        acc.join("alice.txt"),
        "SEIHAIACC 1\nPASSWORD pbkdf2-sha256:1:aa:bb\nNICK Alice\nKILLS 5\n",
    )
    .unwrap();
    fs::write(acc.join("bob.txt"), "SEIHAIACC 1\nNICK Bob\nKILLS 9\n").unwrap();
    run(&["accounts", "out", "kills"], d.path());
    let html = fs::read_to_string(d.path().join("out/kills.html")).unwrap();
    assert!(html.find("Bob").unwrap() < html.find("Alice").unwrap());
    assert!(!html.contains("pbkdf2") && !html.contains("alice"));
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-stats"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-stats "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-stats"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("ACCOUNTS"), "{text}");
}
