//! `level convert|render` and `gmap make|render`, run as a program.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
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

/// A GR-V1.03 level written by hand from `formats/level-binary.md`: 13-bit
/// codes, low bit first; a code with bit 12 set is a run of `code & 0xFF`
/// copies of the next tile.
fn graal_level(tile: u16, link: &str) -> Vec<u8> {
    let mut codes = Vec::new();
    let mut left = 4096u32;
    while left > 0 {
        let n = left.min(255);
        codes.push(0x1000 | n as u16);
        codes.push(tile);
        left -= n;
    }
    let mut out = b"GR-V1.03".to_vec();
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in codes {
        buf |= u32::from(c) << bits;
        bits += 13;
        while bits >= 8 {
            out.push(buf as u8);
            buf >>= 8;
            bits -= 8;
        }
    }
    if bits > 0 {
        out.push(buf as u8);
    }
    out.extend_from_slice(link.as_bytes());
    out.extend_from_slice(b"\n#\n\xff\xff\xff\n#\n#\n");
    out
}

fn png_size(path: &Path) -> (u32, u32) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn a_graal_level_converts_to_nw_and_renders() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("a.graal"),
        graal_level(5, "b.graal 63 0 1 64 0 playery"),
    )
    .unwrap();
    let out = run(&["level", "convert", "a.graal", "a.nw"], d.path());
    assert!(String::from_utf8_lossy(&out.stdout).contains("1 links"));
    let nw = fs::read_to_string(d.path().join("a.nw")).unwrap();
    assert!(nw.starts_with("GLEVNW01\n"));
    assert!(nw.contains("LINK b.graal 63 0 1 64 0 playery"), "{nw}");
    // Tile 5 is "AF" in GLEVNW01's base-64 pairs.
    assert!(nw.contains("BOARD 0 0 64 0 AFAF"), "{nw}");

    run(
        &["level", "render", "a.nw", "a.png", "--scale", "16"],
        d.path(),
    );
    assert_eq!(png_size(&d.path().join("a.png")), (64, 64));

    let refused = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(["level", "convert", "a.nw", "b.graal"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("not done yet"));
}

#[test]
fn a_linked_world_becomes_a_gmap_and_renders() {
    let d = tempfile::tempdir().unwrap();
    let w = d.path().join("world");
    fs::create_dir(&w).unwrap();
    fs::write(
        w.join("a.graal"),
        graal_level(1, "B.graal 63 0 1 64 0 playery"),
    )
    .unwrap();
    fs::write(
        w.join("b.graal"),
        graal_level(2, "a.graal 0 0 1 64 61 playery"),
    )
    .unwrap();
    let out = run(
        &["gmap", "make", "w.gmap", "--links", "world", "a.graal"],
        d.path(),
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("2 by 1, 2 levels"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let gmap = fs::read_to_string(d.path().join("w.gmap")).unwrap();
    assert!(gmap.contains("\"a.graal\",\"B.graal\","), "{gmap}");

    let out = run(
        &[
            "gmap", "render", "w.gmap", "w.png", "--dir", "world", "--scale", "16",
        ],
        d.path(),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("2 levels painted"));
    assert_eq!(png_size(&d.path().join("w.png")), (128, 64));

    fs::write(w.join("w-a01.nw"), "GLEVNW01\n").unwrap();
    fs::write(w.join("w-b02.nw"), "GLEVNW01\n").unwrap();
    run(&["gmap", "make", "n.gmap", "--names", "world"], d.path());
    let gmap = fs::read_to_string(d.path().join("n.gmap")).unwrap();
    assert!(gmap.contains("WIDTH 2\nHEIGHT 2\n"), "{gmap}");
    run(
        &[
            "gmap", "make", "l.gmap", "--width", "2", "a.graal", "-", "b.graal",
        ],
        d.path(),
    );
    let gmap = fs::read_to_string(d.path().join("l.gmap")).unwrap();
    assert!(
        gmap.contains("\"a.graal\",\"\",\n\"b.graal\",\"\",\n"),
        "{gmap}"
    );
}

#[test]
fn generators_write_worlds_that_read_back() {
    let d = tempfile::tempdir().unwrap();
    run(
        &[
            "gen",
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
    let again = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(["gen", "terrain", "t", "--world", "isle", "--size", "2x1"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );

    fs::write(d.path().join("l.txt"), "XXXXXXXXX\nX.......X\nXXXXXXXXX\n").unwrap();
    run(
        &[
            "gen", "dungeon", "d", "--start", "cave_", "--layout", "l.txt",
        ],
        d.path(),
    );
    let out = run(
        &["gmap", "render", "d/cave_.gmap", "d.png", "--scale", "16"],
        d.path(),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("2 levels painted"));
}
