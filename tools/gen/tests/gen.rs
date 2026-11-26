//! `seihai-gen`, run as a program on its own.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gen"))
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
fn terrain_3d_and_dungeon_worlds_are_written_and_kept() {
    let d = tempfile::tempdir().unwrap();
    run(
        &[
            "terrain",
            "t",
            "--world",
            "isle",
            "--size",
            "2x1",
            "--islands",
        ],
        d.path(),
    );
    let gmap = fs::read_to_string(d.path().join("t/isle.gmap")).unwrap();
    assert!(gmap.contains("\"isleA00.nw\",\"isleB00.nw\","), "{gmap}");
    let lvl = fs::read_to_string(d.path().join("t/isleA00.nw")).unwrap();
    assert!(lvl.contains("HEIGHTS\n") && lvl.contains("LINK isleB00.nw 63 0 1 64 0 playery"));
    let again = Command::new(env!("CARGO_BIN_EXE_seihai-gen"))
        .args(["terrain", "t", "--world", "isle", "--size", "2x1"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );

    run(
        &[
            "terrain", "t3", "--world", "isle", "--size", "2x2", "--3d", "--trees", "8",
        ],
        d.path(),
    );
    run(&["3d", "t3/isle.gmap", "from", "--trees", "8"], d.path());
    assert_eq!(
        fs::read(d.path().join("from/isle.3ds")).unwrap(),
        fs::read(d.path().join("t3/isle.3ds")).unwrap()
    );

    fs::write(d.path().join("l.txt"), "XXXXXXXXX\nX.......X\nXXXXXXXXX\n").unwrap();
    run(
        &["dungeon", "d", "--start", "cave_", "--layout", "l.txt"],
        d.path(),
    );
    let gmap = fs::read_to_string(d.path().join("d/cave_.gmap")).unwrap();
    assert!(gmap.starts_with("GRMAP001"), "{gmap}");
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gen"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-gen "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gen"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("terrain"), "{text}");
}
