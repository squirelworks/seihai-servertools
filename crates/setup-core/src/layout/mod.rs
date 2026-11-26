//! A new server folder from scratch (`setup new`): listservers, gservers
//! pointed at them, NPC-servers wired to the gservers that use them, staff
//! accounts for RC, and start and stop scripts, for the Rust stack
//! (seihai-listserver, seihai-gserver, seihai-npcserver, seihai-rc).
//!
//! | Part | What |
//! |---|---|
//! | [`plan`] | The plan and `layout.toml` |
//! | [`check`] | Names, links and ports, before anything is written |
//! | [`render`] | The text of every file |
//! | this module | Find the programs, write the folder, make the accounts |
//!
//! Nothing here connects anywhere. Passwords and tokens are random
//! ([`random_password`]); passwords are never written to a file.

pub mod check;
pub mod plan;
pub mod render;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use check::{check, is_loopback, may_write, port_busy, ports, PortUse};
pub use plan::{Basic, GserverPlan, ListRef, ListserverPlan, NpcMode, Plan, StaffPlan};

/// Where the server programs are. `None`: not found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bins {
    pub gserver: Option<PathBuf>,
    pub listserver: Option<PathBuf>,
    pub npcserver: Option<PathBuf>,
    pub rc: Option<PathBuf>,
}

/// The program's file name on this system (`seihai-gserver.exe` on Windows).
pub fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

impl Bins {
    /// Look for each program in `dirs`, then on `PATH`; first hit wins.
    pub fn find(dirs: &[PathBuf]) -> Bins {
        let path: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        let look = |name: &str| {
            let f = exe(name);
            dirs.iter()
                .chain(path.iter())
                .map(|d| d.join(&f))
                .find(|p| p.is_file())
                .map(|p| fs::canonicalize(&p).unwrap_or(p))
        };
        Bins {
            gserver: look("seihai-gserver"),
            listserver: look("seihai-listserver"),
            npcserver: look("seihai-npcserver"),
            rc: look("seihai-rc"),
        }
    }

    /// The folder the start scripts look in: the gserver's, else the
    /// first program found; empty when none was found (PATH).
    pub fn dir(&self) -> String {
        [&self.gserver, &self.listserver, &self.npcserver, &self.rc]
            .into_iter()
            .flatten()
            .find_map(|p| p.parent().map(|d| d.display().to_string()))
            .unwrap_or_default()
    }

    /// What the layout needs and was not found.
    pub fn missing(&self, plan: &Plan) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.gserver.is_none() {
            v.push("seihai-gserver");
        }
        if !plan.listservers.is_empty() && self.listserver.is_none() {
            v.push("seihai-listserver");
        }
        if plan.gservers.iter().any(|g| g.npcserver != NpcMode::Off) && self.npcserver.is_none() {
            v.push("seihai-npcserver");
        }
        v
    }
}

fn random_from(alphabet: &[u8], n: usize) -> Result<String, String> {
    // Rejection sampling, so every letter is equally likely.
    let limit = 256 - 256 % alphabet.len();
    let mut out = String::with_capacity(n);
    while out.len() < n {
        let mut buf = [0u8; 64];
        getrandom::getrandom(&mut buf).map_err(|e| format!("no random numbers: {e}"))?;
        for b in buf {
            if (b as usize) < limit && out.len() < n {
                out.push(alphabet[b as usize % alphabet.len()] as char);
            }
        }
    }
    Ok(out)
}

/// A random 16-letter password: letters and digits, without the ones that
/// look alike (0 O 1 l I), so it can be read off a screen.
pub fn random_password() -> Result<String, String> {
    random_from(
        b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789",
        16,
    )
}

/// A random 48-hex-digit NPC-server link token.
pub fn random_token() -> Result<String, String> {
    random_from(b"0123456789abcdef", 48)
}

/// The result of [`write`].
#[derive(Debug, Default)]
pub struct Written {
    /// Every file written, relative to the layout.
    pub files: Vec<String>,
    /// One line per account made.
    pub made: Vec<String>,
    /// Steps that could not be done here (a program was missing), as
    /// commands for the owner.
    pub todo: Vec<String>,
}

