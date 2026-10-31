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
pub const DEFAULT_SERVERS: &[&str] = &["default", "classic"];

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
}

/// One gserver in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GserverPlan {
    pub name: String,
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
    let name = exe_name("seihai-listserver");
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
    if !gserver.is_file() {
        return Err(LabError::NoGserver(gserver));
    }

    let names: Vec<String> = if opts.servers.is_empty() {
        let found: Vec<String> = DEFAULT_SERVERS
            .iter()
            .filter(|n| options_path(world, n).is_file())
            .map(|n| n.to_string())
            .collect();
        if found.is_empty() {
            return Err(LabError::NoServersFound(world.clone()));
        }
        found
    } else {
        opts.servers.clone()
    };

    let mut notes = Vec::new();
    let mut gservers = Vec::new();
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
            writeln!(
                f,
                "gserver     {} on {}:{}  (console {})",
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
