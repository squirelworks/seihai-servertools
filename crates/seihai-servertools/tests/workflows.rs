//! The all-in-one `seihai-servertools`: it reaches every tool, and runs
//! the flows that use more than one tool (each tool's own tests are in
//! tools/<tool>/tests).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
}

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
fn version_and_help() {
    let out = bin().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-servertools "));
    let out = bin().arg("--help").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("lab start"));
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
    run(
        &[
            "gen", "terrain", "t3", "--world", "isle", "--size", "2x2", "--3d", "--trees", "8",
        ],
        d.path(),
    );
    let world3d = fs::read_to_string(d.path().join("t3/isle_graal3dworld.txt")).unwrap();
    assert!(
        world3d.contains("object Ground isle.3ds 0 0 0"),
        "{world3d}"
    );
    assert!(world3d.contains("\nspawn "), "{world3d}");
    for f in ["isle.3ds", "isle_tree.3ds", "isle_rock.3ds", "isleA00.nw"] {
        assert!(d.path().join("t3").join(f).exists(), "{f}");
    }
    assert!(fs::read(d.path().join("t3/isle.3ds"))
        .unwrap()
        .starts_with(&[0x4D, 0x4D]));
    let again = Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(["gen", "terrain", "t", "--world", "isle", "--size", "2x1"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );
    // gen 3d rebuilds the same 3D world from the gmap's HEIGHTMAP.
    run(
        &["gen", "3d", "t3/isle.gmap", "from", "--trees", "8"],
        d.path(),
    );
    assert_eq!(
        fs::read(d.path().join("from/isle.3ds")).unwrap(),
        fs::read(d.path().join("t3/isle.3ds")).unwrap()
    );
    let rebuilt = fs::read_to_string(d.path().join("from/isle_graal3dworld.txt")).unwrap();
    let body = |t: &str| {
        t.lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(body(&rebuilt), body(&world3d));

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
