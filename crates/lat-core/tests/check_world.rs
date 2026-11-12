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
        assert!(!ClientVersion::V141.profile().ui.ganis);
        assert!(ClientVersion::V142.profile().ui.ganis);
        assert!(labels.contains(&ClientVersion::V139.label()));
        assert!(labels.contains(&ClientVersion::V141.label()));
        assert!(!labels.contains(&ClientVersion::V142.label()));
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

/// A gserver folder as `seihai-gserver-v2` lays it out: `config/` and the
/// served `world/`, packages in `world/gpaks/`.
fn server(world: &std::path::Path, name: &str, allowed: &str, options: &str) -> std::path::PathBuf {
    let s = world.join("servers").join(name);
    fs::create_dir_all(s.join("config")).unwrap();
    fs::create_dir_all(s.join("world/gpaks")).unwrap();
    fs::write(s.join("config/allowedversions.txt"), allowed).unwrap();
    fs::write(s.join("config/serveroptions.txt"), options).unwrap();
    s
}

fn package_findings(w: &std::path::Path) -> Vec<(String, String)> {
    lat_core::check::check_packages(w)
        .into_iter()
        .map(|f| {
            assert_eq!(f.severity, Severity::Warn);
            (f.file, f.text)
        })
        .collect()
}

#[test]
fn packages_need_their_key() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    let s = server(w, "modern", "G3D0311C\n", "generation = modern\n");
    fs::write(s.join("world/gpaks/art.gpake"), b"zip").unwrap();
    fs::write(s.join("world/gpaks/Music.GPAKE"), b"zip").unwrap();
    fs::write(
        s.join("config/gpakkeys.txt"),
        "# keys\nart.gpake 0123456789ABCDEF\n",
    )
    .unwrap();
    let f = package_findings(w);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].0, "servers/modern/world/gpaks/Music.GPAKE");
    assert!(f[0].1.contains("music.gpake has no key"), "{}", f[0].1);
    assert!(f[0].1.contains("servers/modern/config/gpakkeys.txt"));

    // A key line that does not parse is no key.
    fs::write(
        s.join("config/gpakkeys.txt"),
        "art.gpake 0123\nmusic.gpake 0123456789abcdef\n",
    )
    .unwrap();
    let f = package_findings(w);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].0.ends_with("art.gpake"));

    // check_world reports them too, so `check` and the Check page show them.
    let r = check_world(w);
    assert_eq!(r.findings.len(), 1, "{:?}", r.findings);
    assert_eq!(r.errors(), 0);
}

#[test]
fn a_served_key_file_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    let s = server(w, "default", "", "");
    fs::write(s.join("world/gpakkeys.txt"), "art.gpake 0123456789ABCDEF\n").unwrap();
    fs::write(s.join("world/gpaks/GPAKKEYS.TXT.bak"), "old\n").unwrap();
    fs::write(
        s.join("config/gpakkeys.txt"),
        "art.gpake 0123456789ABCDEF\n",
    )
    .unwrap();
    // seihai-gserver's world folder: config/ and the served files/ at the top.
    fs::create_dir_all(w.join("config")).unwrap();
    fs::create_dir_all(w.join("files")).unwrap();
    fs::write(w.join("files/gpakkeys.txt"), "x\n").unwrap();
    let f = package_findings(w);
    let files: Vec<&str> = f.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(
        files,
        [
            "files/gpakkeys.txt",
            "servers/default/world/gpakkeys.txt",
            "servers/default/world/gpaks/GPAKKEYS.TXT.bak",
        ],
        "{f:?}"
    );
    assert!(f.iter().all(|(_, t)| t.contains("anyone can download it")));
}

