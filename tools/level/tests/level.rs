//! `seihai-level`, run as a program on its own.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
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
    let out = run(&["convert", "a.graal", "a.nw"], d.path());
    assert!(String::from_utf8_lossy(&out.stdout).contains("1 links"));
    let nw = fs::read_to_string(d.path().join("a.nw")).unwrap();
    assert!(nw.starts_with("GLEVNW01\n"));
    assert!(nw.contains("LINK b.graal 63 0 1 64 0 playery"), "{nw}");
    // Tile 5 is "AF" in GLEVNW01's base-64 pairs.
    assert!(nw.contains("BOARD 0 0 64 0 AFAF"), "{nw}");

    run(&["render", "a.nw", "a.png", "--scale", "16"], d.path());
    assert_eq!(png_size(&d.path().join("a.png")), (64, 64));

    // Back to binary: the .graal reads back with the same board and link.
    run(&["convert", "a.nw", "b.graal"], d.path());
    let graal = fs::read(d.path().join("b.graal")).unwrap();
    assert!(graal.starts_with(b"GR-V1.03"));
    run(&["convert", "b.graal", "b.nw"], d.path());
    assert_eq!(fs::read_to_string(d.path().join("b.nw")).unwrap(), nw);

    run(&["convert", "a.nw", "c.zelda"], d.path());
    assert!(fs::read(d.path().join("c.zelda"))
        .unwrap()
        .starts_with(b"Z3-V1.04"));
    run(&["convert", "c.zelda", "c.nw"], d.path());
    assert!(fs::read_to_string(d.path().join("c.nw"))
        .unwrap()
        .contains("BOARD 0 0 64 0 AFAF"));

    let refused = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
        .args(["convert", "a.nw", "b.txt"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
}

#[test]
fn render_draws_chests_signs_and_a_gani_npc() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("a.nw"),
        "GLEVNW01\nCHEST 2 3 greenrupee -1\nSIGN 0 0\nindex\nSIGNEND\nSIGN 4 5\nHello\nSIGNEND\nNPC - 6 6\nshowcharacter;\nNPCEND\n",
    )
    .unwrap();
    run(&["render", "a.nw", "plain.png"], d.path());
    let out = run(&["render", "a.nw", "npcs.png", "--npcs"], d.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("missing"), "{stdout}");
    assert_ne!(
        fs::read(d.path().join("plain.png")).unwrap(),
        fs::read(d.path().join("npcs.png")).unwrap(),
        "the showcharacter NPC is drawn only with --npcs"
    );
    fs::write(
        d.path().join("gone.nw"),
        "GLEVNW01\nNPC missing.gani 1 1\nNPCEND\n",
    )
    .unwrap();
    let out = run(&["render", "gone.nw", "gone.png", "--npcs"], d.path());
    assert!(String::from_utf8_lossy(&out.stdout).contains("missing gani: missing.gani"));
}

#[test]
fn level_fill_runs_complex_objects() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("c.nwt"),
        "[theme]\nname=c\n[rules]\nbase=AA\npatch=3\n[objects]\n<rock>\nAC\n<patch>*\nsetsize(3,3);\nfor (i=0,2) { add(<rock>,i,i); }\nmsg('built');\n",
    )
    .unwrap();
    fs::write(d.path().join("a.nw"), "GLEVNW01\n").unwrap();
    let out = run(&["fill", "c.nwt", "a.nw", "--out", "o"], d.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("3 objects placed") && stdout.contains("script: built"),
        "{stdout}"
    );
    let filled = fs::read_to_string(d.path().join("o/a.nw")).unwrap();
    assert_eq!(filled.matches("AC").count(), 9, "three diagonals of three");
}

