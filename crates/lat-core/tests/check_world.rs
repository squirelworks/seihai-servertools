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
