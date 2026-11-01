//! The world checker on a small made-up world.

use std::fs;

use lat_core::check::{check_world, Severity};

#[test]
fn finds_bad_levels_missing_links_and_gmap_holes() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels/sub")).unwrap();
    fs::write(
        w.join("levels/start.nw"),
        "GLEVNW01\nLINK sub.nw 0 10 1 4 62 playery\nLINK nowhere.nw 63 10 1 4 1 playery\n",
    )
    .unwrap();
    fs::write(w.join("levels/sub/Sub.nw"), "GLEVNW01\n").unwrap();
    fs::write(w.join("levels/broken.nw"), "not a level\n").unwrap();
    fs::write(
        w.join("levels/world.gmap"),
        "GRMAP001\nWIDTH 2\nHEIGHT 1\nLEVELNAMES\n\"start.nw\",\"gone.nw\"\nLEVELNAMESEND\n",
    )
    .unwrap();

    let r = check_world(w);
    assert_eq!((r.levels, r.gmaps, r.ganis), (3, 1, 0));
    let texts: Vec<String> = r
        .findings
        .iter()
        .map(|f| format!("{:?} {} {}", f.severity, f.file, f.text))
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Error levels/broken.nw level does not parse")),
        "{texts:#?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Warn levels/start.nw") && t.contains("nowhere.nw")),
        "{texts:#?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("world.gmap") && t.contains("gone.nw")),
        "{texts:#?}"
    );
    // sub.nw matches Sub.nw in a subfolder: no finding for it.
    assert!(!texts.iter().any(|t| t.contains(" sub.nw")), "{texts:#?}");
    assert_eq!(r.errors(), 2);
    assert!(r.findings.iter().any(|f| f.severity == Severity::Warn));
}

#[test]
fn empty_world_is_clean() {
    let dir = tempfile::tempdir().unwrap();
    let r = check_world(dir.path());
    assert!(r.findings.is_empty());
}

#[test]
fn warns_when_an_npc_or_gani_names_a_missing_image_or_sound() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels/images")).unwrap();
    fs::create_dir_all(w.join("levels/heads")).unwrap();
    fs::create_dir_all(w.join("levels/ganis")).unwrap();
    fs::create_dir_all(w.join("sounds")).unwrap();
    // Different case, and a .png standing in for a named .gif: the resolver
    // accepts both. `-` and a `@…` text picture are not files.
    fs::write(w.join("levels/images/Tree.PNG"), b"").unwrap();
    fs::write(w.join("levels/door.png"), b"").unwrap();
    fs::write(w.join("levels/heads/head0.png"), b"").unwrap();
    fs::write(w.join("sounds/steps.wav"), b"").unwrap();
    fs::write(w.join("sounds/bell.wav"), b"").unwrap();
    fs::write(
        w.join("levels/start.nw"),
        "\
GLEVNW01
NPC - 1 1
NPCEND
NPC tree.png 2 2
NPCEND
NPC door.gif 3 3
NPCEND
NPC gone.png 4 4
NPCEND
NPC @Arial@b@Hi 5 5
NPCEND
NPC gone.png 6 6
NPCEND
",
    )
    .unwrap();
    fs::write(
        w.join("levels/ganis/move.gani"),
        "\
SPRITE 0 SPRITES 0 0 16 16
SPRITE 1 BODY 0 0 32 32
SPRITE 2 HEAD 0 0 32 32
SPRITE 3 SWORD 0 0 32 32
SPRITE 4 SHIELD 0 0 32 32
SPRITE 5 ATTR1 0 0 32 32
SPRITE 6 ATTR5 0 0 32 32
SPRITE 7 custom.png 0 0 16 16
DEFAULTHEAD head0.png
DEFAULTBODY body.png
DEFAULTATTR1 hat0.png
DEFAULTATTR2 star.png
SINGLEDIRECTION
ANI
0 0 0
PLAYSOUND steps.wav 1.5 2
1 0 0
PLAYSOUND +bell 1.5 2
2 0 0
PLAYSOUND gone.wav
3 0 0
PLAYSOUND gone.wav 1 2
ANIEND
",
    )
    .unwrap();

    let r = check_world(w);
    assert_eq!((r.levels, r.gmaps, r.ganis), (1, 0, 1));
    assert_eq!(r.errors(), 0);
    let texts: Vec<String> = r
        .findings
        .iter()
        .map(|f| format!("{:?} {} {}", f.severity, f.file, f.text))
        .collect();
    assert_eq!(
        texts,
        vec![
            "Warn levels/ganis/move.gani image custom.png (gani sprite) is not in the world",
            "Warn levels/ganis/move.gani image body.png (gani default body) is not in the world",
            "Warn levels/ganis/move.gani image hat0.png (gani attribute 1) is not in the world",
            "Warn levels/ganis/move.gani image star.png (gani attribute 2) is not in the world",
            "Warn levels/ganis/move.gani sound gone.wav (gani PLAYSOUND) is not in the world",
            "Warn levels/start.nw image gone.png (NPC image) is not in the world",
        ],
        "{texts:#?}"
    );
}