#[test]
fn plain_gpak_warns_only_when_420_may_log_in() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    let s = server(w, "x", "", "");
    fs::write(s.join("world/gpaks/art.gpak"), b"zip").unwrap();
    fs::write(s.join("world/gpaks/art.zip"), b"zip").unwrap();
    let allowed = s.join("config/allowedversions.txt");
    let options = s.join("config/serveroptions.txt");
    let warns = |allowed_txt: &str, options_txt: &str| {
        fs::write(&allowed, allowed_txt).unwrap();
        fs::write(&options, options_txt).unwrap();
        let f = package_findings(w);
        assert!(f.len() <= 1, "{f:?}");
        f.first().map(|(file, text)| {
            assert_eq!(file, "servers/x/world/gpaks/art.gpak");
            assert!(text.contains("never loads it"), "{text}");
            text.clone()
        })
    };
    // 4.20r8 named, a range around it (4.04r2 to 6.03r7), or everything.
    assert!(warns("G3D20126\n", "").is_some());
    assert!(warns("G3D17026:G3D0311C\n", "").is_some());
    assert!(warns("*\n", "").is_some());
    // 5.12 to 6.03r7 Linux (beta4's stock modern line) leaves 4.20r8 out;
    // so do 2.x-only lists and commented lines.
    assert!(warns("G3D14097:G3D0511C\n", "").is_none());
    assert!(warns("GNW03014:GNW28015\n# G3D20126\n// *\n", "").is_none());
    // Generation lines: only the server's own generation counts.
    let lines = "[generation-range]\nnewmain = GNW03014:GNW28015\nmodern = G3D17026:G3D0511C\n";
    assert!(warns(lines, "generation = newmain\n").is_none());
    let text = warns(lines, "generation = modern\n").unwrap();
    assert!(text.contains("modern = G3D17026:G3D0511C"), "{text}");
    // No generation set: any line may apply.
    assert!(warns(lines, "").is_some());
    // No allow-list at all: nothing to say.
    fs::remove_file(&allowed).unwrap();
    assert!(package_findings(w).is_empty());
}

#[test]
fn level_reach_lists_profiles_without_gmaps_or_the_level_format() {
    use seihai_core::protocol::ClientVersion;
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels")).unwrap();
    fs::write(w.join("levels/start.nw"), "GLEVNW01\n").unwrap();
    // The magic decides, not the extension.
    fs::write(w.join("levels/odd.graal"), "GLEVNW01\n").unwrap();
    fs::write(w.join("levels/broken.nw"), "not a level\n").unwrap();
    fs::write(
        w.join("levels/world.gmap"),
        "GRMAP001\nWIDTH 1\nHEIGHT 1\nLEVELNAMES\n\"start.nw\"\nLEVELNAMESEND\n",
    )
    .unwrap();
    fs::write(w.join("levels/bad.gmap"), "nope\n").unwrap();

    let labels = |lacks: fn(&seihai_core::protocol::Levels) -> bool| -> Vec<&str> {
        ClientVersion::all_versions()
            .iter()
            .filter(|v| lacks(&v.profile().levels))
            .map(|v| v.label())
            .collect()
    };
    let no_nw = labels(|l| !l.nw);
    let no_gmap = labels(|l| !l.gmap);
    // The pinned profiles have both kinds, or this test checks nothing.
    assert!(!no_nw.is_empty() && !no_gmap.is_empty());
    assert!(!no_gmap.contains(&ClientVersion::V222.label()));

    let f = lat_core::check::check_level_reach(w);
    let files: Vec<&str> = f.iter().map(|f| f.file.as_str()).collect();
    assert_eq!(
        files,
        ["levels/odd.graal", "levels/start.nw", "levels/world.gmap"],
        "{f:?}"
    );
    assert!(f.iter().all(|f| f.severity == Severity::Warn));
    assert!(f[0].text.contains("NW level"), "{}", f[0].text);
    assert!(
        f[1].text.contains(&format!("[{}]", no_nw.join(", "))),
        "{}",
        f[1].text
    );
    assert!(
        f[2].text.contains(&format!("[{}]", no_gmap.join(", "))),
        "{}",
        f[2].text
    );
    assert!(f[2].text.contains("linked single levels"));
    // Inventory only: nothing written.
    assert_eq!(fs::read_dir(w.join("levels")).unwrap().count(), 5);
}

