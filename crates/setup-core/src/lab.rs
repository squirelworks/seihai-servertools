//! One-step lab start: the gserver fork's lab world plus a listserver,
//! started with one command and stopped together.
//!
//! A *lab world* is the folder of an unpacked `seihai-gserver-v2` release:
//!
//! ```text
//! <world>/
//!   gs2emu(.exe)                       the gserver
//!   servers/<name>/config/serveroptions.txt
//!   seihai-listserver(.exe)            optional, see below
//!   seihai-listserver.toml             optional listserver settings
//! ```
//!
//! [`plan`] works out what to start without starting anything (so it is
//! testable on every OS and `seihai-servertools lab plan` can print it);
//! the command line then starts the plan with [`crate::run`]. The listserver
//! starts first, so the gservers find it when they register.
//!
//! A lab world can also run `seihai-gserver` (the Rust gserver) on its own
//! world folder, next to the fork's server folders or instead of them:
//! [`LabOptions::rust_worlds`] (`lab start --rust DIR`). Its program,
//! `seihai-gserver`, is found like the listserver below. Its
//! `config/server.txt` is only read, never edited: when its `listserver`
//! setting does not point at the lab listserver, the plan says which line
//! to change.
//!
//! The listserver is `seihai-listserver`, looked for in this order: the path
//! the user gave, the world folder, the folder of the running
//! `seihai-servertools`, then `PATH`. If none is found the gservers start
//! alone: the lab world has `localhost = true`, so logins still work, but
//! clients get no server list.

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::listserver_toml::ListserverLab;
use crate::config::options_txt::OptionsTxt;
use crate::run::ProcessSpec;

/// The server folders started when none are named, if they exist.
pub const DEFAULT_SERVERS: &[&str] = &["default", "classic", "modern"];

/// File name of a program on this OS (`gs2emu` or `gs2emu.exe`).
pub fn exe_name(stem: &str) -> String {
    format!("{stem}{}", env::consts::EXE_SUFFIX)
}

/// Which listserver to start.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ListserverChoice {
    /// Look for `seihai-listserver` (see the module docs); none found is fine.
    #[default]
    Auto,
    /// This program; it must exist.
    Path(PathBuf),
    /// Do not start one.
    Off,
}

/// What the user asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LabOptions {
    /// The world folder.
    pub world: PathBuf,
    /// Server folders to start; empty means [`DEFAULT_SERVERS`] that exist.
    pub servers: Vec<String>,
    pub listserver: ListserverChoice,
    /// The gserver program; `None` means `<world>/gs2emu`.
    pub gserver: Option<PathBuf>,
    /// `seihai-gserver` world folders to start too (relative paths are
    /// taken from the world folder). Each needs `config/server.txt`.
    pub rust_worlds: Vec<PathBuf>,
    /// The `seihai-gserver` program; `None` looks for it like the
    /// listserver (world folder, next to this tool, `PATH`).
    pub rust_gserver: Option<PathBuf>,
}

/// Which gserver a plan line starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerKind {
    /// `gs2emu` from seihai-gserver-v2, one `servers/<name>` folder.
    Fork,
    /// `seihai-gserver`, one world folder with `config/server.txt`.
    Rust,
}

/// One gserver in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GserverPlan {
    pub name: String,
    pub kind: ServerKind,
    /// `serverinterface` and `serverport` from its options, for messages.
    pub interface: Option<String>,
    pub port: Option<String>,
    pub spec: ProcessSpec,
}

/// The listserver in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListserverPlan {
    pub settings: ListserverLab,
    pub spec: ProcessSpec,
}

/// Everything the one-step start will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub listserver: Option<ListserverPlan>,
    pub gservers: Vec<GserverPlan>,
    /// Things to tell the user before starting (not errors).
    pub notes: Vec<String>,
}

/// Why a plan could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabError {
    NoWorld(PathBuf),
    NoGserver(PathBuf),
    NoServerFolder(String),
    NoServersFound(PathBuf),
    NoListserver(PathBuf),
    NoRustWorld(PathBuf),
    NoRustGserver(Option<PathBuf>),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LabError::NoWorld(p) => write!(f, "no folder {}", p.display()),
            LabError::NoGserver(p) => write!(
                f,
                "no gserver at {}: unpack a seihai-gserver-v2 release there, or give --gserver",
                p.display()
            ),
            LabError::NoServerFolder(n) => write!(f, "no server folder servers/{n}"),
            LabError::NoServersFound(p) => write!(
                f,
                "no server folders ({}) under {}",
                DEFAULT_SERVERS.join(", "),
                p.join("servers").display()
            ),
            LabError::NoListserver(p) => write!(f, "no listserver at {}", p.display()),
            LabError::NoRustWorld(p) => write!(
                f,
                "no seihai-gserver world at {} (config/server.txt expected)",
                p.display()
            ),
            LabError::NoRustGserver(Some(p)) => write!(f, "no seihai-gserver at {}", p.display()),
            LabError::NoRustGserver(None) => write!(
                f,
                "no {} found (world folder, next to seihai-servertools, PATH): give --rust-gserver",
                exe_name("seihai-gserver")
            ),
        }
    }
}

