//! `seihai-suite-stress`: start the Seihai server suite on loopback and
//! drive bot clients against it. See `docs/lab/suite-stress.md`.

use seihai_core::protocol::ClientVersion;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use suite_stress::bot::{self, Plan, Shared};
use suite_stress::external::{self, ExtPlan, ExtShared};
use suite_stress::lab::{scan_log, Bins, Lab, LabConfig};
use suite_stress::report::{self, Tick};
use suite_stress::{rc, sample, DEFAULT_VERSIONS, LAB_PASSWORD};

const USAGE: &str = "\
seihai-suite-stress: run the Seihai server suite on 127.0.0.1 and drive bots at it

USAGE:
  seihai-suite-stress run    [options]   bots for --stay seconds, then a report
  seihai-suite-stress compat [options]   one bot per version profile (--versions all),
                                         5 s each: which profiles the suite takes today
  seihai-suite-stress versions           list every Seihai version profile
  seihai-suite-stress probe ADDR VERSION ACCOUNT STEPS
                                         one client against a loopback gserver you
                                         started: log in, run the STEPS file, print
                                         every server message (see docs/lab/probe/)

OPTIONS:
  --bin-dir DIR        where seihai-gserver, -listserver, -npcserver and -rc are
                       (repeatable; the first hit wins). Default: next to this binary.
  --out DIR            lab folder and results (default: ./suite-stress-out/<time>)
  --base-port N        gserver N+2, NPC link N+10, listserver N+100 / N+121 (default 24800)
  --bots N             bot clients (default 24)
  --versions LIST|all  comma-separated version labels, or all (default: a spread from 1.21 to 6.03r7)
  --stay SECS          how long each bot stays logged in (default 60)
  --ramp N             bots started per second (default 20)
  --list-fraction F    share of bots that fetch the server list first (default 1.0)
  --walk-ms MS         walk props every MS (default 100, as real 2.x)
  --chat-secs S        a chat line every S seconds (default 5)
  --probe-secs S       a file request (round-trip probe) every S seconds (default 2)
  --warp-secs S        take a link to the next ring level every S seconds (default 3; 0 = off)
  --levels N           levels in the warp ring written to the world (default 8; 0 = none,
                       for a gserver before stage 2.2)
  --gmap N             an N by N gmap whose cell (0, 0) is the start level (default 3;
                       0 = none). Builds the server puts on it cross cells every
                       --warp-secs instead of taking links
  --rc-every S         run seihai-rc servers and players list every S seconds (default 10; 0 = off)
  --max-players N      gserver max_players (default 2048)
  --list-max-connections N   listserver max_connections (default: its own default)
  --no-npcserver       leave the NPC-server out
  --no-listserver      leave the listserver out
  --verbose            start the gserver with --verbose
  --anticheat on|off   anticheat in the gserver's server.txt (default: its own default)

EXTERNAL CLIENTS (another client program next to the bots, one process each):
  --external PROG      the program, e.g. graal-delphi's Graal205. It is started with
                       --server --account --password-env --headless --seconds --walk
                       --walk-ms --seed [--link-every] and must print one JSON line
                       with \"kind\":\"summary\" on stdout when it ends
  --external-clients N how many (default 1 when --external is given)
  --external-arg ARG   one more argument for each of them (repeatable), e.g. --nodelay
  --external-link-secs S   --link-every for them (default: --warp-secs; 0 = none)
";

struct Opts {
    bin_dirs: Vec<PathBuf>,
    out: Option<PathBuf>,
    base_port: u16,
    bots: usize,
    versions: Vec<ClientVersion>,
    stay: Duration,
    ramp: f64,
    list_fraction: f64,
    walk: Duration,
    chat: Duration,
    probe: Duration,
    warp: Duration,
    levels: usize,
    gmap: usize,
    rc_every: u64,
    max_players: usize,
    list_max: Option<usize>,
    npc: bool,
    list: bool,
    verbose: bool,
    anticheat: Option<String>,
    external: Option<PathBuf>,
    ext_clients: Option<usize>,
    ext_args: Vec<String>,
    ext_link: Option<Duration>,
}

fn parse_versions(s: &str) -> Result<Vec<ClientVersion>, String> {
    if s == "all" {
        return Ok(ClientVersion::all_versions().to_vec());
    }
    s.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| ClientVersion::from_label(v).ok_or_else(|| format!("no version profile {v:?}")))
        .collect()
}

