//! Runs the seihai-pw3d binary on a small made-up playerworld: two single
//! levels linked to each other and to a 2 x 1 gmap.

use seihai_core::assets::{builtin, tile_code, TileTypes};
use std::path::Path;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-pw3d"))
}

/// The first tile id of each kind on Seihai's built-in tiledefs.
fn ids() -> (u16, u16, u16) {
    let t = TileTypes::parse(&String::from_utf8_lossy(
        builtin::bytes("tiledefs.txt").unwrap(),
    ))
    .unwrap();
    let find = |want: u8| (0..4096u16).find(|&i| t.raw(i) == Some(want)).unwrap();
    (find(0), find(22), find(11))
}

/// A level: floor, a wall round the edge with a gap, a pond, and `extra`
/// lines (links, signs, ...).
fn level(extra: &str) -> String {
    let (floor, wall, water) = ids();
    let mut s = String::from("GLEVNW01\n");
    for y in 0..64 {
        let mut row = String::new();
        for x in 0..64 {
            let edge = x == 0 || y == 0 || x == 63 || y == 63;
            let gap = (28..36).contains(&y) && (x == 0 || x == 63);
            let pond = (40..44).contains(&x) && (40..44).contains(&y);
            let id = if edge && !gap {
                wall
            } else if pond {
                water
            } else {
                floor
            };
            row.push_str(std::str::from_utf8(&tile_code(id)).unwrap());
        }
        s.push_str(&format!("BOARD 0 {y} 64 0 {row}\n"));
    }
    s.push_str(extra);
    s
}

fn world(dir: &Path) {
    let w = |name: &str, text: String| std::fs::write(dir.join(name), text).unwrap();
    std::fs::create_dir_all(dir.join("levels")).unwrap();
    w(
        "start.nw",
        level(
            "LINK house.nw 63 28 1 8 1 playery\n\
             LINK fields_a.nw 0 28 1 8 62 32\n\
             SIGN 30 30\nWelcome to the \"test\" world\nSIGNEND\n\
             CHEST 20 20 greenrupee -1\n\
             NPC door.png 10 10\nif (playerenters) message hi;\nNPCEND\n",
        ),
    );
    w(
        "levels/house.nw",
        level("LINK start.nw 0 28 1 8 62 playery\n"),
    );
    w("fields_a.nw", level("LINK start.nw 63 28 1 8 1 32\n"));
    w("fields_b.nw", level("BADDY 30 30 0\n\n\n\nBADDYEND\n"));
    w(
        "fields.gmap",
        "GRMAP001\nWIDTH 2\nHEIGHT 1\nLEVELNAMES\n\"fields_a.nw\",\"fields_b.nw\"\nLEVELNAMESEND\n"
            .to_string(),
    );
}

#[test]
fn converts_a_small_world() {
    let tmp = tempfile::tempdir().unwrap();
    let (input, out) = (tmp.path().join("pw"), tmp.path().join("out"));
    std::fs::create_dir_all(&input).unwrap();
    world(&input);
    let o = bin()
        .args([
            input.to_str().unwrap(),
            out.to_str().unwrap(),
            "--start",
            "start.nw",
        ])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains("4 levels in 3 worlds"), "{stdout}");

    let default = std::fs::read_to_string(out.join("config/graal3dworld.txt")).unwrap();
    assert!(
        default.contains("object PW3D_L0 pw3d_start.3ds 0 0 0"),
        "{default}"
    );
    assert!(default.contains("weapon3d PW3DWalk weapons/pw3dwalk.gs2"));
    assert!(default.contains("playershape pw3d_player.3ds"));
    assert!(default.contains("\nspawn 32.00 -32.00 1.00\n"), "{default}");
    let fields = std::fs::read_to_string(out.join("config/graal3dworlds/fields.txt")).unwrap();
    assert!(
        fields.contains("object PW3D_L1 pw3d_fields_fields_b.3ds 64 0 0"),
        "{fields}"
    );
    assert!(out.join("config/graal3dworlds/house.txt").exists());

    // Every shape a world names is there, and is a 3DS file.
    for text in [&default, &fields] {
        for line in text
            .lines()
            .filter(|l| l.starts_with("object ") || l.starts_with("playershape "))
        {
            let shape = line
                .split_whitespace()
                .nth(if line.starts_with("object") { 2 } else { 1 })
                .unwrap();
            let bytes = std::fs::read(out.join("files").join(shape)).unwrap();
            assert_eq!(&bytes[..2], &0x4D4Du16.to_le_bytes(), "{shape}");
        }
    }
    for i in 0..6 {
        assert!(out.join(format!("files/pw3d_sky{i}.png")).exists());
    }
    // The 2D levels are copied as they were.
    assert!(out.join("levels/house.nw").exists());
    assert!(out.join("levels/fields.gmap").exists());

    // The walker: the links, the sign, and GS2 that seihai-core compiles
    // (the gserver compiles weapon3d scripts with it at link time).
    let walk = std::fs::read_to_string(out.join("weapons/pw3dwalk.gs2")).unwrap();
    assert!(
        walk.contains("{63, 28, 1, 8, \"house\", 0, 0, 1, \"k\", 0, 0, 0, 0}"),
        "{walk}"
    );
    assert!(walk.contains("{0, 28, 1, 8, \"fields\", 0, 0, 62, 32, 0, 0, 0, 0}"));
    // From fields_a back to start: the link's own level is at 0,0 in fields.
    assert!(walk.contains("{63, 28, 1, 8, \"default\", 0, 0, 1, 32, 0, 0, 0, 0}"));
    assert!(walk.contains("Welcome to the \\\"test\\\" world"));
    assert!(
        walk.contains("\"1#62.1#\""),
        "the gap rows run wall, floor, wall"
    );
    let client = walk.split("//#CLIENTSIDE").nth(1).unwrap();
    if let Err(e) = seihai_core::scripting::gs2::compile::compile_gs2(client) {
        panic!("the walker does not compile: {e:?}");
    }

    let report = std::fs::read_to_string(out.join("PW3D-REPORT.md")).unwrap();
    assert!(report.contains("| start.nw | default |"), "{report}");
    assert!(report.contains("Dropped: NPC images and scripts"));

    // A second run refuses to overwrite; --force replaces.
    let again = bin()
        .args([input.to_str().unwrap(), out.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!again.status.success());
    let forced = bin()
        .args([input.to_str().unwrap(), out.to_str().unwrap(), "--force"])
        .output()
        .unwrap();
    assert!(forced.status.success());
}

#[test]
fn usage_and_bad_input() {
    let v = bin().arg("--version").output().unwrap();
    assert!(v.status.success());
    assert!(String::from_utf8_lossy(&v.stdout).starts_with("seihai-pw3d "));
    let o = bin().arg("--help").output().unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).contains("seihai-pw3d INPUT OUT"));
    let tmp = tempfile::tempdir().unwrap();
    let o = bin()
        .args([
            tmp.path().to_str().unwrap(),
            tmp.path().join("o").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("no levels"));
}