impl std::error::Error for LabError {}

fn options_path(world: &Path, name: &str) -> PathBuf {
    world
        .join("servers")
        .join(name)
        .join("config")
        .join("serveroptions.txt")
}

fn find_listserver(world: &Path) -> Option<PathBuf> {
    find_program(world, "seihai-listserver")
}

/// A program in the world folder, next to the running tool, or on `PATH`.
fn find_program(world: &Path, stem: &str) -> Option<PathBuf> {
    let name = exe_name(stem);
    let mut dirs = vec![world.to_path_buf()];
    if let Some(dir) = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    if let Some(path) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&path));
    }
    dirs.into_iter()
        .map(|d| d.join(&name))
        .find(|p| p.is_file())
}

/// A server name must be a plain folder name: no separators, no `..`.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

/// Work out what to start. Nothing is started and nothing is written.
pub fn plan(opts: &LabOptions) -> Result<Plan, LabError> {
    let world = &opts.world;
    if !world.is_dir() {
        return Err(LabError::NoWorld(world.clone()));
    }
    let gserver = opts
        .gserver
        .clone()
        .unwrap_or_else(|| world.join(exe_name("gs2emu")));
    // With Rust worlds and no fork folders named, the fork is optional:
    // a world may hold only a seihai-gserver world folder.
    let rust_only = !opts.rust_worlds.is_empty() && opts.servers.is_empty();
    let mut notes = Vec::new();

    let names: Vec<String> = if opts.servers.is_empty() {
        let found: Vec<String> = DEFAULT_SERVERS
            .iter()
            .filter(|n| options_path(world, n).is_file())
            .map(|n| n.to_string())
            .collect();
        if found.is_empty() && !rust_only {
            return Err(LabError::NoServersFound(world.clone()));
        }
        if rust_only && !gserver.is_file() {
            if !found.is_empty() {
                notes.push(format!(
                    "no gserver at {}: starting the seihai-gserver worlds only, not {}",
                    gserver.display(),
                    found.join(", ")
                ));
            }
            Vec::new()
        } else {
            found
        }
    } else {
        opts.servers.clone()
    };
    if !names.is_empty() && !gserver.is_file() {
        return Err(LabError::NoGserver(gserver));
    }

    let mut gservers = Vec::new();
    let mut rust_settings: Vec<RustSettings> = Vec::new();
    for name in names {
        let path = options_path(world, &name);
        if !valid_name(&name) || !path.is_file() {
            return Err(LabError::NoServerFolder(name));
        }
        let options = OptionsTxt::load(&path).unwrap_or_default();
        let interface = options.get("serverinterface").map(str::to_string);
        if interface.as_deref() != Some("127.0.0.1") {
            notes.push(format!(
                "servers/{name} is not in lab mode (serverinterface = {}); it may be reachable from other machines",
                interface.as_deref().unwrap_or("not set")
            ));
        }
        gservers.push(GserverPlan {
            kind: ServerKind::Fork,
            port: options.get("serverport").map(str::to_string),
            interface,
            spec: ProcessSpec {
                label: name.clone(),
                program: gserver.clone(),
                args: vec!["--server".into(), name.clone()],
                cwd: world.clone(),
                console: world
                    .join("servers")
                    .join(&name)
                    .join("logs")
                    .join("console.txt"),
            },
            name,
        });
    }

    if !opts.rust_worlds.is_empty() {
        let program = match &opts.rust_gserver {
            Some(p) if p.is_file() => p.clone(),
            Some(p) => return Err(LabError::NoRustGserver(Some(p.clone()))),
            None => find_program(world, "seihai-gserver").ok_or(LabError::NoRustGserver(None))?,
        };
        for dir in &opts.rust_worlds {
            let dir = if dir.is_absolute() {
                dir.clone()
            } else {
                world.join(dir)
            };
            let config = dir.join("config").join("server.txt");
            if !config.is_file() {
                return Err(LabError::NoRustWorld(dir));
            }
            let settings = RustSettings::load(&config);
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "rust".into());
            if settings.bind != "127.0.0.1" {
                notes.push(format!(
                    "{name} (seihai-gserver) binds {}: it may be reachable from other machines",
                    settings.bind
                ));
            }
            gservers.push(GserverPlan {
                kind: ServerKind::Rust,
                interface: Some(settings.bind.clone()),
                port: Some(settings.port.clone()),
                spec: ProcessSpec {
                    label: name.clone(),
                    program: program.clone(),
                    args: vec!["--world".into(), dir.display().to_string()],
                    cwd: world.clone(),
                    console: world.join("logs").join(format!("{name}-console.txt")),
                },
                name,
            });
            rust_settings.push(settings);
        }
    }
    // Two servers on one address cannot both start.
    for (i, a) in gservers.iter().enumerate() {
        for b in &gservers[i + 1..] {
            if a.port.is_some() && a.port == b.port && a.interface == b.interface {
                notes.push(format!(
                    "{} and {} both listen on {}:{}: the second will not start; change one port",
                    a.name,
                    b.name,
                    a.interface.as_deref().unwrap_or("AUTO"),
                    a.port.as_deref().unwrap_or("?")
                ));
            }
        }
    }

    let program = match &opts.listserver {
        ListserverChoice::Off => None,
        ListserverChoice::Path(p) if p.is_file() => Some(p.clone()),
        ListserverChoice::Path(p) => return Err(LabError::NoListserver(p.clone())),
        ListserverChoice::Auto => {
            let found = find_listserver(world);
            if found.is_none() {
                notes.push(format!(
                    "no {} found (world folder, next to seihai-servertools, PATH): starting the gservers only; clients get no server list",
                    exe_name("seihai-listserver")
                ));
            }
            found
        }
    };
    let listserver = program.map(|program| {
        let config = world.join("seihai-listserver.toml");
        let (settings, args) = if config.is_file() {
            let text = fs::read_to_string(&config).unwrap_or_default();
            (
                ListserverLab::scan(&text),
                vec!["--config".into(), config.display().to_string()],
            )
        } else {
            (ListserverLab::default(), Vec::new())
        };
        if !settings.is_loopback() {
            notes.push(format!(
                "the listserver binds {}: it is reachable from other machines",
                settings.bind
            ));
        }
        if settings.silent_list_after_ms == 0 {
            notes.push(
                "the listserver's silent_list_after_ms is 0: real 1.41 rev 1 gets no server list (set it to 500 in seihai-listserver.toml, or run `setup host-listserver`)"
                    .into(),
            );
        }
        ListserverPlan {
            settings,
            spec: ProcessSpec {
                label: "listserver".into(),
                program,
                args,
                cwd: world.clone(),
                console: world.join("logs").join("listserver-console.txt"),
            },
        }
    });

    // The Rust gservers register only where their `listserver` points.
    if let Some(l) = &listserver {
        let want = format!("127.0.0.1:{}", l.settings.gserver_port);
        let rust = gservers.iter().filter(|g| g.kind == ServerKind::Rust);
        for (g, s) in rust.zip(&rust_settings) {
            if s.listserver != want {
                let before = if s.listserver.is_empty() {
                    "no listserver line".to_string()
                } else {
                    format!("`listserver = {}`", s.listserver)
                };
                notes.push(format!(
                    "{} (seihai-gserver) will not show in the lab list: its config/server.txt has {before}; change it to `listserver = {want}` (not edited for you)",
                    g.name
                ));
            }
        }
    }

    Ok(Plan {
        listserver,
        gservers,
        notes,
    })
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(l) = &self.listserver {
            writeln!(
                f,
                "listserver  {} {}  (gservers register on {}:{}; console {})",
                l.spec.program.display(),
                l.spec.args.join(" "),
                l.settings.bind,
                l.settings.gserver_port,
                l.spec.console.display()
            )?;
        }
        for g in &self.gservers {
            let kind = match g.kind {
                ServerKind::Fork => "gserver    ",
                ServerKind::Rust => "rust       ",
            };
            writeln!(
                f,
                "{kind} {} on {}:{}  (console {})",
                g.name,
                g.interface.as_deref().unwrap_or("AUTO"),
                g.port.as_deref().unwrap_or("?"),
                g.spec.console.display()
            )?;
        }
        for n in &self.notes {
            writeln!(f, "note: {n}")?;
        }
        Ok(())
    }
}

/// What the plan reads from a `seihai-gserver` world's `config/server.txt`:
/// `key = value`, `#` starts a comment, the last line for a key wins, and
/// a missing key takes seihai-gserver's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSettings {
    pub bind: String,
    pub port: String,
    /// `host:port`, or empty when it registers with no listserver.
    pub listserver: String,
}

impl RustSettings {
    pub fn parse(text: &str) -> Self {
        let mut out = RustSettings {
            bind: "127.0.0.1".into(),
            port: "14802".into(),
            listserver: String::new(),
        };
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("");
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim().to_string();
            match k.trim() {
                "bind" => out.bind = v,
                "port" => out.port = v,
                "listserver" => out.listserver = v,
                _ => {}
            }
        }
        out
    }

    pub fn load(path: &Path) -> Self {
        Self::parse(&fs::read_to_string(path).unwrap_or_default())
    }
}
