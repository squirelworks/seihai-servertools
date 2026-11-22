//! Authored parsed-data preservation controls; no renderer/native-range claim.
use lat_core::{gani::GaniDoc, level::LevelDoc};

#[test]
fn level_save_preserves_large_finite_npc_coordinates() {
    for (x, y) in [
        ("4294967296", "-4294967296"),
        ("2147483648", "-2147483904"),
        ("1.5", "2"),
        ("-0", "0"),
    ] {
        let input = format!("GLEVNW01\nNPC - {x} {y}\nNPCEND\n");
        let doc = LevelDoc::open(input.as_bytes()).unwrap();
        assert_eq!(doc.npcs().len(), 1);
        let before = &doc.npcs()[0];
        assert_eq!(
            (before.x, before.y),
            (x.parse::<f32>().unwrap(), y.parse::<f32>().unwrap())
        );
        let again = LevelDoc::open(&doc.encode()).unwrap();
        assert_eq!(again.npcs().len(), 1);
        assert_eq!((again.npcs()[0].x, again.npcs()[0].y), (before.x, before.y));
        assert_eq!(again.npcs()[0].image, before.image);
    }
}

#[test]
fn gani_save_preserves_large_finite_sound_coordinates() {
    for (x, y) in [
        ("4294967296", "-4294967296"),
        ("2147483648", "-2147483904"),
        ("1.5", "2"),
        ("-0", "0"),
    ] {
        let input = format!("SINGLEDIRECTION\nANI\n0 0 0\nPLAYSOUND own.wav {x} {y}\nANIEND\n");
        let doc = GaniDoc::open(input.as_bytes()).unwrap();
        assert_eq!(doc.gani().frames.len(), 1);
        assert_eq!(doc.gani().frames[0].sounds.len(), 1);
        let before = &doc.gani().frames[0].sounds[0];
        assert_eq!(
            (before.x, before.y),
            (x.parse::<f32>().unwrap(), y.parse::<f32>().unwrap())
        );
        let again = GaniDoc::open(&doc.encode()).unwrap();
        assert_eq!(again.gani().frames.len(), 1);
        assert_eq!(again.gani().frames[0].sounds.len(), 1);
        let after = &again.gani().frames[0].sounds[0];
        assert_eq!((after.x, after.y), (before.x, before.y));
        assert_eq!(after.file, "own.wav");
    }
}

#[test]
fn tile_edit_preserves_large_baddy_coordinates_and_verses() {
    let mut doc = LevelDoc::open(
        b"GLEVNW01\nBOARD 0 0 1 0 AA\nBADDY 4294967296 -4294967296 1\nauthored verse\nBADDYEND\n",
    )
    .unwrap();
    assert_eq!(doc.level().baddies.len(), 1);
    let before = doc.level().baddies[0].clone();
    assert_eq!((before.x, before.y), (4294967296.0_f32, -4294967296.0_f32));
    doc.set_tile(0, 0, 1).unwrap();
    let again = LevelDoc::open(&doc.encode()).unwrap();
    assert_eq!(again.tile(0, 0), Some(1));
    assert_eq!(again.level().baddies.len(), 1);
    let after = &again.level().baddies[0];
    assert_eq!((after.x, after.y), (before.x, before.y));
    assert_eq!(after.kind, before.kind);
    assert_eq!(after.verses, before.verses);
}

#[test]
fn tile_edit_preserves_large_link_arrivals_and_keep_semantics() {
    let mut doc = LevelDoc::open(
        b"GLEVNW01\nBOARD 0 0 1 0 AA\nLINK absolute.nw 0 0 1 1 4294967296 2147483648\nLINK relative.nw 1 0 1 1 playerx+4294967296 playery-4294967296\nLINK keep.nw 2 0 1 1 -1 playery\n",
    )
    .unwrap();
    assert_eq!(doc.level().links.len(), 3);
    let before = doc.level().links.clone();
    assert_eq!(
        (before[0].new_x, before[0].new_y),
        (Some(4294967296.0_f32), Some(2147483648.0_f32))
    );
    assert_eq!((before[1].new_x, before[1].new_y), (None, None));
    assert_eq!(
        (before[1].x_offset, before[1].y_offset),
        (4294967296.0_f32, -4294967296.0_f32)
    );
    // Negative absolute arrivals are intentionally parsed as "keep", not negatives to preserve.
    assert_eq!(
        (
            before[2].new_x,
            before[2].new_y,
            before[2].x_offset,
            before[2].y_offset
        ),
        (None, None, 0.0, 0.0)
    );
    doc.set_tile(0, 0, 1).unwrap();
    let again = LevelDoc::open(&doc.encode()).unwrap();
    assert_eq!(again.tile(0, 0), Some(1));
    assert_eq!(again.level().links.len(), before.len());
    for (after, old) in again.level().links.iter().zip(&before) {
        assert_eq!(
            (&after.dest, after.x, after.y, after.width, after.height),
            (&old.dest, old.x, old.y, old.width, old.height)
        );
        assert_eq!(
            (after.new_x, after.new_y, after.x_offset, after.y_offset),
            (old.new_x, old.new_y, old.x_offset, old.y_offset)
        );
    }
}
