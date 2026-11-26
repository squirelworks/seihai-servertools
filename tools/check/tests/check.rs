//! `seihai-check`, run as a program on its own.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-check"))
}

const LAB: &str = "# lab world\nname = My Server\nserverinterface = 127.0.0.1\nserverip = 127.0.0.1\nlocalip = 127.0.0.1\nserverport = 14802\nlistip = 127.0.0.1\nlistport = 14900\nupnp = false\nstaff = (Manager),YOURACCOUNT\nlocalhost = true\n";

fn lab_world() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("servers/default/config");
    fs::create_dir_all(&cfg).unwrap();
    fs::write(cfg.join("serveroptions.txt"), LAB).unwrap();
    dir
}

#[test]
fn check_fails_on_a_broken_level() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bad.nw"), "nope").unwrap();
    let out = bin().arg(dir.path()).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("error: bad.nw"));
}

#[test]
fn client_reach_is_optional_and_included_in_json() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("start.nw"),
        "GLEVNW01\nNPC tree.png 1 1\nNPCEND\n",
    )
    .unwrap();
    fs::write(dir.path().join("tree.png"), b"art").unwrap();
    let normal = bin().arg(dir.path()).output().unwrap();
    assert!(normal.status.success());
    assert!(!String::from_utf8_lossy(&normal.stdout).contains("client reach:"));
    for json in [false, true] {
        let mut cmd = bin();
        cmd.args(["--client-reach"]).arg(dir.path());
        if json {
            cmd.arg("--json");
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(text.contains("client reach:"), "{text}");
        assert!(text.contains("tree.gif"), "{text}");
        // start.nw is an NW level, which the oldest profiles cannot open.
        assert!(text.contains("client reach: NW level"), "{text}");
        if json {
            assert_eq!(text.lines().count(), 1);
            assert!(text.contains("\"warnings\":2"), "{text}");
        }
    }
    assert!(!dir.path().join("tree.gif").exists());
}

#[test]
fn client_reach_reports_gani_fallback_in_text_and_json_only_when_requested() {
    let dir = tempfile::tempdir().unwrap();
    let source = "SPRITE 0 BODY 0 0 16 16\nSINGLEDIRECTION\nANI\n0 0 0\nANIEND\n";
    fs::write(dir.path().join("idle.gani"), source).unwrap();
    let normal = bin().arg(dir.path()).output().unwrap();
    assert!(normal.status.success());
    assert!(!String::from_utf8_lossy(&normal.stdout).contains("fallback review"));
    for json in [false, true] {
        let mut cmd = bin();
        cmd.args(["--client-reach"]).arg(dir.path());
        if json {
            cmd.arg("--json");
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(text.contains("idle.gani"), "{text}");
        assert!(
            text.contains("server animation fallback review if it is used"),
            "{text}"
        );
        if json {
            assert_eq!(text.lines().count(), 1);
            assert!(text.contains("\"ganis\":1"), "{text}");
            assert!(text.contains("\"warnings\":1"), "{text}");
        }
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("idle.gani")).unwrap(),
        source
    );
}

#[test]
fn check_json_reports_both_kinds_of_findings() {
    let w = lab_world();
    fs::write(w.path().join("bad.nw"), "nope").unwrap();
    let cfg = w.path().join("servers/default/config/serveroptions.txt");
    fs::write(
        &cfg,
        LAB.replace("serverinterface = 127.0.0.1", "serverinterface = AUTO"),
    )
    .unwrap();
    for json_first in [true, false] {
        let mut cmd = bin();
        if json_first {
            cmd.arg("--json");
        }
        cmd.arg(w.path());
        if !json_first {
            cmd.arg("--json");
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(1), "{text}");
        let text = text.trim();
        assert_eq!(text.lines().count(), 1, "one object: {text}");
        assert!(
            text.starts_with("{\"format\":1,\"levels\":1,\"gmaps\":0,\"ganis\":0,\"errors\":"),
            "{text}"
        );
        assert!(text.ends_with("]}"), "{text}");
        assert!(
            text.contains("{\"level\":\"error\",\"source\":\"world\",\"file\":\"bad.nw\",\"key\":null,\"text\":\"level does not parse"),
            "{text}"
        );
        assert!(
            text.contains("{\"level\":\"error\",\"source\":\"settings\",\"file\":\"servers/default/config/serveroptions.txt\",\"key\":\"localhost\",\"text\":\"localhost = true lets"),
            "{text}"
        );
    }
}

#[test]
fn check_runs_the_setup_checks_on_server_folders() {
    let w = lab_world();
    let out = bin().arg(w.path()).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "lab defaults are fine: {text}");
    assert!(text.contains("and the server settings: 0 errors"), "{text}");

    let cfg = w.path().join("servers/default/config/serveroptions.txt");
    fs::write(
        &cfg,
        LAB.replace("serverinterface = 127.0.0.1", "serverinterface = AUTO"),
    )
    .unwrap();
    let out = bin().arg(w.path()).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains("error: servers/default/config/serveroptions.txt [localhost]: "),
        "{text}"
    );
}

