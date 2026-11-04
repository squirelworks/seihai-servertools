//! A throwaway lab: the config files for each server and the running
//! processes, all on `127.0.0.1`.

use std::fs;
use std::io::{self, BufRead};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Where the server binaries are. `None` leaves that part of the suite out.
#[derive(Debug, Clone, Default)]
pub struct Bins {
    pub gserver: PathBuf,
    pub listserver: Option<PathBuf>,
    pub npcserver: Option<PathBuf>,
    pub rc: Option<PathBuf>,
}

impl Bins {
    /// Look for each binary in `dirs`, first hit wins. The gserver is
    /// required; the rest are optional.
    pub fn find(dirs: &[PathBuf]) -> Result<Self, String> {
        let exe = |name: &str| -> Option<PathBuf> {
            let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
            dirs.iter().map(|d| d.join(&file)).find(|p| p.is_file())
        };
        Ok(Bins {
            gserver: exe("seihai-gserver")
                .ok_or_else(|| format!("seihai-gserver not found in {dirs:?}"))?,
            listserver: exe("seihai-listserver"),
            npcserver: exe("seihai-npcserver"),
            rc: exe("seihai-rc"),
        })
    }
}

/// Ports and limits. Every port is on 127.0.0.1.
#[derive(Debug, Clone)]
pub struct LabConfig {
    pub dir: PathBuf,
    pub gserver_port: u16,
    pub npc_port: u16,
    /// The listserver's gserver port.
    pub list_gserver_port: u16,
    /// The listserver's client port.
    pub list_client_port: u16,
    pub max_players: usize,
    /// `max_connections` for the listserver; `None` keeps its default.
    pub list_max_connections: Option<usize>,
    pub server_name: String,
    /// `--verbose` on the gserver.
    pub verbose: bool,
    /// Levels in the warp ring written to `levels/` (see [`ring_names`]).
    /// 0 writes no levels at all, as for a stage-1 gserver.
    pub ring_levels: usize,
    /// Side of the square gmap written to `levels/stress.gmap`, its cell
    /// (0, 0) being the start level (see [`gmap_text`]). 0 or 1: no gmap.
    pub gmap_size: usize,
}

impl LabConfig {
    /// Ports from `base`: gserver `base+2`, NPC link `base+10`, listserver
    /// gserver port `base+100`, client port `base+121`. The default base
    /// 24800 keeps clear of the usual 14xxx lab ports.
    pub fn new(dir: PathBuf, base: u16) -> Self {
        LabConfig {
            dir,
            gserver_port: base + 2,
            npc_port: base + 10,
            list_gserver_port: base + 100,
            list_client_port: base + 121,
            max_players: 2048,
            list_max_connections: None,
            server_name: "Seihai Stress".into(),
            verbose: false,
            ring_levels: 8,
            gmap_size: 3,
        }
    }

    pub fn gserver_addr(&self) -> String {
        format!("127.0.0.1:{}", self.gserver_port)
    }

    pub fn list_client_addr(&self) -> String {
        format!("127.0.0.1:{}", self.list_client_port)
    }

    pub fn world(&self) -> PathBuf {
        self.dir.join("world")
    }

    pub fn logs(&self) -> PathBuf {
        self.dir.join("logs")
    }
}

/// The start level every bot logs in to.
pub const START_LEVEL: &str = "onlinestartlocal.nw";

/// The warp ring's level names: the start level, then `stress_w1.nw`…
pub fn ring_names(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            if i == 0 {
                START_LEVEL.to_string()
            } else {
                format!("stress_w{i}.nw")
            }
        })
        .collect()
}

/// A full 64x64 `.nw` level for ring slot `i` of `n`: every row of the
/// board (so a warp moves a real board, about 8 KB), a link on the top
/// edge to the next level, and a sign.
pub fn ring_level(i: usize, n: usize) -> String {
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let names = ring_names(n);
    let mut s = String::from("GLEVNW01\n");
    for y in 0..64 {
        s.push_str(&format!("BOARD 0 {y} 64 0 "));
        for x in 0..64usize {
            // Two base64 digits a tile; vary them so boards differ.
            s.push(B64[(i + y / 8) % 64] as char);
            s.push(B64[(x + y) % 64] as char);
        }
        s.push('\n');
    }
    s.push_str(&format!("LINK {} 30 0 4 1 30 60\n", names[(i + 1) % n]));
    s.push_str(&format!("SIGN 20 21\nRing level {i} of {n}.\nSIGNEND\n"));
    s
}