fn parse(args: &[String], compat: bool) -> Result<Opts, String> {
    let mut o = Opts {
        bin_dirs: Vec::new(),
        out: None,
        base_port: 24800,
        bots: 24,
        versions: parse_versions(&DEFAULT_VERSIONS.join(","))?,
        stay: Duration::from_secs(60),
        ramp: 20.0,
        list_fraction: 1.0,
        walk: Duration::from_millis(100),
        chat: Duration::from_secs(5),
        probe: Duration::from_secs(2),
        warp: Duration::from_secs(3),
        levels: 8,
        gmap: 3,
        rc_every: 10,
        max_players: 2048,
        list_max: None,
        npc: true,
        list: true,
        verbose: false,
        anticheat: None,
        external: None,
        ext_clients: None,
        ext_args: Vec::new(),
        ext_link: None,
    };
    if compat {
        o.versions = ClientVersion::all_versions().to_vec();
        o.stay = Duration::from_secs(5);
        o.rc_every = 0;
    }
    let mut bots_given = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        let num = |s: String| {
            s.parse::<f64>()
                .map_err(|_| format!("{a}: not a number: {s}"))
        };
        match a.as_str() {
            "--bin-dir" => o.bin_dirs.push(val()?.into()),
            "--out" => o.out = Some(val()?.into()),
            "--base-port" => o.base_port = num(val()?)? as u16,
            "--bots" => {
                o.bots = num(val()?)? as usize;
                bots_given = true;
            }
            "--versions" => o.versions = parse_versions(&val()?)?,
            "--stay" => o.stay = Duration::from_secs_f64(num(val()?)?),
            "--ramp" => o.ramp = num(val()?)?.max(0.1),
            "--list-fraction" => o.list_fraction = num(val()?)?.clamp(0.0, 1.0),
            "--walk-ms" => o.walk = Duration::from_millis(num(val()?)? as u64),
            "--chat-secs" => o.chat = Duration::from_secs_f64(num(val()?)?),
            "--probe-secs" => o.probe = Duration::from_secs_f64(num(val()?)?),
            "--warp-secs" => o.warp = Duration::from_secs_f64(num(val()?)?),
            "--gmap" => o.gmap = num(val()?)? as usize,
            "--levels" => o.levels = num(val()?)? as usize,
            "--rc-every" => o.rc_every = num(val()?)? as u64,
            "--max-players" => o.max_players = num(val()?)? as usize,
            "--list-max-connections" => o.list_max = Some(num(val()?)? as usize),
            "--no-npcserver" => o.npc = false,
            "--no-listserver" => o.list = false,
            "--verbose" => o.verbose = true,
            "--anticheat" => o.anticheat = Some(val()?),
            "--external" => o.external = Some(val()?.into()),
            "--external-clients" => o.ext_clients = Some(num(val()?)? as usize),
            "--external-arg" => o.ext_args.push(val()?),
            "--external-link-secs" => o.ext_link = Some(Duration::from_secs_f64(num(val()?)?)),
            _ => return Err(format!("unknown option {a}\n\n{USAGE}")),
        }
    }
    if compat && !bots_given {
        o.bots = o.versions.len();
    }
    if o.ext_clients.is_some_and(|n| n > 0) && o.external.is_none() {
        return Err("--external-clients needs --external PROG".into());
    }
    if o.versions.is_empty() {
        return Err("no versions".into());
    }
    if o.bin_dirs.is_empty() {
        if let Some(d) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
        {
            o.bin_dirs.push(d);
        }
    }
    Ok(o)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => go(&args[1..], false),
        Some("compat") => go(&args[1..], true),
        Some("probe") => probe(&args[1..]),
        Some("versions") => {
            for v in ClientVersion::all_versions() {
                println!("{}\t{}", v.label(), v.token());
            }
            Ok(())
        }
        _ => {
            eprint!("{USAGE}");
            Err(String::new())
        }
    };
    if let Err(e) = code {
        if !e.is_empty() {
            eprintln!("seihai-suite-stress: {e}");
        }
        std::process::exit(2);
    }
}

