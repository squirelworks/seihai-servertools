//! `seihai-setup new`: a full layout written into a temporary folder, and,
//! when the server programs are at hand, booted on 127.0.0.1.
//!
//! The boot test needs seihai-gserver, seihai-listserver, seihai-npcserver
//! and seihai-rc in the folder named by `SEIHAI_SUITE_BIN_DIR`; without it
//! the test says so and passes (CI has no server programs).

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn setup(args: &[&str], stdin: &str, path: Option<&str>) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_seihai-setup"));
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// `n` ports free on 127.0.0.1 right now.
fn free_ports(n: usize) -> Vec<u16> {
    let held: Vec<TcpListener> = (0..n)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    held.iter()
        .map(|l| l.local_addr().unwrap().port())
        .collect()
}

/// Two listservers, three gservers (NPC-server auto, external, off; the
/// third on the second listserver), two staff accounts.
fn plan(p: &[u16]) -> String {
    format!(
        r#"bind = "127.0.0.1"
[[listserver]]
name = "main"
gserver_port = {}
client_ports = [{}]
[[listserver]]
name = "side"
title = "Side list"
gserver_port = {}
client_ports = [{}]
accounts = "file"
[[gserver]]
name = "hub"
title = "Hub World"
port = {}
listserver = "main"
npcserver = "auto"
npcserver_port = {}
[[gserver]]
name = "arena"
title = "Arena"
port = {}
listserver = "main"
npcserver = "external"
npcserver_port = {}
[[gserver]]
name = "quiet"
title = "Quiet Place"
port = {}
listserver = "side"
npcserver = "off"
[[staff]]
account = "admin"
[[staff]]
account = "builder"
gservers = ["quiet"]
folders = ["rw levels/*"]
"#,
        p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8]
    )
}

/// The passwords printed once at the end, by account.
fn passwords(out: &str) -> Vec<(String, String)> {
    out.lines()
        .skip_while(|l| !l.starts_with("Staff passwords"))
        .skip(2)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| {
            let mut w = l.split_whitespace();
            Some((w.next()?.to_string(), w.next()?.to_string()))
        })
        .collect()
}

#[test]
fn help_and_print_plan() {
    let (code, text) = setup(&["new", "--help"], "", None);
    assert_eq!(code, Some(0));
    assert!(text.contains("--npcserver MODE"));
    let (code, text) = setup(&["new", "--print-plan", "--gservers", "2"], "", None);
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("[[gserver]]") && text.contains("port = 14812"));
    let (_, text) = setup(&["--help"], "", None);
    assert!(text.contains("setup new"));
}

#[test]
fn writes_a_full_layout_without_the_programs() {
    let tmp = tempfile::tempdir().unwrap();
    let plan_file = tmp.path().join("layout.toml");
    fs::write(&plan_file, plan(&free_ports(9))).unwrap();
    let dir = tmp.path().join("srv");
    let (code, out) = setup(
        &[
            "new",
            dir.to_str().unwrap(),
            "--plan",
            plan_file.to_str().unwrap(),
            "--yes",
        ],
        "",
        Some(""),
    );
    assert_eq!(code, Some(0), "{out}");
    for f in [
        "README.txt",
        "layout.toml",
        "start.sh",
        "stop.sh",
        "start.bat",
        "stop.bat",
        "listservers/main/seihai-listserver.toml",
        "listservers/side/seihai-listserver.toml",
        "gservers/hub/config/server.txt",
        "gservers/arena/config/npcserver-token.txt",
        "gservers/quiet/config/allowedversions.txt",
    ] {
        assert!(dir.join(f).is_file(), "{f}\n{out}");
    }
    let quiet = fs::read_to_string(dir.join("gservers/quiet/config/server.txt")).unwrap();
    let side = fs::read_to_string(dir.join("listservers/side/seihai-listserver.toml")).unwrap();
    let port = side
        .lines()
        .find_map(|l| l.strip_prefix("gserver_port = "))
        .unwrap();
    assert!(
        quiet.contains(&format!("listserver = 127.0.0.1:{port}\n")),
        "{quiet}"
    );
    assert!(quiet.contains("npcserver = off\n"));
    assert!(side.contains("accounts = \"accounts.txt\""));
    // The programs were not found: the passwords are still printed, with
    // the commands that make the accounts.
    let pw = passwords(&out);
    assert_eq!(pw.len(), 2, "{out}");
    assert!(out.contains("Still to do"));
    assert!(out.contains("seihai-gserver account admin"));
    assert!(out.contains("seihai-listserver add-account listservers/side/accounts.txt builder"));
    for (_, p) in &pw {
        for f in [
            "layout.toml",
            "README.txt",
            "gservers/hub/config/server.txt",
        ] {
            assert!(!fs::read_to_string(dir.join(f))
                .unwrap()
                .contains(p.as_str()));
        }
    }
    // Not over a folder that has files.
    let (code, out) = setup(&["new", dir.to_str().unwrap(), "--yes"], "", Some(""));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("already has files"));
}