/// The level name of gmap cell `(x, y)`: the start level at (0, 0).
pub fn cell_name(x: usize, y: usize) -> String {
    if (x, y) == (0, 0) {
        START_LEVEL.to_string()
    } else {
        format!("stress_g{x}_{y}.nw")
    }
}

/// The `.gmap` text for an `n` by `n` grid of [`cell_name`]s.
pub fn gmap_text(n: usize) -> String {
    let mut s = format!("GRMAP001\nWIDTH {n}\nHEIGHT {n}\nLEVELNAMES\n");
    for y in 0..n {
        for x in 0..n {
            s.push_str(&format!("\"{}\",", cell_name(x, y)));
        }
        s.push('\n');
    }
    s.push_str("LEVELNAMESEND\n");
    s
}

/// A gmap cell's level: a full board, no links (the seams are the way out).
pub fn cell_level(x: usize, y: usize) -> String {
    let full = ring_level(x * 7 + y * 13 + 3, 2);
    let mut s: String = full
        .lines()
        .filter(|l| l.starts_with("GLEVNW01") || l.starts_with("BOARD"))
        .map(|l| format!("{l}\n"))
        .collect();
    s.push_str(&format!("SIGN 20 21\nGmap cell {x},{y}.\nSIGNEND\n"));
    s
}

/// The cells around the edge of an `n` by `n` grid, in walking order: each
/// is next to the one before, and the last is next to the first.
pub fn gmap_loop(n: usize) -> Vec<(i32, i32)> {
    if n < 2 {
        return Vec::new();
    }
    let n = n as i32;
    let mut v = Vec::new();
    v.extend((0..n).map(|x| (x, 0)));
    v.extend((1..n).map(|y| (n - 1, y)));
    v.extend((0..n - 1).rev().map(|x| (x, n - 1)));
    v.extend((1..n - 1).rev().map(|y| (0, y)));
    v
}

/// The gserver's `config/server.txt` for this lab.
pub fn gserver_config(cfg: &LabConfig, bins: &Bins) -> String {
    let mut s = format!(
        "# Written by seihai-suite-stress. Loopback lab only.\n\
         name = {}\nbind = 127.0.0.1\nport = {}\n\
         create_accounts = true\nheartbeat = 5\nmax_players = {}\n",
        cfg.server_name, cfg.gserver_port, cfg.max_players
    );
    match &bins.npcserver {
        Some(p) => s.push_str(&format!(
            "npcserver = auto\nnpcserver_bind = 127.0.0.1\nnpcserver_port = {}\nnpcserver_path = {}\n",
            cfg.npc_port,
            p.display()
        )),
        None => s.push_str("npcserver = off\n"),
    }
    if bins.listserver.is_some() {
        s.push_str(&format!(
            "listserver = 127.0.0.1:{}\ndescription = suite stress lab\n",
            cfg.list_gserver_port
        ));
    }
    s
}

/// The listserver's TOML for this lab: open mode (every account gets in),
/// one client port, the encrypted gserver link on.
pub fn listserver_config(cfg: &LabConfig) -> String {
    let mut s = format!(
        "# Written by seihai-suite-stress. Loopback lab only.\n\
         bind = \"127.0.0.1\"\ngserver_port = {}\nclient_ports = [{}]\n\
         list_name = \"Stress listserver\"\naccounts = \"\"\nstatus_log_secs = 10\n\
         login_fail_limit = 0\n",
        cfg.list_gserver_port, cfg.list_client_port
    );
    if let Some(n) = cfg.list_max_connections {
        s.push_str(&format!("max_connections = {n}\n"));
    }
    s
}

/// One server process and the file its output goes to.
pub struct Proc {
    pub name: &'static str,
    pub child: Option<Child>,
    pub pid: u32,
    pub log: PathBuf,
}