fn go(args: &[String], compat: bool) -> Result<(), String> {
    let mut o = parse(args, compat)?;
    let mut bins = Bins::find(&o.bin_dirs)?;
    if !o.npc {
        bins.npcserver = None;
    }
    if !o.list {
        bins.listserver = None;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let out = o
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("suite-stress-out/{stamp}")));
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut cfg = LabConfig::new(out.join("lab"), o.base_port);
    cfg.max_players = o.max_players;
    cfg.list_max_connections = o.list_max;
    cfg.verbose = o.verbose;
    cfg.anticheat = o.anticheat.clone();
    cfg.ring_levels = o.levels;
    cfg.gmap_size = o.gmap;
    if o.levels == 0 && o.gmap < 2 {
        o.warp = Duration::ZERO;
    }
    suite_stress::check_loopback(&cfg.gserver_addr())?;

    eprintln!("starting the suite in {}", cfg.dir.display());
    let mut lab = Lab::up(cfg.clone(), &bins)?;
    for (n, pid) in lab.pids() {
        eprintln!("  {n}: pid {pid}");
    }
    if bins.npcserver.is_some() && !lab.pids().iter().any(|(n, _)| *n == "npcserver") {
        eprintln!("  npcserver: not seen as a child of the gserver (not sampled)");
    }
    // The gserver registers with the listserver shortly after it starts.
    if bins.listserver.is_some() {
        std::thread::sleep(Duration::from_secs(2));
    }

    let shared = Arc::new(Shared::default());
    {
        let s = shared.clone();
        let _ = ctrlc::set_handler(move || s.stop.store(true, Ordering::Relaxed));
    }

    let run_tag = format!("s{}", stamp % 100_000);
    let mut handles = Vec::new();
    let start = Instant::now();
    let rc_probes = Arc::new(Mutex::new(Vec::new()));
    let rc_thread = match (&bins.rc, o.rc_every) {
        (Some(rcbin), n) if n > 0 => {
            let (rcbin, s, probes) = (rcbin.clone(), shared.clone(), rc_probes.clone());
            let (list, gs) = (cfg.list_client_addr(), cfg.gserver_addr());
            let has_list = bins.listserver.is_some();
            Some(std::thread::spawn(move || {
                let mut next = Instant::now() + Duration::from_secs(3);
                while !s.stop.load(Ordering::Relaxed) {
                    if Instant::now() >= next {
                        if has_list {
                            let p = rc::servers(&rcbin, &list, "stressrc");
                            probes.lock().unwrap().push(p);
                        }
                        let p = rc::players(&rcbin, &gs, "stressrc");
                        probes.lock().unwrap().push(p);
                        next = Instant::now() + Duration::from_secs(n);
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            }))
        }
        _ => None,
    };

    let mut ticks = Vec::new();
    let mut exited = Vec::new();
    let mut next_tick = Instant::now();
    let mut started = 0usize;
    let n_ext = if o.external.is_some() {
        o.ext_clients.unwrap_or(1)
    } else {
        0
    };
    let ext_shared = Arc::new(ExtShared::default());
    let mut ext_handles = Vec::new();
    let ext_dir = out.join("external");
    if n_ext > 0 {
        std::fs::create_dir_all(&ext_dir).map_err(|e| format!("{}: {e}", ext_dir.display()))?;
    }
    let gap = Duration::from_secs_f64(1.0 / o.ramp);
    let mut next_bot = Instant::now();
    loop {
        let now = Instant::now();
        while started + ext_handles.len() < o.bots + n_ext
            && now >= next_bot
            && !shared.stop.load(Ordering::Relaxed)
        {
            // Spread the external clients evenly among the bots.
            let e = ext_handles.len();
            if e < n_ext && (started >= o.bots || e * o.bots <= started * n_ext) {
                let account = format!("{run_tag}x{e}");
                let plan = ExtPlan {
                    index: e,
                    program: o.external.clone().expect("n_ext > 0"),
                    extra: o.ext_args.clone(),
                    log: ext_dir.join(format!("{account}.log")),
                    account,
                    password: LAB_PASSWORD.into(),
                    gserver: cfg.gserver_addr(),
                    stay: o.stay,
                    walk_every: o.walk,
                    link_every: o.ext_link.unwrap_or(o.warp),
                };
                let (s, es) = (shared.clone(), ext_shared.clone());
                let h = std::thread::Builder::new()
                    .name(format!("ext{e}"))
                    .spawn(move || external::run(plan, &es, &s.stop))
                    .map_err(|err| format!("external thread {e}: {err}"))?;
                ext_handles.push(h);
                next_bot += gap;
                continue;
            }
            let i = started;
            let v = o.versions[i % o.versions.len()];
            // Spread the list fetches evenly over the bots.
            let fetches =
                ((i + 1) as f64 * o.list_fraction).floor() > (i as f64 * o.list_fraction).floor();
            let list = (bins.listserver.is_some() && fetches).then(|| cfg.list_client_addr());
            let plan = Plan {
                index: i,
                version: v,
                account: format!("{run_tag}b{i}"),
                password: LAB_PASSWORD.into(),
                gserver: cfg.gserver_addr(),
                listserver: list,
                server_name: cfg.server_name.clone(),
                stay: o.stay,
                walk_every: o.walk,
                chat_every: o.chat,
                probe_every: o.probe,
                warp_every: o.warp,
                warp_ring: if o.warp.is_zero() {
                    Vec::new()
                } else {
                    suite_stress::lab::ring_names(o.levels)
                },
                gmap: (o.gmap >= 2).then(|| {
                    (
                        suite_stress::lab::gmap_text(o.gmap),
                        suite_stress::lab::gmap_loop(o.gmap),
                    )
                }),
            };
            let s = shared.clone();
            let h = std::thread::Builder::new()
                .name(format!("bot{i}"))
                .stack_size(512 * 1024)
                .spawn(move || bot::run(plan, &s))
                .map_err(|e| format!("bot thread {i}: {e}"))?;
            handles.push(h);
            started += 1;
            next_bot += gap;
        }
        if now >= next_tick {
            let procs = lab
                .pids()
                .into_iter()
                .filter_map(|(n, pid)| sample::read(pid).map(|s| (n, s)))
                .collect();
            ticks.push(Tick {
                t: start.elapsed().as_secs_f64(),
                online: shared.online.load(Ordering::Relaxed),
                logins_ok: shared.logins_ok.load(Ordering::Relaxed),
                logins_failed: shared.logins_failed.load(Ordering::Relaxed),
                dropped: shared.dropped.load(Ordering::Relaxed),
                sent_msgs: shared.sent_msgs.load(Ordering::Relaxed),
                frames_in: shared.frames_in.load(Ordering::Relaxed),
                procs,
            });
            for e in lab.exited() {
                if !exited.iter().any(|(n, _): &(&str, String)| *n == e.0) {
                    eprintln!("!! {} exited: {}", e.0, e.1);
                    exited.push(e);
                }
            }
            let t = ticks.last().unwrap();
            eprintln!(
                "[{:6.1}s] started {started}/{} online {} ok {} failed {} dropped {}{}",
                t.t,
                o.bots,
                t.online,
                t.logins_ok,
                t.logins_failed,
                t.dropped,
                if n_ext > 0 {
                    format!(
                        " external {}/{n_ext} running {}",
                        ext_handles.len(),
                        ext_shared.running.load(Ordering::Relaxed)
                    )
                } else {
                    String::new()
                }
            );
            next_tick += Duration::from_secs(1);
        }
        let all_started = (started >= o.bots && ext_handles.len() >= n_ext)
            || shared.stop.load(Ordering::Relaxed);
        if all_started
            && handles.iter().all(|h| h.is_finished())
            && ext_handles.iter().all(|h| h.is_finished())
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    shared.stop.store(true, Ordering::Relaxed);
    let outcomes: Vec<_> = handles.into_iter().filter_map(|h| h.join().ok()).collect();
    let ext_outcomes: Vec<_> = ext_handles
        .into_iter()
        .filter_map(|h| h.join().ok())
        .collect();
    if let Some(t) = rc_thread {
        let _ = t.join();
    }
    // A last sample after every bot left: memory that stays is a leak lead.
    std::thread::sleep(Duration::from_secs(2));
    let procs = lab
        .pids()
        .into_iter()
        .filter_map(|(n, pid)| sample::read(pid).map(|s| (n, s)))
        .collect();
    ticks.push(Tick {
        t: start.elapsed().as_secs_f64(),
        online: 0,
        logins_ok: shared.logins_ok.load(Ordering::Relaxed),
        logins_failed: shared.logins_failed.load(Ordering::Relaxed),
        dropped: shared.dropped.load(Ordering::Relaxed),
        sent_msgs: shared.sent_msgs.load(Ordering::Relaxed),
        frames_in: shared.frames_in.load(Ordering::Relaxed),
        procs,
    });
    lab.down();

    let logs: Vec<_> = ["listserver.log", "gserver.log"]
        .iter()
        .filter(|n| cfg.logs().join(n).exists())
        .map(|n| {
            let (c, ex) = scan_log(&cfg.logs().join(n), 8);
            (n.to_string(), c, ex)
        })
        .collect();
    let rc_probes = rc_probes.lock().unwrap().clone();
    let labels: Vec<&str> = o.versions.iter().map(|v| v.label()).collect();
    let settings = vec![
        ("bots".into(), o.bots.to_string()),
        ("versions".into(), labels.join(", ")),
        ("stay".into(), format!("{:?}", o.stay)),
        ("ramp".into(), format!("{} bots/s", o.ramp)),
        (
            "walk / chat / probe / warp".into(),
            format!("{:?} / {:?} / {:?} / {:?}", o.walk, o.chat, o.probe, o.warp),
        ),
        ("ring levels".into(), o.levels.to_string()),
        (
            "gmap".into(),
            if o.gmap >= 2 {
                format!("{0}x{0}", o.gmap)
            } else {
                "none".into()
            },
        ),
        ("list fraction".into(), o.list_fraction.to_string()),
        (
            "suite".into(),
            lab.pids()
                .iter()
                .map(|p| p.0)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ("max_players".into(), o.max_players.to_string()),
    ];
    let mut settings = settings;
    if let Some(p) = &o.external {
        settings.push((
            "external clients".into(),
            format!(
                "{n_ext} x `{}`{}",
                p.display(),
                if o.ext_args.is_empty() {
                    String::new()
                } else {
                    format!(" {}", o.ext_args.join(" "))
                }
            ),
        ));
    }
    let md = report::markdown(&report::Run {
        title: if compat {
            "Suite compatibility pass".into()
        } else {
            "Suite stress run".into()
        },
        settings,
        outcomes: &outcomes,
        ticks: &ticks,
        rc: &rc_probes,
        exited: &exited,
        logs: &logs,
        external: &external::markdown(&ext_outcomes, o.external.as_deref()),
    });
    let w = |name: &str, body: String| {
        std::fs::write(out.join(name), body).map_err(|e| format!("{name}: {e}"))
    };
    w("report.md", md.clone())?;
    w("timeline.tsv", report::timeline_tsv(&ticks))?;
    w("bots.tsv", report::bots_tsv(&outcomes))?;
    if !ext_outcomes.is_empty() {
        w("external.tsv", external::tsv(&ext_outcomes))?;
    }
    println!("{md}");
    eprintln!("results in {}", out.display());
    Ok(())
}

/// `probe ADDR VERSION ACCOUNT STEPS`. Password `labpw` (a lab gserver with
/// `localhost = true` takes any). `STEP_MS` sets the wait after each step
/// (default 300 ms).
fn probe(args: &[String]) -> Result<(), String> {
    use seihai_core::net::{describe_message, login_gserver, random_key};
    use suite_stress::probe::{parse_steps, Step};
    let [addr, ver, account, steps] = args else {
        return Err("usage: seihai-suite-stress probe ADDR VERSION ACCOUNT STEPS".into());
    };
    suite_stress::check_loopback(addr)?;
    let version =
        ClientVersion::from_label(ver).ok_or_else(|| format!("no version profile {ver:?}"))?;
    let text = std::fs::read_to_string(steps).map_err(|e| format!("{steps}: {e}"))?;
    let steps = parse_steps(&text).map_err(|e| format!("{steps}: {e}"))?;
    let step_wait = Duration::from_millis(
        std::env::var("STEP_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300),
    );
    let mut s = login_gserver(addr, version, random_key(), account, LAB_PASSWORD, false)
        .map_err(|e| format!("login: {e}"))?;
    let rx = s.conn.spawn_reader().map_err(|e| e.to_string())?;
    let t0 = Instant::now();
    let drain = |until: Instant| -> Result<(), String> {
        while Instant::now() < until {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(body)) => {
                    for m in body.split_inclusive(|b| *b == b'\n') {
                        println!(
                            "[{:6.2}] S>C {}",
                            t0.elapsed().as_secs_f32(),
                            describe_message(m)
                        );
                    }
                }
                Ok(Err(e)) => {
                    println!("[{:6.2}] closed: {e}", t0.elapsed().as_secs_f32());
                    return Err("the server closed the connection".into());
                }
                Err(_) => {}
            }
        }
        Ok(())
    };
    drain(Instant::now() + Duration::from_secs(2))?;
    let _ = s.enter_level.at_login(&mut s.conn);
    for step in &steps {
        println!("[{:6.2}] STEP {step:?}", t0.elapsed().as_secs_f32());
        let sent = match step {
            Step::Wait(secs) => {
                drain(Instant::now() + Duration::from_secs_f32(*secs))?;
                Ok(())
            }
            Step::Walk { x, y, dir } => s.conn.send_position(*x, *y, *dir),
            Step::Chat(text) => s.conn.send_chat(text),
            Step::Trigger {
                x,
                y,
                action,
                params,
            } => s.conn.send_trigger_action(*x, *y, action, params),
            Step::Warp { level, x, y } => s.conn.send_levelwarp(*x, *y, level),
            Step::Enter => s.conn.send_enter_level(version, &[]).map(|_| ()),
        };
        if let Err(e) = sent {
            println!("send error {e}");
        }
        drain(Instant::now() + step_wait)?;
    }
    drain(Instant::now() + Duration::from_secs(1))
}