#[test]
fn a_port_clash_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let mut p = free_ports(9);
    p[6] = p[4];
    let plan_file = tmp.path().join("layout.toml");
    fs::write(&plan_file, plan(&p)).unwrap();
    let dir = tmp.path().join("srv");
    let (code, out) = setup(
        &[
            "new",
            dir.to_str().unwrap(),
            "--plan",
            plan_file.to_str().unwrap(),
            "--yes",
        ],
        "",
        Some(""),
    );
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("is used twice"), "{out}");
    assert!(!dir.exists());
}

#[test]
fn asks_step_by_step() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("asked");
    let p = free_ports(5);
    // Folder, lab, 1 listserver, 2 gservers, staff, RC range, then no to
    // the defaults: listserver name, title, ports, accounts; each gserver's
    // name, title, port, listserver, NPC mode (and link port); then yes.
    let answers = format!(
        "{}\nlab\n1\n2\nboss\n127.0.0.1\nno\nhublist\n\n{}\n{}\nopen\nalpha\nAlpha\n{}\nhublist\nexternal\n{}\nbeta\n\n{}\nnone\noff\nyes\n",
        dir.display(), p[0], p[1], p[2], p[3], p[4]
    );
    let (code, out) = setup(&["new", "--no-port-check"], &answers, Some(""));
    assert_eq!(code, Some(0), "{out}");
    let toml = fs::read_to_string(dir.join("layout.toml")).unwrap();
    assert!(toml.contains("name = \"hublist\""), "{toml}");
    let a = fs::read_to_string(dir.join("gservers/alpha/config/server.txt")).unwrap();
    assert!(
        a.contains(&format!("listserver = 127.0.0.1:{}\n", p[0])),
        "{a}"
    );
    assert!(a.contains("npcserver = external\n"));
    let b = fs::read_to_string(dir.join("gservers/beta/config/server.txt")).unwrap();
    assert!(
        b.contains("listserver =\n") && b.contains("npcserver = off\n"),
        "{b}"
    );
    assert_eq!(passwords(&out)[0].0, "boss");
}