fn put(dir: &Path, rel: &str, text: &str, w: &mut Written) -> Result<(), String> {
    let p = dir.join(rel);
    if let Some(d) = p.parent() {
        fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    w.files.push(rel.to_string());
    Ok(())
}

#[cfg(unix)]
fn chmod(p: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(p, fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn chmod(_: &Path, _: u32) {}

/// Where the RC password goes for the account commands: an environment
/// variable, never the command line.
const PW_ENV: &str = "SEIHAI_SETUP_PASSWORD";

fn run_with_password(cmd: &mut Command, password: &str) -> Result<(), String> {
    let out = cmd
        .env(PW_ENV, password)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The lines that make an account staff with these rights.
pub fn staff_lines(s: &StaffPlan) -> String {
    let mut t = String::from("STAFF 1\n");
    if !s.ip_ranges.is_empty() {
        t.push_str(&format!("IPRANGE {}\n", s.ip_ranges.join(",")));
    }
    for f in &s.folders {
        t.push_str(&format!("FOLDER {f}\n"));
    }
    t
}

/// Make staff account `s` on gserver world `world` with `password`:
/// `seihai-gserver account`, then the staff lines added to its file.
pub fn make_staff(
    gserver: &Path,
    world: &Path,
    s: &StaffPlan,
    password: &str,
) -> Result<(), String> {
    run_with_password(
        Command::new(gserver)
            .arg("account")
            .arg(&s.account)
            .arg("--password-env")
            .arg(PW_ENV)
            .arg("--world")
            .arg(world),
        password,
    )?;
    let file = world
        .join("accounts")
        .join(format!("{}.txt", s.account.to_ascii_lowercase()));
    let mut text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let keep: Vec<&str> = text
        .lines()
        .filter(|l| {
            let k = l.split_whitespace().next().unwrap_or("");
            !matches!(k, "STAFF" | "IPRANGE" | "FOLDER")
        })
        .collect();
    text = keep.join("\n");
    text.push('\n');
    text.push_str(&staff_lines(s));
    fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))
}

/// Write the layout into `dir`, which must not exist or be empty, and make
/// the staff accounts with `passwords` (one per [`Plan::staff`] entry, in
/// order). The plan must have passed [`check`].
pub fn write(
    plan: &Plan,
    dir: &Path,
    bins: &Bins,
    passwords: &[String],
) -> Result<Written, String> {
    if passwords.len() != plan.staff.len() {
        return Err("one password per staff account".into());
    }
    if dir.exists()
        && fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .next()
            .is_some()
    {
        return Err(format!(
            "{} already has files in it; give a new or empty folder",
            dir.display()
        ));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut w = Written::default();
    let bin_dir = bins.dir();
    put(dir, "layout.toml", &plan.to_toml(), &mut w)?;
    put(dir, "README.txt", &render::readme(plan), &mut w)?;
    for l in &plan.listservers {
        let ld = render::listserver_dir(&l.name);
        put(
            dir,
            &format!("{ld}/seihai-listserver.toml"),
            &render::listserver_toml(plan, l),
            &mut w,
        )?;
    }
    let npc_bin = bins
        .npcserver
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    for g in &plan.gservers {
        let gd = render::gserver_dir(&g.name);
        let token = if g.npcserver == NpcMode::External {
            let t = random_token()?;
            let tf = render::token_file(&g.name);
            put(dir, &tf, &t, &mut w)?;
            chmod(&dir.join(&tf), 0o600);
            t
        } else {
            String::new()
        };
        let cfg = format!("{gd}/config/server.txt");
        put(
            dir,
            &cfg,
            &render::gserver_txt(plan, g, &npc_bin, &token),
            &mut w,
        )?;
        if !token.is_empty() {
            chmod(&dir.join(&cfg), 0o600);
        }
        put(
            dir,
            &format!("{gd}/config/allowedversions.txt"),
            "# Client builds allowed to log in; * is every build seihai-core knows.\n*\n",
            &mut w,
        )?;
        for sub in ["accounts", "levels", "files", "npcs"] {
            fs::create_dir_all(dir.join(&gd).join(sub)).map_err(|e| e.to_string())?;
        }
    }
    put(dir, "start.sh", &render::start_sh(plan, &bin_dir), &mut w)?;
    put(dir, "stop.sh", &render::stop_sh(plan), &mut w)?;
    chmod(&dir.join("start.sh"), 0o755);
    chmod(&dir.join("stop.sh"), 0o755);
    put(dir, "start.bat", &render::start_bat(plan, &bin_dir), &mut w)?;
    put(dir, "stop.bat", &render::stop_bat(plan), &mut w)?;
    fs::create_dir_all(dir.join("logs")).map_err(|e| e.to_string())?;

    for (s, pw) in plan.staff.iter().zip(passwords) {
        for g in plan.staff_gservers(s) {
            let world = dir.join(render::gserver_dir(&g.name));
            match &bins.gserver {
                Some(gs) => {
                    make_staff(gs, &world, s, pw)
                        .map_err(|e| format!("staff account {} on {}: {e}", s.account, g.name))?;
                    w.made.push(format!("{} (staff) on gserver {}", s.account, g.name));
                }
                None => w.todo.push(format!(
                    "{env}=<password> seihai-gserver account {a} --password-env {env} --world {gd}\n  then add these lines to {gd}/accounts/{f}.txt:\n{lines}",
                    env = PW_ENV,
                    a = s.account,
                    gd = render::gserver_dir(&g.name),
                    f = s.account.to_ascii_lowercase(),
                    lines = staff_lines(s).lines().map(|l| format!("    {l}")).collect::<Vec<_>>().join("\n"),
                )),
            }
        }
        for l in plan.staff_listservers(s) {
            let file = format!("{}/accounts.txt", render::listserver_dir(&l.name));
            match &bins.listserver {
                Some(ls) => {
                    run_with_password(
                        Command::new(ls)
                            .arg("add-account")
                            .arg(dir.join(&file))
                            .arg(&s.account)
                            .arg("--password-env")
                            .arg(PW_ENV),
                        pw,
                    )
                    .map_err(|e| format!("account {} on listserver {}: {e}", s.account, l.name))?;
                    w.made.push(format!("{} on listserver {}", s.account, l.name));
                }
                None => w.todo.push(format!(
                    "{PW_ENV}=<password> seihai-listserver add-account {file} {} --password-env {PW_ENV}",
                    s.account
                )),
            }
        }
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_are_random_and_readable() {
        let a = random_password().unwrap();
        let b = random_password().unwrap();
        assert_eq!(a.len(), 16);
        assert_ne!(a, b);
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && !"0O1lI".contains(c)));
        let t = random_token().unwrap();
        assert_eq!(t.len(), 48);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn writes_a_layout_without_programs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("srv");
        let plan = Plan::basic(&Basic {
            gservers: 2,
            npcserver: NpcMode::External,
            ..Default::default()
        });
        let pw = vec![random_password().unwrap()];
        let w = write(&plan, &dir, &Bins::default(), &pw).unwrap();
        for f in [
            "layout.toml",
            "README.txt",
            "listservers/list/seihai-listserver.toml",
            "gservers/world/config/server.txt",
            "gservers/world2/config/npcserver-token.txt",
            "start.sh",
            "stop.sh",
            "start.bat",
            "stop.bat",
        ] {
            assert!(dir.join(f).is_file(), "{f}");
        }
        assert!(dir.join("gservers/world/levels").is_dir());
        let token =
            fs::read_to_string(dir.join("gservers/world2/config/npcserver-token.txt")).unwrap();
        let cfg = fs::read_to_string(dir.join("gservers/world2/config/server.txt")).unwrap();
        assert!(cfg.contains(&format!("npcserver_token = {token}\n")));
        // No password anywhere in the folder.
        for f in &w.files {
            let t = fs::read_to_string(dir.join(f)).unwrap();
            assert!(!t.contains(&pw[0]), "{f}");
        }
        assert_eq!(w.todo.len(), 2, "{:?}", w.todo);
        assert!(w.made.is_empty());
        assert_eq!(
            Plan::from_toml(&fs::read_to_string(dir.join("layout.toml")).unwrap()).unwrap(),
            plan
        );
        // A second write into the same folder is refused.
        assert!(write(&plan, &dir, &Bins::default(), &pw).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = fs::metadata(dir.join("start.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(m & 0o111, 0o111);
        }
    }

    #[test]
    fn staff_lines_text() {
        let s = StaffPlan {
            account: "boss".into(),
            gservers: vec![],
            ip_ranges: vec!["127.0.0.1".into(), "10.0.0.0/8".into()],
            folders: vec!["rw levels/*".into()],
        };
        assert_eq!(
            staff_lines(&s),
            "STAFF 1\nIPRANGE 127.0.0.1,10.0.0.0/8\nFOLDER rw levels/*\n"
        );
    }
}
