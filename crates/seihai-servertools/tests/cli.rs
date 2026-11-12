//! The command line, run as a program.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
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
fn capacity_edits_only_the_selected_gserver_format_and_can_undo() {
    let w = lab_world();
    let fork = w.path().join("servers/default/config/serveroptions.txt");
    let original = fs::read_to_string(&fork).unwrap();
    fs::create_dir_all(w.path().join("config")).unwrap();
    let rust = w.path().join("config/server.txt");
    fs::write(&rust, "# Rust config\nmax_players = 256 # players\n").unwrap();
    for kind in ["fork", "rust"] {
        let out = bin()
            .args(["setup", "capacity", "--kind", kind, "--world"])
            .arg(w.path())
            .args(["--max-players", "1200", "--yes"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{text} {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("Backup:"));
        assert!(text.contains("max_connections"));
        if kind == "fork" {
            assert!(fs::read_to_string(&fork)
                .unwrap()
                .ends_with("maxplayers = 1200\n"));
            assert_eq!(
                fs::read_to_string(&rust).unwrap(),
                "# Rust config\nmax_players = 256 # players\n"
            );
        } else {
            assert!(fs::read_to_string(&rust)
                .unwrap()
                .ends_with("max_players = 1200 # players\n"));
        }
        let out = bin()
            .args(["setup", "capacity", "--kind", kind, "--world"])
            .arg(w.path())
            .args(["--undo", "--yes"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_eq!(fs::read_to_string(fork).unwrap(), original);
    assert_eq!(
        fs::read_to_string(rust).unwrap(),
        "# Rust config\nmax_players = 256 # players\n"
    );
}

#[test]
fn capacity_requires_confirmation_and_rejects_invalid_or_ambiguous_requests() {
    let w = lab_world();
    let path = w.path().join("servers/default/config/serveroptions.txt");
    let original = fs::read(&path).unwrap();
    for extra in [
        vec!["--kind", "fork", "--max-players", "1000"],
        vec!["--kind", "fork", "--max-players", "0", "--yes"],
        vec!["--kind", "fork", "--max-players", "-1", "--yes"],
        vec![
            "--kind",
            "rust",
            "default",
            "--max-players",
            "1000",
            "--yes",
        ],
        vec!["--max-players", "1000", "--yes"],
        vec!["--kind", "fork", "--max-connections", "1000", "--yes"],
    ] {
        let out = bin()
            .args(["setup", "capacity", "--world"])
            .arg(w.path())
            .args(extra)
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}

#[test]
fn lab_plan_prints_the_servers() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    fs::write(
        w.join(format!("gs2emu{}", std::env::consts::EXE_SUFFIX)),
        b"",
    )
    .unwrap();
    let cfg = w.join("servers/default/config");
    fs::create_dir_all(&cfg).unwrap();
    fs::write(
        cfg.join("serveroptions.txt"),
        "serverinterface = 127.0.0.1\nserverport = 14802\n",
    )
    .unwrap();
    let out = bin()
        .args(["lab", "plan", "--no-listserver", "--world"])
        .arg(w)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("gserver     default on 127.0.0.1:14802"),
        "{text}"
    );
}

#[test]
fn check_fails_on_a_broken_level() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bad.nw"), "nope").unwrap();
    let out = bin().arg("check").arg(dir.path()).output().unwrap();
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
    let normal = bin().arg("check").arg(dir.path()).output().unwrap();
    assert!(normal.status.success());
    assert!(!String::from_utf8_lossy(&normal.stdout).contains("client reach:"));
    for json in [false, true] {
        let mut cmd = bin();
        cmd.args(["check", "--client-reach"]).arg(dir.path());
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
    let normal = bin().arg("check").arg(dir.path()).output().unwrap();
    assert!(normal.status.success());
    assert!(!String::from_utf8_lossy(&normal.stdout).contains("fallback review"));
    for json in [false, true] {
        let mut cmd = bin();
        cmd.args(["check", "--client-reach"]).arg(dir.path());
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
    for args in [["check", "--json"], ["check", ""]] {
        let mut cmd = bin();
        cmd.arg(args[0]);
        if !args[1].is_empty() {
            cmd.arg(args[1]);
        }
        cmd.arg(w.path());
        if args[1].is_empty() {
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
    let out = bin().arg("check").arg(w.path()).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "lab defaults are fine: {text}");
    assert!(text.contains("and the server settings: 0 errors"), "{text}");

    let cfg = w.path().join("servers/default/config/serveroptions.txt");
    fs::write(
        &cfg,
        LAB.replace("serverinterface = 127.0.0.1", "serverinterface = AUTO"),
    )
    .unwrap();
    let out = bin().arg("check").arg(w.path()).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains("error: servers/default/config/serveroptions.txt [localhost]: "),
        "{text}"
    );
}

const LAB: &str = "# lab world\nname = My Server\nserverinterface = 127.0.0.1\nserverip = 127.0.0.1\nlocalip = 127.0.0.1\nserverport = 14802\nlistip = 127.0.0.1\nlistport = 14900\nupnp = false\nstaff = (Manager),YOURACCOUNT\nlocalhost = true\n";

fn lab_world() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("servers/default/config");
    fs::create_dir_all(&cfg).unwrap();
    fs::write(cfg.join("serveroptions.txt"), LAB).unwrap();
    dir
}

fn options(w: &std::path::Path) -> String {
    fs::read_to_string(w.join("servers/default/config/serveroptions.txt")).unwrap()
}

fn setup(w: &std::path::Path, args: &[&str], stdin: &str) -> (Option<i32>, String) {
    use std::io::Write;
    let mut child = bin()
        .arg("setup")
        .args(args)
        .arg("--world")
        .arg(w)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

#[test]
fn setup_public_by_prompts_then_lab_then_undo() {
    let w = lab_world();
    // account, name, description, url, interface, server ip, listserver, upnp, save
    let answers = "owner\nOwner's World\nA test world\n\n\n\n127.0.0.1:14900\nno\nyes\n";
    let (code, text) = setup(w.path(), &["public"], answers);
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("localhost: true -> false"), "{text}");
    assert!(
        text.contains("This machine's addresses:") && text.contains("127.0.0.1 ("),
        "the Listen on question lists the addresses: {text}"
    );
    let o = options(w.path());
    assert!(o.contains("serverinterface = AUTO\n"), "{o}");
    assert!(o.contains("staff = (Manager),owner\n"), "{o}");
    assert!(o.contains("localhost = false\n"), "{o}");
    assert!(o.starts_with("# lab world\n"), "comments kept: {o}");

    let (code, text) = setup(w.path(), &["status"], "");
    assert_eq!(code, Some(0));
    assert!(text.contains(": Public"), "{text}");

    let (code, text) = setup(w.path(), &["lab", "--yes"], "");
    assert_eq!(code, Some(0), "{text}");
    let o = options(w.path());
    assert!(
        o.contains("serverinterface = 127.0.0.1\n") && o.contains("localhost = true\n"),
        "{o}"
    );

    let (code, text) = setup(w.path(), &["undo"], "");
    assert_eq!(code, Some(0), "{text}");
    assert!(options(w.path()).contains("localhost = false\n"));
}

#[test]
fn setup_public_refuses_a_placeholder_listserver() {
    let w = lab_world();
    let (code, text) = setup(
        w.path(),
        &[
            "public",
            "--yes",
            "--account",
            "owner",
            "--name",
            "World",
            "--listserver",
            "<listserver-host>",
        ],
        "",
    );
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("BLOCKED [listip]"), "{text}");
    assert_eq!(options(w.path()), LAB, "nothing written");
}

#[test]
fn setup_public_with_yes_needs_the_required_answers() {
    let w = lab_world();
    let (code, text) = setup(w.path(), &["public", "--yes"], "");
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("--yes asks nothing"), "{text}");
    assert_eq!(options(w.path()), LAB);
}

#[test]
fn setup_saying_no_writes_nothing() {
    let w = lab_world();
    let answers = "owner\nWorld\n\n\n\n\n127.0.0.1\nno\nno\n";
    let (code, text) = setup(w.path(), &["public"], answers);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("Not saved."), "{text}");
    assert_eq!(options(w.path()), LAB);
}

/// A stand-in gserver: prints the console lines the fork prints when it
/// starts and links to the listserver, then waits.
fn stand_in(dir: &std::path::Path) -> std::path::PathBuf {
    if cfg!(windows) {
        let p = dir.join("gs2emu-standin.bat");
        fs::write(
            &p,
            "@echo off\r\necho Listening on: 0.0.0.0:14802.\r\necho listserver - Connected to 127.0.0.1:14900.\r\nping -n 30 127.0.0.1 > nul\r\n",
        )
        .unwrap();
        p
    } else {
        let p = dir.join("gs2emu-standin");
        fs::write(
            &p,
            "#!/bin/sh\necho 'Listening on: 0.0.0.0:14802.'\necho 'listserver - Connected to 127.0.0.1:14900.'\nexec sleep 30\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }
}

#[test]
fn setup_start_shows_the_lights() {
    let w = lab_world();
    let g = stand_in(w.path());
    let (code, text) = setup(
        w.path(),
        &["start", "--for", "3", "--gserver", g.to_str().unwrap()],
        "",
    );
    assert_eq!(code, Some(0), "{text}");
    assert!(
        text.contains("Listening on:            [ ok ] 0.0.0.0:14802"),
        "{text}"
    );
    assert!(
        text.contains("Listserver:              [ ok ] connected to 127.0.0.1:14900"),
        "{text}"
    );
    assert!(text.contains("Reachable from outside:  [ ?? ]"), "{text}");
}

#[test]
fn setup_test_connection_on_loopback() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let out = bin()
        .args(["setup", "test-connection", &format!("127.0.0.1:{port}")])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains(": open"));
}

#[test]
fn setup_host_listserver_then_back_to_lab() {
    let w = lab_world();
    let toml = w.path().join("seihai-listserver.toml");
    fs::write(&toml, "# my listserver\nlist_name = \"Lab listserver\"\n").unwrap();
    // Open mode on every interface is refused.
    let (code, text) = setup(
        w.path(),
        &["host-listserver", "--yes", "--accounts", ""],
        "",
    );
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("BLOCKED [accounts]"), "{text}");
    // Defaults by prompts: accounts, bind, gserver port, client ports,
    // max connections, silent list, who may register (127.0.0.1) and its
    // HQ password, gserver_ip, site, upgrade, save.
    let (code, text) = setup(
        w.path(),
        &["host-listserver"],
        "\n\n\n\n\n\n\nlab-hq-pw\n\n\n\nyes\n",
    );
    assert!(
        text.contains("silent_list_after_ms: (not set) -> 500"),
        "{text}"
    );
    assert!(
        !text.contains("max_connections:"),
        "default not written: {text}"
    );
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("bind: (not set) -> 0.0.0.0"), "{text}");
    assert!(
        text.contains("gservers: (not set) -> 127.0.0.1 (HQ password set)"),
        "{text}"
    );
    assert!(
        !text.contains("lab-hq-pw"),
        "password never echoed back: {text}"
    );
    assert!(!text.contains("warning [gservers]"), "{text}");
    assert!(text.contains("does not exist yet"), "{text}");
    assert!(text.contains("--listserver 127.0.0.1:14900"), "{text}");
    let t = fs::read_to_string(&toml).unwrap();
    assert!(
        t.starts_with("# my listserver\nlist_name = \"Lab listserver\"\n"),
        "{t}"
    );
    assert!(
        t.contains("bind = \"0.0.0.0\"") && t.contains("accounts = \"accounts.txt\""),
        "{t}"
    );
    assert!(
        t.contains("gservers = [{ ip = \"127.0.0.1\", hq_password = \"lab-hq-pw\" }]"),
        "{t}"
    );

    let (code, text) = setup(w.path(), &["listserver-lab", "--yes"], "");
    assert_eq!(code, Some(0), "{text}");
    assert!(fs::read_to_string(&toml)
        .unwrap()
        .contains("bind = \"127.0.0.1\""));
    let (code, _) = setup(w.path(), &["listserver-undo"], "");
    assert_eq!(code, Some(0));
    assert!(fs::read_to_string(&toml)
        .unwrap()
        .contains("bind = \"0.0.0.0\""));
}