#[test]
fn level_fill_runs_across_a_gmap() {
    let d = tempfile::tempdir().unwrap();
    // Long logs, tried over the edge, with a border under each patch.
    fs::write(
        d.path().join("l.nwt"),
        "[theme]\nname=l\n[rules]\nbase=AA\nlog=300\n[objects]\n<log>\n.bttm=100%,moss\nAFAFAFAF\n<moss>\nAH\n",
    )
    .unwrap();
    for n in ["w-a01.nw", "w-b01.nw"] {
        fs::write(d.path().join(n), "GLEVNW01\n").unwrap();
    }
    fs::write(
        d.path().join("w.gmap"),
        "GRMAP001\nWIDTH 2\nHEIGHT 1\nLEVELNAMES\n\"W-A01.nw\",\"w-b01.nw\",\nLEVELNAMESEND\n",
    )
    .unwrap();
    let out = run(
        &[
            "fill", "l.nwt", "w-a01.nw", "w-b01.nw", "--out", "o", "--edges", "--across", "w.gmap",
            "--seed", "2",
        ],
        d.path(),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("w-b01.nw"), "{stdout}");
    let right = fs::read_to_string(d.path().join("o/w-b01.nw")).unwrap();
    let left = fs::read_to_string(d.path().join("o/w-a01.nw")).unwrap();
    assert!(left.contains("AH"), "borders drawn");
    // Some log in the left level ran over into the right one: a row of the
    // right level that starts with a log piece shorter than a whole log.
    let crossed = right.lines().filter(|l| l.starts_with("BOARD")).any(|l| {
        let cells = &l.split_whitespace().nth(5).unwrap();
        let run = cells
            .as_bytes()
            .chunks(2)
            .take_while(|c| *c == b"AF")
            .count();
        run % 4 != 0
    });
    assert!(crossed, "a log runs across the edge");
}

#[test]
fn level_convert_goes_to_tiled_and_back() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("a.nw"),
        "GLEVNW01\nBOARD 0 0 2 0 ABAC\nLINK b.nw 0 0 1 64 61 playery\n",
    )
    .unwrap();
    run(&["convert", "a.nw", "a.tmx"], d.path());
    let tmx = fs::read_to_string(d.path().join("a.tmx")).unwrap();
    assert!(tmx.contains("<image source=\"pics1.png\""), "{tmx}");
    run(&["convert", "a.tmx", "b.nw"], d.path());
    let back = fs::read_to_string(d.path().join("b.nw")).unwrap();
    assert!(back.contains("LINK b.nw 0 0 1 64 61 playery"), "{back}");
    assert!(back.contains("BOARD 0 0 64 0 ABAC"), "{back}");
}

#[test]
fn invalid_tmx_does_not_replace_an_existing_level() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("bad.tmx"), "<map width=\"64\" height=\"64\">").unwrap();
    let original = b"GLEVNW01\nBOARD 0 0 1 0 AB\n";
    fs::write(d.path().join("keep.nw"), original).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
        .args(["convert", "bad.tmx", "keep.nw"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(fs::read(d.path().join("keep.nw")).unwrap(), original);
}

#[test]
fn unsupported_tmx_layout_does_not_replace_an_existing_level() {
    let d = tempfile::tempdir().unwrap();
    let original = b"GLEVNW01\nBOARD 0 0 1 0 AB\n";
    fs::write(d.path().join("keep.nw"), original).unwrap();
    run(&["convert", "keep.nw", "valid.tmx"], d.path());
    let valid = fs::read_to_string(d.path().join("valid.tmx")).unwrap();
    for bad in [
        valid.replace("columns=\"128\"", "columns=\"64\""),
        valid.replace("<layer ", "<layer offsetx=\"16\" "),
        valid.replace("</map>", "<group offsety=\"16\"/></map>"),
    ] {
        fs::write(d.path().join("bad.tmx"), bad).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
            .args(["convert", "bad.tmx", "keep.nw"])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(fs::read(d.path().join("keep.nw")).unwrap(), original);
    }
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-level "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-level"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("convert"), "{text}");
}