#[test]
fn client_reach_lists_gs2_level_npcs_and_new_tiles() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("start.nw"),
        "GLEVNW01\nBOARD 0 0 1 0 4E\nNPC - 1 1\nfunction onCreated() { this.x = 1; }\nNPCEND\n",
    )
    .unwrap();
    let normal = bin().arg(dir.path()).output().unwrap();
    assert!(!String::from_utf8_lossy(&normal.stdout).contains("GS2 only"));
    for json in [false, true] {
        let mut cmd = bin();
        cmd.args(["--client-reach"]).arg(dir.path());
        if json {
            cmd.arg("--json");
        }
        let out = cmd.output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{text}");
        assert!(text.contains("NPC at 1,1 script is GS2 only"), "{text}");
        assert!(text.contains("tiles past column 116"), "{text}");
    }
}

#[test]
fn make_gif_writes_only_the_copies_client_reach_asks_for() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::create_dir_all(w.join("images")).unwrap();
    fs::write(
        w.join("start.nw"),
        "GLEVNW01\nNPC tree.png 1 1\nNPCEND\nNPC rock.png 2 2\nNPCEND\n",
    )
    .unwrap();
    // A real 1x1 PNG for the tree; the rock already has a GIF; an unused PNG.
    let mut png_bytes = Vec::new();
    {
        let mut e = png::Encoder::new(&mut png_bytes, 1, 1);
        e.set_color(png::ColorType::Rgb);
        e.write_header()
            .unwrap()
            .write_image_data(&[10, 20, 30])
            .unwrap();
    }
    fs::write(w.join("images/tree.png"), &png_bytes).unwrap();
    fs::write(w.join("images/rock.png"), &png_bytes).unwrap();
    fs::write(w.join("images/ROCK.GIF"), b"old").unwrap();
    fs::write(w.join("images/unused.png"), &png_bytes).unwrap();

    let refused = bin()
        .args(["--make-gif", "--json"])
        .arg(w)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(!w.join("images/tree.gif").exists());

    let out = bin()
        .args(["--client-reach", "--make-gif"])
        .arg(w)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("wrote ") && text.contains("tree.gif"),
        "{text}"
    );
    // After the copy, the reach report has nothing to say about the tree.
    assert!(!text.contains("no tree.gif counterpart"), "{text}");
    assert!(fs::read(w.join("images/tree.gif"))
        .unwrap()
        .starts_with(b"GIF8"));
    assert_eq!(fs::read(w.join("images/ROCK.GIF")).unwrap(), b"old");
    // Only tree.gif was added: no rock copy, no copy of the unused PNG.
    assert_eq!(fs::read_dir(w.join("images")).unwrap().count(), 5);

    // A second run has nothing left to write.
    let again = bin().args(["--make-gif"]).arg(w).output().unwrap();
    assert!(!String::from_utf8_lossy(&again.stdout).contains("wrote "));
}

#[test]
fn check_client_selection_is_explicit_and_keeps_json_format_one() {
    let out = bin().args(["--list-clients"]).output().unwrap();
    assert!(out.status.success());
    let labels = String::from_utf8(out.stdout).unwrap();
    assert!(labels.lines().any(|l| l == "2.22"));
    assert!(labels.lines().any(|l| l == "1.39"));
    let w = tempfile::tempdir().unwrap();
    fs::write(w.path().join("start.nw"), "GLEVNW01\n").unwrap();
    for (extra, selected) in [(vec![], false), (vec!["--client", "1.39"], true)] {
        let out = bin()
            .args(["--json"])
            .args(extra)
            .arg(w.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8(out.stdout).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert!(text.starts_with("{\"format\":1,\"levels\":1,\"gmaps\":0,\"ganis\":0,"));
        assert!(text.ends_with("]}\n"));
        assert_eq!(text.contains("client profile 1.39"), selected, "{text}");
        assert_eq!(text.contains("NW level format"), selected, "{text}");
    }
}

#[test]
fn invalid_check_client_arguments_fail_before_optional_gif_writes() {
    let w = tempfile::tempdir().unwrap();
    fs::write(w.path().join("art.png"), b"not decoded on invalid input").unwrap();
    for extra in [
        vec!["--client"],
        vec!["--client", "unknown"],
        vec!["--client", "2.22", "--client", "1.39"],
        vec!["--list-clients"],
        vec!["--client", "GNW13110"],
    ] {
        let out = bin()
            .args(["--make-gif"])
            .arg(w.path())
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert!(!w.path().join("art.gif").exists());
    }
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-check"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-check "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-check"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("--json"), "{text}");
}
