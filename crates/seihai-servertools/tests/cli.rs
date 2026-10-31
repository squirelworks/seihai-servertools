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
    // gserver_ip, site, upgrade, save.
    let (code, text) = setup(w.path(), &["host-listserver"], "\n\n\n\n\n\n\nyes\n");
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("bind: (not set) -> 0.0.0.0"), "{text}");
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