/// The running suite.
pub struct Lab {
    pub cfg: LabConfig,
    pub procs: Vec<Proc>,
}

fn wait_port(port: u16, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    while Instant::now() < end {
        if TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_millis(200),
        )
        .is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn spawn(name: &'static str, cmd: &mut Command, log: PathBuf) -> io::Result<Proc> {
    let out = fs::File::create(&log)?;
    let err = out.try_clone()?;
    let child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()?;
    Ok(Proc {
        name,
        pid: child.id(),
        child: Some(child),
        log,
    })
}

impl Lab {
    /// Write the lab folder and start the suite: the listserver first (so
    /// the gserver's link finds it), then the gserver, which starts the
    /// NPC-server itself. Returns once each listening port answers.
    pub fn up(cfg: LabConfig, bins: &Bins) -> Result<Lab, String> {
        let world = cfg.world();
        fs::create_dir_all(world.join("config")).map_err(|e| e.to_string())?;
        fs::create_dir_all(cfg.logs()).map_err(|e| e.to_string())?;
        let mut lab = Lab {
            cfg: cfg.clone(),
            procs: Vec::new(),
        };
        if let Some(ls) = &bins.listserver {
            let toml = cfg.dir.join("listserver.toml");
            fs::write(&toml, listserver_config(&cfg)).map_err(|e| e.to_string())?;
            let p = spawn(
                "listserver",
                Command::new(ls).arg("--config").arg(&toml),
                cfg.logs().join("listserver.log"),
            )
            .map_err(|e| format!("start listserver: {e}"))?;
            lab.procs.push(p);
            if !wait_port(cfg.list_client_port, Duration::from_secs(15)) {
                return Err("listserver did not open its client port".into());
            }
        }
        fs::write(world.join("config/server.txt"), gserver_config(&cfg, bins))
            .map_err(|e| e.to_string())?;
        fs::write(world.join("config/allowedversions.txt"), "*\n").map_err(|e| e.to_string())?;
        if cfg.ring_levels > 0 {
            fs::create_dir_all(world.join("levels")).map_err(|e| e.to_string())?;
            for (i, name) in ring_names(cfg.ring_levels).iter().enumerate() {
                fs::write(
                    world.join("levels").join(name),
                    ring_level(i, cfg.ring_levels),
                )
                .map_err(|e| e.to_string())?;
            }
        }
        if cfg.gmap_size >= 2 {
            fs::create_dir_all(world.join("levels")).map_err(|e| e.to_string())?;
            fs::write(world.join("levels/stress.gmap"), gmap_text(cfg.gmap_size))
                .map_err(|e| e.to_string())?;
            for y in 0..cfg.gmap_size {
                for x in 0..cfg.gmap_size {
                    if (x, y) != (0, 0) {
                        fs::write(world.join("levels").join(cell_name(x, y)), cell_level(x, y))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            if cfg.ring_levels == 0 {
                fs::write(world.join("levels").join(START_LEVEL), cell_level(0, 0))
                    .map_err(|e| e.to_string())?;
            }
        }
        let mut cmd = Command::new(&bins.gserver);
        cmd.arg("--world").arg(&world);
        if cfg.verbose {
            cmd.arg("--verbose");
        }
        let p = spawn("gserver", &mut cmd, cfg.logs().join("gserver.log"))
            .map_err(|e| format!("start gserver: {e}"))?;
        let gpid = p.pid;
        lab.procs.push(p);
        if !wait_port(cfg.gserver_port, Duration::from_secs(15)) {
            return Err("gserver did not open its port".into());
        }
        if bins.npcserver.is_some() {
            // Started by the gserver; find it for the samples.
            let end = Instant::now() + Duration::from_secs(10);
            while Instant::now() < end {
                if let Some(pid) = crate::sample::find_child(gpid, "seihai-npcserver") {
                    lab.procs.push(Proc {
                        name: "npcserver",
                        child: None,
                        pid,
                        log: cfg.logs().join("gserver.log"),
                    });
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        Ok(lab)
    }

    /// Pids by name, for the sampler.
    pub fn pids(&self) -> Vec<(&'static str, u32)> {
        self.procs.iter().map(|p| (p.name, p.pid)).collect()
    }

    /// Which processes have exited, with their status.
    pub fn exited(&mut self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        for p in &mut self.procs {
            if let Some(c) = &mut p.child {
                if let Ok(Some(st)) = c.try_wait() {
                    out.push((p.name, st.to_string()));
                }
            } else if crate::sample::read(p.pid).is_none() && cfg!(target_os = "linux") {
                out.push((p.name, "gone".into()));
            }
        }
        out
    }

    /// Stop everything, the gserver's NPC-server included.
    pub fn down(&mut self) {
        for p in self.procs.iter_mut().rev() {
            if let Some(c) = &mut p.child {
                let _ = c.kill();
                let _ = c.wait();
            } else if cfg!(unix) {
                // Started by the gserver (the NPC-server): not our child.
                let _ = Command::new("kill").arg(p.pid.to_string()).status();
            }
        }
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        self.down();
    }
}

/// Lines of a server log that look like trouble: a panic, an error, a
/// refused or dropped link. Returns (count, first few lines).
pub fn scan_log(path: &Path, max_examples: usize) -> (usize, Vec<String>) {
    let Ok(f) = fs::File::open(path) else {
        return (0, Vec::new());
    };
    let mut n = 0;
    let mut ex = Vec::new();
    for line in io::BufReader::new(f).lines().map_while(Result::ok) {
        let l = line.to_ascii_lowercase();
        if l.contains("panic")
            || l.contains("error")
            || l.contains("refused")
            || l.contains("too many")
            || l.contains("full")
        {
            n += 1;
            if ex.len() < max_examples {
                ex.push(line);
            }
        }
    }
    (n, ex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_levels_parse_and_link_round() {
        let names = ring_names(3);
        assert_eq!(
            names,
            ["onlinestartlocal.nw", "stress_w1.nw", "stress_w2.nw"]
        );
        for i in 0..3 {
            let text = ring_level(i, 3);
            let lv = seihai_core::assets::Level::parse(&text).unwrap();
            assert_eq!(lv.tile_ids().len(), 64 * 64);
            assert!(text.contains(&format!("LINK {} ", names[(i + 1) % 3])));
        }
        assert_ne!(ring_level(0, 3), ring_level(1, 3));
    }

    #[test]
    fn gmap_grid_and_loop() {
        let g = seihai_core::gmap::Gmap::parse(&gmap_text(3)).unwrap();
        let _ = g;
        assert!(
            gmap_text(3).contains("\"onlinestartlocal.nw\",\"stress_g1_0.nw\",\"stress_g2_0.nw\",")
        );
        let l = gmap_loop(3);
        assert_eq!(l.len(), 8);
        assert_eq!(l[0], (0, 0));
        for i in 0..l.len() {
            let (a, b) = (l[i], l[(i + 1) % l.len()]);
            assert_eq!((a.0 - b.0).abs() + (a.1 - b.1).abs(), 1, "{a:?} {b:?}");
        }
        assert!(gmap_loop(1).is_empty());
        let lv = seihai_core::assets::Level::parse(&cell_level(1, 2)).unwrap();
        assert_eq!(lv.tile_ids().len(), 64 * 64);
    }

    #[test]
    fn configs_stay_on_loopback() {
        let cfg = LabConfig::new(PathBuf::from("/x"), 24800);
        let bins = Bins {
            gserver: "g".into(),
            listserver: Some("l".into()),
            npcserver: Some("n".into()),
            rc: None,
        };
        let g = gserver_config(&cfg, &bins);
        assert!(g.contains("bind = 127.0.0.1"));
        assert!(g.contains("port = 24802"));
        assert!(g.contains("npcserver = auto"));
        assert!(g.contains("listserver = 127.0.0.1:24900"));
        let l = listserver_config(&cfg);
        assert!(l.contains("bind = \"127.0.0.1\""));
        assert!(l.contains("client_ports = [24921]"));
        assert!(!l.contains("max_connections"));
        let none = Bins {
            gserver: "g".into(),
            ..Default::default()
        };
        let g = gserver_config(&cfg, &none);
        assert!(g.contains("npcserver = off"));
        assert!(!g.contains("listserver ="));
    }
}