#[test]
fn script_reach_lists_gs2_and_mixed_level_npcs_and_weapons() {
    use seihai_core::protocol::ClientVersion;
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::write(
        w.join("start.nw"),
        "GLEVNW01\n\
         NPC - 1 1\nif (playerenters) { message hi; }\nNPCEND\n\
         NPC - 2 2\nfunction onCreated() { this.x = 1; }\nNPCEND\n\
         NPC - 3 3\nif (created) { message hi; }\nfunction onPlayerChats() { temp.a = 1; }\nNPCEND\n",
    )
    .unwrap();
    fs::create_dir_all(w.join("servers/default/weapons")).unwrap();
    fs::write(
        w.join("servers/default/weapons/weapon-Bow.txt"),
        "GRAWP001\nREALNAME -Bow\nIMAGE bow.png\nSCRIPT\nfunction onActionServerSide() { player.chat = \"x\"; }\nSCRIPTEND\n",
    )
    .unwrap();
    fs::write(
        w.join("servers/default/weapons/weapon-Old.txt"),
        "GRAWP001\nREALNAME -Old\nIMAGE old.gif\nSCRIPT\nif (weaponfired) { message #c; }\nSCRIPTEND\n",
    )
    .unwrap();
    // Not a weapon file: wrong folder.
    fs::write(
        w.join("notes.txt"),
        "GRAWP001\nSCRIPT\nfunction onCreated() {}\nSCRIPTEND\n",
    )
    .unwrap();
    let f = lat_core::check::check_script_reach(w);
    let texts: Vec<String> = f
        .iter()
        .map(|f| format!("{}: {}", f.file, f.text))
        .collect();
    assert_eq!(f.len(), 3, "{texts:#?}");
    assert!(f.iter().all(|f| f.severity == Severity::Warn));
    assert!(
        texts[0].starts_with(
            "servers/default/weapons/weapon-Bow.txt: client reach: weapon -Bow script is GS2 only"
        ),
        "{texts:#?}"
    );
    assert!(
        texts[1].starts_with("start.nw: client reach: NPC at 2,2 script is GS2 only"),
        "{texts:#?}"
    );
    assert!(
        texts[2].starts_with("start.nw: client reach: NPC at 3,3 script is both GS1 and GS2"),
        "{texts:#?}"
    );
    for t in &texts {
        let lists: Vec<Vec<&str>> = t
            .split('[')
            .skip(1)
            .map(|p| p.split(']').next().unwrap().split(", ").collect())
            .collect();
        let (no, unknown) = (&lists[0], &lists[1]);
        // The lists are seihai-core's `scripts.gs2` field.
        for v in ClientVersion::all_versions() {
            let gs2 = v.profile().scripts.gs2.as_str();
            assert_eq!(no.contains(&v.label()), gs2 == "no", "{v:?} {t}");
            assert_eq!(unknown.contains(&v.label()), gs2 == "unknown", "{v:?} {t}");
        }
        for v in [
            ClientVersion::V121,
            ClientVersion::V231,
            ClientVersion::V304,
        ] {
            assert!(no.contains(&v.label()), "{v:?} {t}");
        }
        for v in [
            ClientVersion::V3153,
            ClientVersion::V404R2,
            ClientVersion::V603R7,
        ] {
            assert!(
                !no.contains(&v.label()) && !unknown.contains(&v.label()),
                "{v:?} {t}"
            );
        }
        for v in [
            ClientVersion::V210,
            ClientVersion::V230,
            ClientVersion::V6015,
        ] {
            assert!(
                unknown.contains(&v.label()) && !no.contains(&v.label()),
                "{v:?} {t}"
            );
        }
    }
    let weapon =
        lat_core::check::read_weapon(&w.join("servers/default/weapons/weapon-Bow.txt")).unwrap();
    assert_eq!(weapon.name, "-Bow");
    assert_eq!(weapon.image, "bow.png");
    assert!(lat_core::check::read_weapon(&w.join("notes.txt")).is_none());
}

#[test]
fn tile_reach_lists_levels_past_the_1856_wide_sheet() {
    use seihai_core::protocol::ClientVersion;
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    // 3588 ("4E") sits in block 7, column 112 + 4 = 116: past the old sheet.
    // 3587 ("4D") is column 115, the old sheet's last. "AA" is tile 0.
    fs::write(w.join("new.nw"), "GLEVNW01\nBOARD 0 0 4 0 AA4E4E4D\n").unwrap();
    fs::write(w.join("old.nw"), "GLEVNW01\nBOARD 0 0 2 0 AA4D\n").unwrap();
    let f = lat_core::check::check_tile_reach(w);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].file, "new.nw");
    assert_eq!(f[0].severity, Severity::Warn);
    assert!(
        f[0].text.contains("2 cells use tiles past column 116"),
        "{}",
        f[0].text
    );
    assert!(f[0].text.contains("(ids 3588)"), "{}", f[0].text);
    let labels: Vec<&str> = f[0]
        .text
        .split('[')
        .nth(1)
        .unwrap()
        .split(']')
        .next()
        .unwrap()
        .split(", ")
        .collect();
    for v in [
        ClientVersion::V128B2,
        ClientVersion::V139,
        ClientVersion::V205,
    ] {
        assert!(labels.contains(&v.label()), "{v:?}");
    }
    for v in [
        ClientVersion::V121,
        ClientVersion::V142,
        ClientVersion::V222,
    ] {
        assert!(!labels.contains(&v.label()), "{v:?}");
    }
}