#[test]
fn host_listserver_who_may_register_by_options() {
    let w = lab_world();
    let toml = w.path().join("seihai-listserver.toml");
    let base = ["host-listserver", "--yes", "--accounts", "accounts.txt"];

    // --yes with neither option leaves the list alone and warns.
    let (code, text) = setup(w.path(), &base, "");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("warning [gservers]"), "{text}");
    assert!(!fs::read_to_string(&toml).unwrap().contains("gservers"));

    // A new address needs the password from the environment.
    let mut args = base.to_vec();
    args.extend(["--allow-gserver", "127.0.0.1"]);
    let (code, text) = setup(w.path(), &args, "");
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("--hq-password-env"), "{text}");

    let mut child = bin();
    child
        .arg("setup")
        .args(&args)
        .args([
            "--allow-gserver",
            "::1",
            "--hq-password-env",
            "SEIHAI_TEST_HQ",
        ])
        .arg("--world")
        .arg(w.path())
        .env("SEIHAI_TEST_HQ", "env-hq-pw");
    let out = child.output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(out.status.success(), "{text}");
    assert!(!text.contains("env-hq-pw"), "{text}");
    assert!(
        text.contains("note [gservers]: Only 127.0.0.1, ::1 may register"),
        "{text}"
    );
    let t = fs::read_to_string(&toml).unwrap();
    assert!(
        t.contains("gservers = [{ ip = \"127.0.0.1\", hq_password = \"env-hq-pw\" }, { ip = \"::1\", hq_password = \"env-hq-pw\" }]"),
        "{t}"
    );

    // An address already listed keeps its password without the variable.
    let (code, text) = setup(w.path(), &args, "");
    assert_eq!(code, Some(0), "{text}");
    assert!(fs::read_to_string(&toml)
        .unwrap()
        .contains("gservers = [{ ip = \"127.0.0.1\", hq_password = \"env-hq-pw\" }]"),);

    // Not an address: refused before anything is written.
    let mut bad = base.to_vec();
    bad.extend(["--allow-gserver", "my-gserver"]);
    let (code, text) = setup(w.path(), &bad, "");
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("not an IP address"), "{text}");

    // --any-gserver opens it again.
    let mut open = base.to_vec();
    open.push("--any-gserver");
    let (code, text) = setup(w.path(), &open, "");
    assert_eq!(code, Some(0), "{text}");
    assert!(fs::read_to_string(&toml).unwrap().contains("gservers = []"));
    assert!(text.contains("warning [gservers]"), "{text}");
}