fn wait_for(file: &Path, text: &str, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    while Instant::now() < end {
        if fs::read_to_string(file).is_ok_and(|t| t.contains(text)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

/// Stops the layout when the test ends, passing or not.
struct Running(PathBuf);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = Command::new(self.0.join("stop.sh")).output();
    }
}

#[cfg(unix)]
#[test]
fn boots_on_loopback() {
    let Some(bin) = std::env::var_os("SEIHAI_SUITE_BIN_DIR").map(PathBuf::from) else {
        eprintln!("boots_on_loopback: SEIHAI_SUITE_BIN_DIR not set; not booting the servers");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let plan_file = tmp.path().join("layout.toml");
    fs::write(&plan_file, plan(&free_ports(9))).unwrap();
    let dir = tmp.path().join("srv");
    let (code, out) = setup(
        &[
            "new",
            dir.to_str().unwrap(),
            "--plan",
            plan_file.to_str().unwrap(),
            "--bin-dir",
            bin.to_str().unwrap(),
            "--yes",
        ],
        "",
        None,
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("account made: admin (staff) on gserver hub"),
        "{out}"
    );
    assert!(
        out.contains("account made: builder on listserver side"),
        "{out}"
    );
    assert!(!out.contains("Still to do"), "{out}");
    let pw = passwords(&out);

    let start = Command::new(dir.join("start.sh")).output().unwrap();
    assert!(start.status.success());
    let _running = Running(dir.clone());
    let logs = dir.join("logs");
    let limit = Duration::from_secs(20);
    for g in ["hub", "arena", "quiet"] {
        let log = logs.join(format!("gserver-{g}.log"));
        assert!(
            wait_for(&log, ": registered", limit),
            "{g} did not register:\n{}",
            fs::read_to_string(&log).unwrap_or_default()
        );
    }
    for g in ["hub", "arena"] {
        let log = logs.join(format!("gserver-{g}.log"));
        assert!(
            wait_for(&log, "NPC-server connected", limit),
            "{g}: no NPC-server:\n{}",
            fs::read_to_string(&log).unwrap_or_default()
        );
    }
    assert!(!fs::read_to_string(logs.join("gserver-quiet.log"))
        .unwrap()
        .contains("NPC-server"));
    assert!(wait_for(
        &logs.join("listserver-main.log"),
        "newserver Hub World",
        limit
    ));
    assert!(wait_for(
        &logs.join("listserver-main.log"),
        "newserver Arena",
        limit
    ));
    assert!(wait_for(
        &logs.join("listserver-side.log"),
        "newserver Quiet Place",
        limit
    ));

    // The staff accounts log in with seihai-rc.
    let layout = fs::read_to_string(dir.join("layout.toml")).unwrap();
    let port_of = |name: &str| -> String {
        let t = layout
            .split("[[gserver]]")
            .find(|t| t.contains(&format!("name = \"{name}\"")))
            .unwrap();
        t.lines()
            .find_map(|l| l.strip_prefix("port = "))
            .unwrap()
            .to_string()
    };
    let rc = |account: &str, password: &str, port: &str| {
        Command::new(bin.join("seihai-rc"))
            .args([
                "players",
                "list",
                "--server",
                &format!("127.0.0.1:{port}"),
                "--account",
                account,
            ])
            .env("SEIHAI_RC_PASSWORD", password)
            .output()
            .unwrap()
    };
    let admin = &pw.iter().find(|(a, _)| a == "admin").unwrap().1;
    let builder = &pw.iter().find(|(a, _)| a == "builder").unwrap().1;
    for g in ["hub", "arena", "quiet"] {
        let o = rc("admin", admin, &port_of(g));
        assert!(
            o.status.success(),
            "admin on {g}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    assert!(rc("builder", builder, &port_of("quiet")).status.success());
    // builder is staff on quiet only, and a wrong password is refused.
    assert!(!rc("builder", builder, &port_of("hub")).status.success());
    assert!(!rc("admin", "not-the-password", &port_of("hub"))
        .status
        .success());

    // The RC list of the main listserver shows its two gservers.
    let main = fs::read_to_string(dir.join("listservers/main/seihai-listserver.toml")).unwrap();
    let cport = main
        .lines()
        .find_map(|l| l.strip_prefix("client_ports = ["))
        .unwrap()
        .trim_end_matches(']')
        .to_string();
    let o = Command::new(bin.join("seihai-rc"))
        .args([
            "servers",
            "--listserver",
            &format!("127.0.0.1:{cport}"),
            "--account",
            "admin",
        ])
        .env("SEIHAI_RC_PASSWORD", admin)
        .output()
        .unwrap();
    let list = String::from_utf8_lossy(&o.stdout);
    assert!(
        list.contains("Hub World") && list.contains("Arena"),
        "{list}"
    );
    assert!(!list.contains("Quiet Place"), "{list}");

    let stop = Command::new(dir.join("stop.sh")).output().unwrap();
    assert!(stop.status.success());
    let stopped = String::from_utf8_lossy(&stop.stdout);
    assert!(
        stopped.contains("stopped gserver-hub") && stopped.contains("stopped listserver-main"),
        "{stopped}"
    );
}