#[test]
fn weapon_images_join_the_gif_inventory_and_copies() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("servers/default/weapons")).unwrap();
    fs::write(
        w.join("servers/default/weapons/weapon-Bow.txt"),
        "GRAWP001\nREALNAME -Bow\nIMAGE bow.png\nSCRIPT\nif (weaponfired) { setimg arrow.png; }\nSCRIPTEND\n",
    )
    .unwrap();
    fs::write(w.join("bow.png"), b"png").unwrap();
    fs::write(w.join("arrow.png"), b"png").unwrap();
    fs::write(w.join("unused.png"), b"png").unwrap();
    let f = lat_core::check::check_image_reach(w);
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f
        .iter()
        .all(|f| f.file == "servers/default/weapons/weapon-Bow.txt"));
    assert!(
        f[0].text.contains("bow.gif") && f[1].text.contains("arrow.gif"),
        "{f:?}"
    );
    let copies = lat_core::check::gif_copies_wanted(w);
    assert_eq!(
        copies,
        vec![
            (w.join("arrow.png"), w.join("arrow.gif")),
            (w.join("bow.png"), w.join("bow.gif")),
        ]
    );
    // A GRAWP001 file outside weapons/ names nothing.
    fs::remove_dir_all(w.join("servers")).unwrap();
    fs::write(w.join("weapon-Bow.txt"), "GRAWP001\nIMAGE bow.png\n").unwrap();
    assert!(lat_core::check::check_image_reach(w).is_empty());
}

#[test]
fn run_time_image_names_are_matched_to_world_files() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("levels/images")).unwrap();
    let level = "GLEVNW01\nNPC - 1 1\nif (created) {\n  setimg bomb#v(this.n).png;\n  showimg 1,#s(this.pic),1,1;\n  showimg 2,ghost#v(this.n).png,1,1;\n}\nNPCEND\n";
    fs::write(w.join("levels/start.nw"), level).unwrap();
    // bomb1 has its GIF, bomb2 does not, and Bomb3.PNG checks case. tree.png
    // is not named at all.
    for name in [
        "bomb1.png",
        "bomb1.gif",
        "bomb2.png",
        "Bomb3.PNG",
        "tree.png",
    ] {
        fs::write(w.join("levels/images").join(name), b"art").unwrap();
    }
    let f = lat_core::check::check_image_reach(w);
    let texts: Vec<&str> = f.iter().map(|f| f.text.as_str()).collect();
    assert_eq!(f.len(), 4, "{texts:#?}");
    assert!(f
        .iter()
        .all(|f| f.file == "levels/start.nw" && f.severity == Severity::Warn));
    for stem in ["bomb2", "bomb3"] {
        assert!(
            texts.iter().any(|t| t.contains(&format!("no {stem}.gif"))
                && t.contains("a run-time name, bomb#v(this.n).png, can be this file")),
            "{stem}: {texts:#?}"
        );
    }
    assert!(!texts.iter().any(|t| t.contains("bomb1.gif")));
    assert!(!texts.iter().any(|t| t.contains("tree.gif")));
    assert!(texts
        .iter()
        .any(|t| t.contains("#s(this.pic)") && t.contains("says too little")));
    assert!(texts
        .iter()
        .any(|t| t.contains("ghost#v(this.n).png") && t.contains("matches no PNG or GIF")));

    // --make-gif covers what the run-time name can be, and nothing else.
    let copies = lat_core::check::gif_copies_wanted(w);
    assert_eq!(
        copies,
        vec![
            (
                w.join("levels/images/Bomb3.PNG"),
                w.join("levels/images/bomb3.gif")
            ),
            (
                w.join("levels/images/bomb2.png"),
                w.join("levels/images/bomb2.gif")
            ),
        ]
    );
    // The ordinary check does not report run-time names as missing files.
    assert!(
        check_world(w).findings.is_empty(),
        "{:?}",
        check_world(w).findings
    );
}