#[test]
fn host_listserver_capacity_and_silent_list_options() {
    let w = lab_world();
    let toml = w.path().join("seihai-listserver.toml");
    let base = ["host-listserver", "--yes", "--accounts", "accounts.txt"];
    let mut args = base.to_vec();
    args.extend(["--max-connections", "1200", "--silent-list-ms", "750"]);
    let (code, text) = setup(w.path(), &args, "");
    assert_eq!(code, Some(0), "{text}");
    let t = fs::read_to_string(&toml).unwrap();
    assert!(t.contains("max_connections = 1200\n"), "{t}");
    assert!(t.contains("silent_list_after_ms = 750\n"), "{t}");
    assert!(!text.contains("note [silent_list_after_ms]"), "{text}");

    // --yes keeps what the file has.
    let (code, _) = setup(w.path(), &base, "");
    assert_eq!(code, Some(0));
    let t = fs::read_to_string(&toml).unwrap();
    assert!(
        t.contains("max_connections = 1200\n") && t.contains("silent_list_after_ms = 750\n"),
        "{t}"
    );

    // Off again: the review says what 1.41 rev 1 loses.
    let mut off = base.to_vec();
    off.extend(["--silent-list-ms", "0"]);
    let (code, text) = setup(w.path(), &off, "");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("note [silent_list_after_ms]"), "{text}");

    let mut bad = base.to_vec();
    bad.extend(["--max-connections", "0"]);
    let (code, text) = setup(w.path(), &bad, "");
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("at least 1"), "{text}");
}

