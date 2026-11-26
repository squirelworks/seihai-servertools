//! `seihai-code`, run as a program on its own.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-code"))
}

#[test]
fn code_cache_round_trip_on_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.bin");
    let packed = dir.path().join("start.nw-14802.code");
    let dumped = dir.path().join("dump.bin");
    let script_plain = b"if (playerenters) { }";
    fs::write(&plain, script_plain).unwrap();
    let key = "0123456789ABCDEF";
    let out = bin()
        .args(["pack", "--key", key, "--out"])
        .arg(&packed)
        .arg(&plain)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let crc = bin()
        .args(["crc", "--key", key])
        .arg(&packed)
        .output()
        .unwrap();
    assert!(
        crc.status.success(),
        "{}",
        String::from_utf8_lossy(&crc.stderr)
    );
    let crc_text = String::from_utf8_lossy(&crc.stdout);
    assert!(crc_text.trim().chars().all(|c| c.is_ascii_digit()));
    let show = bin()
        .args(["show", "--key", key])
        .arg(&packed)
        .output()
        .unwrap();
    let shown = String::from_utf8_lossy(&show.stdout);
    assert!(shown.contains("GRESCR01"), "{shown}");
    assert!(
        shown.contains(&format!("stored_crc32: {}", crc_text.trim())),
        "{shown}"
    );
    let out = bin()
        .args(["dump", "--key", key, "--out"])
        .arg(&dumped)
        .arg(&packed)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fs::read(&dumped).unwrap(), script_plain);

    let level = lat_core::code_cache::LevelCache {
        host: "127.0.0.1".into(),
        signature: 73,
        mod_time: 1_700_000_000,
        level: "start.nw".into(),
        body: b"board".to_vec(),
    };
    let level_plain = dir.path().join("level.txt");
    let level_code = dir.path().join("start.nw-14802.code");
    fs::write(
        &level_plain,
        lat_core::code_cache::level_plaintext(&level).unwrap(),
    )
    .unwrap();
    let out = bin()
        .args(["pack", "--out"])
        .arg(&level_code)
        .arg(&level_plain)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let show = bin().args(["show"]).arg(&level_code).output().unwrap();
    let shown = String::from_utf8_lossy(&show.stdout);
    assert!(shown.contains("host: 127.0.0.1"), "{shown}");
    assert!(shown.contains("level: start.nw"), "{shown}");
    assert!(shown.contains("signature: 73"), "{shown}");
    assert!(shown.contains("body_bytes: 5"), "{shown}");
}

#[test]
fn code_crc_defaults_to_the_stored_domain_on_known_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.code");
    let mut file = b"GRESCR01".to_vec();
    file.extend([
        0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05, b'x', b'y', b'z',
    ]);
    fs::write(&path, &file).unwrap();
    for args in [
        vec![],
        vec!["--domain", "plain"],
        vec!["--domain", "stored"],
    ] {
        let stored = !args.contains(&"plain");
        let out = bin()
            .args(["crc"])
            .arg(&path)
            .args(args)
            .args(if stored {
                vec![]
            } else {
                vec!["--key", "C82CEA9ED93DFB8F"]
            })
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            if stored { "2627766304" } else { "2773182824" }
        );
    }
    let shown = bin()
        .args(["show"])
        .arg(&path)
        .args(["--key", "C82CEA9ED93DFB8F"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&shown.stdout);
    assert!(shown.status.success());
    assert!(text.contains("crc32: 2773182824\n"));
    assert!(text.contains("stored_crc32: 2627766304\n"));
    for args in [
        vec!["crc", "--domain", "plain"],
        vec!["crc", "--domain", "unknown"],
        vec!["show", "--domain", "stored"],
    ] {
        let out = bin().args(args).arg(&path).output().unwrap();
        assert!(!out.status.success());
    }
    assert_eq!(fs::read(path).unwrap(), file);
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-code"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-code "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-code"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("crc"), "{text}");
}
