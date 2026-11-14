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

    // Back to binary: the .graal reads back with the same board and link.
    run(&["level", "convert", "a.nw", "b.graal"], d.path());
    let graal = fs::read(d.path().join("b.graal")).unwrap();
    assert!(graal.starts_with(b"GR-V1.03"));
    run(&["level", "convert", "b.graal", "b.nw"], d.path());
    assert_eq!(fs::read_to_string(d.path().join("b.nw")).unwrap(), nw);

    run(&["level", "convert", "a.nw", "c.zelda"], d.path());
    assert!(fs::read(d.path().join("c.zelda"))
        .unwrap()
        .starts_with(b"Z3-V1.04"));
    run(&["level", "convert", "c.zelda", "c.nw"], d.path());
    assert!(fs::read_to_string(d.path().join("c.nw"))
        .unwrap()
        .contains("BOARD 0 0 64 0 AFAF"));

    let refused = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(["level", "convert", "a.nw", "b.txt"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(!refused.status.success());
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
    run(&["stats", "accounts", "out", "kills"], d.path());
    let html = fs::read_to_string(d.path().join("out/kills.html")).unwrap();
    assert!(html.find("Bob").unwrap() < html.find("Alice").unwrap());
    assert!(!html.contains("pbkdf2") && !html.contains("alice"));
}

/// A 2 x 1 GIF89a: one image of colour index `c` per entry of `frames`.
fn gif(frames: &[u8]) -> Vec<u8> {
    let mut g = b"GIF89a".to_vec();
    g.extend([2, 0, 1, 0, 0x81, 0, 0]);
    g.extend([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    for &c in frames {
        g.extend([0x2C, 0, 0, 0, 0, 2, 0, 1, 0, 0]);
        // LZW, min code 2, 3-bit codes: clear c clear c end.
        let (mut acc, mut n, mut bytes) = (0u32, 0u32, Vec::new());
        for code in [4u32, u32::from(c), 4, u32::from(c), 5] {
            acc |= code << n;
            n += 3;
            while n >= 8 {
                bytes.push(acc as u8);
                acc >>= 8;
                n -= 8;
            }
        }
        if n > 0 {
            bytes.push(acc as u8);
        }
        g.extend([2, bytes.len() as u8]);
        g.extend(bytes);
        g.push(0);
    }
    g.push(0x3B);
    g
}

#[test]
fn gif2mng_writes_a_png_or_an_mng() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("still.gif"), gif(&[1])).unwrap();
    fs::write(d.path().join("walk.gif"), gif(&[1, 2, 3])).unwrap();
    let out = run(
        &["gif2mng", "still.gif", "walk.gif", "--out", "o"],
        d.path(),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("3 frames"));
    assert!(fs::read(d.path().join("o/still.png"))
        .unwrap()
        .starts_with(b"\x89PNG"));
    assert!(fs::read(d.path().join("o/walk.mng"))
        .unwrap()
        .starts_with(b"\x8aMNG"));
    let again = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(["gif2mng", "walk.gif", "--out", "o"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );
    run(
        &["gif2mng", "-b", "--force", "walk.gif", "--out", "o"],
        d.path(),
    );
}

#[test]
fn render_draws_npc_images_with_npcs() {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir(d.path().join("art")).unwrap();
    // A 1 x 1 GIF of colour index 1 (red) stands in for the NPC's art.
    fs::write(d.path().join("art/Rock.GIF"), gif(&[1])).unwrap();
    fs::write(
        d.path().join("a.nw"),
        "GLEVNW01\nNPC rock.gif 1 1\nNPCEND\nNPC gone.png 2 2\nNPCEND\n",
    )
    .unwrap();
    run(&["level", "render", "a.nw", "plain.png"], d.path());
    let out = run(&["level", "render", "a.nw", "npcs.png", "--npcs"], d.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("missing image: gone.png"), "{stdout}");
    assert!(!stdout.contains("rock.gif"), "{stdout}");
    assert_ne!(
        fs::read(d.path().join("plain.png")).unwrap(),
        fs::read(d.path().join("npcs.png")).unwrap()
    );

    fs::write(
        d.path().join("w.gmap"),
        "GRMAP001\nWIDTH 1\nHEIGHT 1\nLEVELNAMES\n\"a.nw\",\nLEVELNAMESEND\n",
    )
    .unwrap();
    let out = run(
        &["gmap", "render", "w.gmap", "w.png", "--images", "art"],
        d.path(),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("missing image: gone.png"));
}

#[test]
fn level_fill_scatters_a_theme_and_theme_make_writes_one() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("t.nwt"),
        "[theme]\nname=t\n[rules]\nbase=AA\nflower=20\n[objects]\n<flower>\nAB\n",
    )
    .unwrap();
    fs::write(
        d.path().join("a.nw"),
        "GLEVNW01\nLINK b.nw 0 0 1 64 61 playery\n",
    )
    .unwrap();
    let out = run(
        &[
            "level", "fill", "t.nwt", "a.nw", "--out", "o", "--seed", "5", "--log", "l.txt",
        ],
        d.path(),
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("objects placed"));
    let filled = fs::read_to_string(d.path().join("o/a.nw")).unwrap();
    assert!(filled.contains("AB") && filled.ends_with("LINK b.nw 0 0 1 64 61 playery\n"));
    assert!(fs::read_to_string(d.path().join("l.txt"))
        .unwrap()
        .contains("flower:"));
    // The same seed gives the same level.
    run(
        &[
            "level", "fill", "t.nwt", "a.nw", "--out", "p", "--seed", "5",
        ],
        d.path(),
    );
    assert_eq!(filled, fs::read_to_string(d.path().join("p/a.nw")).unwrap());

    let mut tm = String::from("[Tiles]\n");
    for i in 0..16 {
        tm.push_str(&format!("Tile{i:02}=B{}\n", (b'A' + i) as char));
    }
    tm.push_str("[Objects]\nWidthMin=3\nWidthMax=5\nHeightMin=3\nHeightMax=5\nDensityMin=40\nDensityMax=80\nBaseObjectName=patch\n[Theme]\n#ofObjects=2\nQtyPerObject=c(50%,2)\nDescription=Patches\n");
    fs::write(d.path().join("p.tm"), tm).unwrap();
    run(&["theme", "make", "p.tm", "p.nwt"], d.path());
    let nwt = fs::read_to_string(d.path().join("p.nwt")).unwrap();
    assert!(nwt.contains("patch1=c(50%,2)|BA"), "{nwt}");
    run(&["level", "fill", "p.nwt", "a.nw", "--in-place"], d.path());
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
    let out = run(&["level", "fill", "c.nwt", "a.nw", "--out", "o"], d.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("3 objects placed") && stdout.contains("script: built"),
        "{stdout}"
    );
    let filled = fs::read_to_string(d.path().join("o/a.nw")).unwrap();
    assert_eq!(filled.matches("AC").count(), 9, "three diagonals of three");
}
