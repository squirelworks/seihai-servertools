//! The world checker on a small made-up world.

use std::fs;

use lat_core::check::{check_world, Severity};

#[test]
fn gani_reach_uses_profile_support_even_with_gif_art_or_symbolic_sheets() {
    use seihai_core::protocol::ClientVersion;
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels/ganis")).unwrap();
    let literal = "SPRITE 0 custom.png 0 0 16 16\nDEFAULTHEAD head.gif\nSINGLEDIRECTION\nANI\n0 0 0\nANIEND\n";
    let symbolic = "SPRITE 0 BODY 0 0 16 16\nSPRITE 1 HEAD 0 0 16 16\nSINGLEDIRECTION\nANI\n0 0 0,1 0 0\nANIEND\n";
    fs::write(w.join("levels/ganis/custom.gani"), literal).unwrap();
    fs::write(w.join("levels/ganis/idle.GANI"), symbolic).unwrap();
    fs::write(w.join("custom.png"), b"png").unwrap();
    fs::write(w.join("custom.gif"), b"gif").unwrap();
    fs::write(w.join("head.gif"), b"gif").unwrap();
    fs::write(w.join("notes.txt"), literal).unwrap();
    let findings = lat_core::check::check_image_reach(w);
    assert_eq!(findings.len(), 2, "{findings:?}");
    for f in &findings {
        assert_eq!(f.severity, Severity::Warn);
        assert!(f.file.to_ascii_lowercase().ends_with(".gani"));
        assert!(f.text.contains("if it is used"));
        assert!(f.text.contains("no nearest built-in animation is inferred"));
        let labels = f
            .text
            .split('[')
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap()
            .split(", ")
            .collect::<Vec<_>>();
        assert!(!ClientVersion::V139.profile().ui.ganis);
        assert!(ClientVersion::V141.profile().ui.ganis);
        assert!(labels.contains(&ClientVersion::V139.label()));
        assert!(!labels.contains(&ClientVersion::V141.label()));
        assert!(!labels.contains(&ClientVersion::V205.label()));
    }
    // No literal BODY/HEAD lookup, conversion or mutation is introduced.
    assert_eq!(
        fs::read_to_string(w.join("levels/ganis/custom.gani")).unwrap(),
        literal
    );
    assert_eq!(
        fs::read_to_string(w.join("levels/ganis/idle.GANI")).unwrap(),
        symbolic
    );
    assert!(!w.join("BODY.gif").exists());
    let ordinary = check_world(w);
    assert!(ordinary.findings.is_empty(), "{:?}", ordinary.findings);
}

#[test]
fn image_reach_checks_references_and_gif_counterparts_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels/images")).unwrap();
    let level = "GLEVNW01\nNPC Tree.PNG 1 1\nshowimg 1,tree.png,1,1;\nshowimg 2,flower.gif,1,1;\nNPCEND\nNPC paired.png 2 2\nNPCEND\nNPC - 3 3\nNPCEND\n";
    fs::write(w.join("levels/start.nw"), level).unwrap();
    for name in [
        "Tree.PNG",
        "flower.png",
        "paired.png",
        "PAIRED.GIF",
        "unused.png",
    ] {
        fs::write(w.join("levels/images").join(name), b"art").unwrap();
    }
    let findings = lat_core::check::check_image_reach(w);
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert!(findings
        .iter()
        .all(|f| f.file == "levels/start.nw" && f.severity == Severity::Warn));
    assert!(findings.iter().any(|f| f.text.contains("tree.gif")));
    assert!(findings.iter().any(|f| f.text.contains("flower.gif")));
    assert_eq!(
        fs::read_to_string(w.join("levels/start.nw")).unwrap(),
        level
    );
    assert!(!w.join("levels/images/tree.gif").exists());
    fs::write(w.join("levels/images/tree.gif"), b"gif").unwrap();
    assert_eq!(lat_core::check::check_image_reach(w).len(), 1);
}

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

#[test]
fn checks_level_npc_scripts() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels")).unwrap();
    fs::create_dir_all(w.join("sounds")).unwrap();
    fs::write(w.join("levels/door.png"), b"").unwrap();
    fs::write(w.join("sounds/chest.wav"), b"").unwrap();
    fs::write(
        w.join("levels/start.nw"),
        "\
GLEVNW01
NPC - 1 1
if (playerenters) {
  setimg door.png;
  showimg 1,lamp.png,30,30;
}
if (playertouchsme) { play chest.wav; play creak.wav; }
setimg #s(this.pic);
NPCEND
NPC - 2 2
//#CLIENTSIDE
if (created) {
  frobnicate now;
  setimg door.png
NPCEND
NPC - 3 3
if (a) { x = ; } if (b) { y = ; } if (c) { z = ; } if (d) { w = ; }
NPCEND
",
    )
    .unwrap();

    let r = check_world(w);
    assert_eq!(r.errors(), 0, "script problems are warnings");
    let texts: Vec<String> = r
        .findings
        .iter()
        .map(|f| format!("{:?} {}", f.severity, f.text))
        .collect();
    assert_eq!(
        texts[..2],
        [
            "Warn image lamp.png (showimg in an NPC script) is not in the world",
            "Warn sound creak.wav (play in an NPC script) is not in the world",
        ],
        "{texts:#?}"
    );
    let npc2: Vec<&String> = texts.iter().filter(|t| t.contains("NPC at 2,2")).collect();
    assert!(
        npc2.iter()
            .any(|t| t.contains("GS1 script line") && t.contains("skipped")),
        "{texts:#?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("frobnicate")),
        "unknown names are not reported: {texts:#?}"
    );
    let npc3: Vec<&String> = texts.iter().filter(|t| t.contains("NPC at 3,3")).collect();
    assert_eq!(npc3.len(), 4, "three listed, then a count: {texts:#?}");
    assert!(npc3[3].ends_with("1 more GS1 script problem"), "{texts:#?}");
}