#[test]
fn hq_password_is_written_from_the_environment() {
    let w = lab_world();
    let admin = w.path().join("servers/default/config/adminconfig.txt");
    fs::write(
        &admin,
        "# ServerHQ password for the server.\nhq_password = \nhq_level = 1\n",
    )
    .unwrap();

    let (code, text) = setup(w.path(), &["hq-password"], "");
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("--hq-password-env"), "{text}");

    let out = bin()
        .args([
            "setup",
            "hq-password",
            "--hq-password-env",
            "SEIHAI_TEST_HQ",
            "--world",
        ])
        .arg(w.path())
        .env("SEIHAI_TEST_HQ", "gs-hq-pw")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Saved the HQ password"), "{text}");
    assert!(!text.contains("gs-hq-pw"), "{text}");
    let a = fs::read_to_string(&admin).unwrap();
    assert!(a.contains("hq_password = gs-hq-pw\nhq_level = 1\n"), "{a}");
}

/// `gpak pack --server` puts a `.gpake` under `world/gpaks/` and its key in
/// `config/gpakkeys.txt`; `gpak unpack --server` gets the files back.
#[test]
fn gpak_pack_and_unpack_through_a_server_folder() {
    let src = tempfile::tempdir().unwrap();
    fs::create_dir_all(src.path().join("tiles")).unwrap();
    fs::write(src.path().join("tiles/Tile.png"), vec![7u8; 100]).unwrap();
    fs::write(src.path().join("note.txt"), b"hello").unwrap();
    let world = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["gpak", "pack"])
        .arg(src.path())
        .args(["art.gpake", "--server"])
        .arg(world.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pkg = world.path().join("world/gpaks/art.gpake");
    assert!(pkg.is_file());
    let keys = fs::read_to_string(world.path().join("config/gpakkeys.txt")).unwrap();
    assert!(keys.lines().any(|l| l.starts_with("art.gpake ")), "{keys}");
    let back = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["gpak", "unpack"])
        .arg(&pkg)
        .arg(back.path())
        .arg("--server")
        .arg(world.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read(back.path().join("tiles/tile.png")).unwrap(),
        vec![7u8; 100]
    );
    assert_eq!(fs::read(back.path().join("note.txt")).unwrap(), b"hello");
    // Without the key, an encrypted package is refused.
    let out = bin()
        .args(["gpak", "unpack"])
        .arg(&pkg)
        .arg(back.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
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
        .args(["code", "pack", "--key", key, "--out"])
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
        .args(["code", "crc", "--key", key])
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
        .args(["code", "show", "--key", key])
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
        .args(["code", "dump", "--key", key, "--out"])
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
        .args(["code", "pack", "--out"])
        .arg(&level_code)
        .arg(&level_plain)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let show = bin()
        .args(["code", "show"])
        .arg(&level_code)
        .output()
        .unwrap();
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
            .args(["code", "crc"])
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
        .args(["code", "show"])
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
        let out = bin().arg("code").args(args).arg(&path).output().unwrap();
        assert!(!out.status.success());
    }
    assert_eq!(fs::read(path).unwrap(), file);
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
        .args(["check", "--make-gif", "--json"])
        .arg(w)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(2));
    assert!(!w.join("images/tree.gif").exists());

    let out = bin()
        .args(["check", "--client-reach", "--make-gif"])
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
    let again = bin().args(["check", "--make-gif"]).arg(w).output().unwrap();
    assert!(!String::from_utf8_lossy(&again.stdout).contains("wrote "));
}
