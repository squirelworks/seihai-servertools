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
