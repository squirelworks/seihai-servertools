//! `seihai-setup`, run as a program on its own.

use std::fs;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-setup"))
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
fn capacity_edits_only_the_selected_gserver_format_and_can_undo() {
    let w = lab_world();
    let fork = w.path().join("servers/default/config/serveroptions.txt");
    let original = fs::read_to_string(&fork).unwrap();
    fs::create_dir_all(w.path().join("config")).unwrap();
    let rust = w.path().join("config/server.txt");
    fs::write(&rust, "# Rust config\nmax_players = 256 # players\n").unwrap();
    for kind in ["fork", "rust"] {
        let out = bin()
            .args(["capacity", "--kind", kind, "--world"])
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
            .args(["capacity", "--kind", kind, "--world"])
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
            .args(["capacity", "--world"])
            .arg(w.path())
            .args(extra)
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
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
        .args(["test-connection", &format!("127.0.0.1:{port}")])
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

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-setup"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-setup "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-setup"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("public"), "{text}");
}
