//! `seihai-gpak`, run as a program on its own.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-gpak"))
}

/// `gpak pack --server` puts a `.gpake` under `world/gpaks/` and its key in
/// `config/gpakkeys.txt`; `gpak unpack --server` gets the files back.
#[test]
fn gpak_pack_and_unpack_through_a_server_folder() {
    let src = tempfile::tempdir().unwrap();
    fs::create_dir_all(src.path().join("tiles")).unwrap();
    fs::write(src.path().join("tiles/Tile.png"), vec![7u8; 100]).unwrap();
    fs::write(src.path().join("note.txt"), b"hello").unwrap();
    let world = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["pack"])
        .arg(src.path())
        .args(["art.gpake", "--server"])
        .arg(world.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pkg = world.path().join("world/gpaks/art.gpake");
    assert!(pkg.is_file());
    let keys = fs::read_to_string(world.path().join("config/gpakkeys.txt")).unwrap();
    assert!(keys.lines().any(|l| l.starts_with("art.gpake ")), "{keys}");
    let back = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["unpack"])
        .arg(&pkg)
        .arg(back.path())
        .arg("--server")
        .arg(world.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(back.path().join("tiles/tile.png")).unwrap(),
        vec![7u8; 100]
    );
    assert_eq!(fs::read(back.path().join("note.txt")).unwrap(), b"hello");
    // Without the key, an encrypted package is refused.
    let out = bin()
        .args(["unpack"])
        .arg(&pkg)
        .arg(back.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gpak"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-gpak "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gpak"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("pack"), "{text}");
}
