//! Other client programs mixed in with the bots: one child process per
//! client, each told where the gserver is and how long to stay, each
//! printing one JSON line with `"kind":"summary"` on stdout when it ends.
//!
//! The flags and the summary are the ones graal-delphi's headless run mode
//! takes and prints (`--server --account --password-env --headless
//! --seconds --walk --walk-ms --seed --link-every`); a program that takes
//! other flags can be wrapped in a script. Everything it prints on stderr
//! goes to `<out>/external/<account>.log`.

use crate::stats::summary;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The environment variable the lab password is handed over in.
pub const PASSWORD_ENV: &str = "SEIHAI_LAB_PASSWORD";

/// How to start one external client.
#[derive(Debug, Clone)]
pub struct ExtPlan {
    pub index: usize,
    pub program: PathBuf,
    /// Extra arguments, after the ones the rig passes.
    pub extra: Vec<String>,
    pub account: String,
    pub password: String,
    pub gserver: String,
    pub stay: Duration,
    pub walk_every: Duration,
    /// 0 = no links.
    pub link_every: Duration,
    pub log: PathBuf,
}

impl ExtPlan {
    /// The arguments the program is started with.
    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = vec![
            "--server".into(),
            self.gserver.clone(),
            "--account".into(),
            self.account.clone(),
            "--password-env".into(),
            PASSWORD_ENV.into(),
            "--headless".into(),
            "--seconds".into(),
            format!("{}", self.stay.as_secs_f64().ceil() as u64),
            "--walk".into(),
            if self.walk_every.is_zero() {
                "idle".into()
            } else {
                "random".into()
            },
            "--walk-ms".into(),
            self.walk_every.as_millis().max(1).to_string(),
            "--seed".into(),
            (self.index + 1).to_string(),
        ];
        if !self.link_every.is_zero() {
            a.push("--link-every".into());
            a.push(format!("{}", self.link_every.as_secs_f64().ceil() as u64));
        }
        a.extend(self.extra.iter().cloned());
        a
    }
}

/// What one external client reported, or what the rig saw when it did not.
#[derive(Debug, Clone, Default)]
pub struct ExtOutcome {
    pub account: String,
    /// The client's own version label, from its summary.
    pub version: String,
    pub exit: Option<i32>,
    /// clean / nologin / dropped / error, or the rig's own note
    /// ("no summary", "killed", "did not start").
    pub end: String,
    pub error: String,
    pub login_ms: Option<f64>,
    pub warps: u64,
    pub warps_done: u64,
    pub warp_ms_p50: Option<f64>,
    pub warp_ms_max: Option<f64>,
    pub tick_ms_p99: Option<f64>,
    pub tick_ms_max: Option<f64>,
    pub frames_in: u64,
    pub msgs_out: u64,
    pub rss_peak_kib: Option<u64>,
    pub nodelay: Option<bool>,
}

/// Counters the main loop reads once a second.
#[derive(Debug, Default)]
pub struct ExtShared {
    pub running: AtomicUsize,
}

/// Run one external client to its end. It is killed if it is still
/// running 30 s after its stay, or when `stop` turns true for more than
/// 10 s (it gets the stay itself, so it normally ends on its own).
pub fn run(p: ExtPlan, shared: &ExtShared, stop: &std::sync::atomic::AtomicBool) -> ExtOutcome {
    let mut out = ExtOutcome {
        account: p.account.clone(),
        ..Default::default()
    };
    let log = std::fs::File::create(&p.log);
    let mut cmd = Command::new(&p.program);
    cmd.args(p.args())
        .env(PASSWORD_ENV, &p.password)
        .stdin(Stdio::null())
        .stdout(Stdio::piped());
    match log {
        Ok(f) => cmd.stderr(f),
        Err(_) => cmd.stderr(Stdio::null()),
    };
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            out.end = "did not start".into();
            out.error = e.to_string();
            return out;
        }
    };
    shared.running.fetch_add(1, Ordering::Relaxed);
    let stdout = child.stdout.take().expect("piped");
    let last = Arc::new(std::sync::Mutex::new(None::<String>));
    let reader = {
        let last = last.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if json_str(&line, "kind").as_deref() == Some("summary") {
                    *last.lock().unwrap() = Some(line);
                }
            }
        })
    };
    let deadline = Instant::now() + p.stay + Duration::from_secs(30);
    let mut stop_seen: Option<Instant> = None;
    let mut killed = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {}
            Err(_) => break None,
        }
        let now = Instant::now();
        if stop.load(Ordering::Relaxed) {
            stop_seen.get_or_insert(now);
        }
        if now >= deadline || stop_seen.is_some_and(|t| now - t > Duration::from_secs(10)) {
            let _ = child.kill();
            killed = true;
            break child.wait().ok();
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let _ = reader.join();
    shared.running.fetch_sub(1, Ordering::Relaxed);
    out.exit = status.and_then(|s| s.code());
    match last.lock().unwrap().take() {
        Some(line) => fill(&mut out, &line),
        None => {
            out.end = if killed { "killed" } else { "no summary" }.into();
        }
    }
    out
}

fn fill(o: &mut ExtOutcome, line: &str) {
    o.version = json_str(line, "version").unwrap_or_default();
    o.end = json_str(line, "end").unwrap_or_else(|| "?".into());
    o.error = json_str(line, "error").unwrap_or_default();
    o.login_ms = json_num(line, "login_ms").filter(|_| o.end != "nologin");
    o.warps = json_num(line, "warps").unwrap_or(0.0) as u64;
    o.warps_done = json_num(line, "warps_done").unwrap_or(0.0) as u64;
    o.warp_ms_p50 = json_num(line, "warp_ms_p50").filter(|_| o.warps_done > 0);
    o.warp_ms_max = json_num(line, "warp_ms_max").filter(|_| o.warps_done > 0);
    o.tick_ms_p99 = json_num(line, "tick_ms_p99");
    o.tick_ms_max = json_num(line, "tick_ms_max");
    o.frames_in = json_num(line, "frames_in").unwrap_or(0.0) as u64;
    o.msgs_out = json_num(line, "msgs_out").unwrap_or(0.0) as u64;
    o.rss_peak_kib = json_num(line, "rss_peak_kib").map(|v| v as u64);
    o.nodelay = json_raw(line, "nodelay").and_then(|v| match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    });
}

/// The raw value text of `"key":` in a flat JSON object: up to the next
/// `,` or `}` for a bare value, or the quoted string with its quotes.
fn json_raw<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let at = line.find(&pat)? + pat.len();
    let rest = line[at..].trim_start();
    if let Some(body) = rest.strip_prefix('"') {
        let mut esc = false;
        for (i, c) in body.char_indices() {
            match (esc, c) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => return Some(&rest[..i + 2]),
                _ => {}
            }
        }
        None
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

/// A string value, with `\"`, `\\`, `\n` and `\t` unescaped.
pub fn json_str(line: &str, key: &str) -> Option<String> {
    let raw = json_raw(line, key)?;
    let body = raw.strip_prefix('"')?.strip_suffix('"')?;
    let mut s = String::with_capacity(body.len());
    let mut it = body.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some(o) => s.push(o),
                None => {}
            }
        } else {
            s.push(c);
        }
    }
    Some(s)
}

/// A number value; `None` for `null` or anything that is not a number.
pub fn json_num(line: &str, key: &str) -> Option<f64> {
    json_raw(line, key)?.parse().ok()
}

/// One row per external client.
pub fn tsv(o: &[ExtOutcome]) -> String {
    let mut s = String::from(
        "account\tversion\texit\tend\tlogin_ms\twarps\twarps_done\twarp_ms_p50\twarp_ms_max\ttick_ms_p99\ttick_ms_max\tframes_in\tmsgs_out\trss_peak_kib\tnodelay\terror\n",
    );
    let f = |v: Option<f64>| v.map_or(String::new(), |v| format!("{v:.1}"));
    for x in o {
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            x.account,
            x.version,
            x.exit.map_or(String::new(), |c| c.to_string()),
            x.end,
            f(x.login_ms),
            x.warps,
            x.warps_done,
            f(x.warp_ms_p50),
            f(x.warp_ms_max),
            f(x.tick_ms_p99),
            f(x.tick_ms_max),
            x.frames_in,
            x.msgs_out,
            x.rss_peak_kib.map_or(String::new(), |v| v.to_string()),
            x.nodelay.map_or(String::new(), |v| v.to_string()),
            x.error.replace(['\t', '\n'], " ")
        );
    }
    s
}

/// The report's section on external clients; empty when there were none.
pub fn markdown(o: &[ExtOutcome], program: Option<&Path>) -> String {
    if o.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n## External clients\n\n");
    if let Some(p) = program {
        let _ = writeln!(s, "Program: `{}`.\n", p.display());
    }
    let mut versions: Vec<&str> = o
        .iter()
        .map(|x| x.version.as_str())
        .filter(|v| !v.is_empty())
        .collect();
    versions.sort_unstable();
    versions.dedup();
    let count = |e: &str| o.iter().filter(|x| x.end == e).count();
    let other = o.len() - count("clean") - count("nologin") - count("dropped");
    let login: Vec<f64> = o.iter().filter_map(|x| x.login_ms).collect();
    let warps: u64 = o.iter().map(|x| x.warps).sum();
    let done: u64 = o.iter().map(|x| x.warps_done).sum();
    let wmax = o.iter().filter_map(|x| x.warp_ms_max).fold(None, max);
    let tick99: Vec<f64> = o.iter().filter_map(|x| x.tick_ms_p99).collect();
    let rss: Vec<f64> = o
        .iter()
        .filter_map(|x| x.rss_peak_kib)
        .map(|v| v as f64 / 1024.0)
        .collect();
    let nodelay: Vec<bool> = o.iter().filter_map(|x| x.nodelay).collect();
    let _ = writeln!(
        s,
        "- Clients: {} ({}); ended clean: {}; never logged in: {}; dropped: {}; other: {other}.",
        o.len(),
        if versions.is_empty() {
            "no version reported".into()
        } else {
            versions.join(", ")
        },
        count("clean"),
        count("nologin"),
        count("dropped"),
    );
    let _ = writeln!(s, "- Login ms p50/p95/p99/max: {}.", fmt4(summary(&login)));
    let _ = writeln!(
        s,
        "- Links taken: {done} of {warps} tried; slowest {} ms (the client polls each 50 ms tick).",
        wmax.map_or("-".into(), |v| format!("{v:.0}"))
    );
    let _ = writeln!(
        s,
        "- Client tick p99 ms, p50/max over clients: {}; peak RSS MiB p50/max: {}.",
        pair(summary(&tick99)),
        pair(summary(&rss))
    );
    let _ = writeln!(
        s,
        "- TCP_NODELAY: {}.",
        match (nodelay.iter().any(|v| *v), nodelay.iter().any(|v| !*v)) {
            (true, false) => "on",
            (false, true) => "off",
            (true, true) => "mixed",
            _ => "not reported",
        }
    );
    let bad: Vec<&ExtOutcome> = o.iter().filter(|x| x.end != "clean").take(5).collect();
    for x in bad {
        let _ = writeln!(
            s,
            "  - `{}`: {} (exit {}){}",
            x.account,
            x.end,
            x.exit.map_or("-".into(), |c| c.to_string()),
            if x.error.is_empty() {
                String::new()
            } else {
                format!(": {}", x.error)
            }
        );
    }
    s
}

fn max(a: Option<f64>, b: f64) -> Option<f64> {
    Some(a.map_or(b, |a| a.max(b)))
}

fn fmt4(v: Option<[f64; 4]>) -> String {
    v.map_or("-".into(), |v| {
        format!("{:.1} / {:.1} / {:.1} / {:.1}", v[0], v[1], v[2], v[3])
    })
}

fn pair(v: Option<[f64; 4]>) -> String {
    v.map_or("-".into(), |v| format!("{:.2} / {:.2}", v[0], v[3]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = r#"{"kind":"summary","version":"2.05r2","token":"GNW03081","account":"a1","end":"clean","error":"a \"b\"","login_ms":102,"warps":3,"warps_done":2,"warp_ms_p50":50,"warp_ms_max":100,"tick_ms_p99":0.290,"tick_ms_max":0.411,"frames_in":10,"msgs_out":103,"rss_peak_kib":null,"nodelay":false}"#;

    #[test]
    fn reads_the_summary_line() {
        assert_eq!(json_str(LINE, "kind").as_deref(), Some("summary"));
        assert_eq!(json_str(LINE, "error").as_deref(), Some("a \"b\""));
        assert_eq!(json_num(LINE, "login_ms"), Some(102.0));
        assert_eq!(json_num(LINE, "tick_ms_p99"), Some(0.29));
        assert_eq!(json_num(LINE, "rss_peak_kib"), None);
        assert_eq!(json_num(LINE, "missing"), None);
        let mut o = ExtOutcome::default();
        fill(&mut o, LINE);
        assert_eq!(o.version, "2.05r2");
        assert_eq!(o.end, "clean");
        assert_eq!((o.warps, o.warps_done), (3, 2));
        assert_eq!(o.warp_ms_max, Some(100.0));
        assert_eq!(o.nodelay, Some(false));
        assert_eq!(o.rss_peak_kib, None);
        let md = markdown(&[o.clone()], None);
        assert!(md.contains("ended clean: 1"));
        assert!(md.contains("Links taken: 2 of 3"));
        assert!(md.contains("TCP_NODELAY: off"));
        assert_eq!(tsv(&[o]).lines().count(), 2);
    }

    #[test]
    fn args_follow_the_plan() {
        let p = ExtPlan {
            index: 4,
            program: "x".into(),
            extra: vec!["--nodelay".into()],
            account: "acc".into(),
            password: "pw".into(),
            gserver: "127.0.0.1:1".into(),
            stay: Duration::from_millis(1500),
            walk_every: Duration::from_millis(100),
            link_every: Duration::ZERO,
            log: "l".into(),
        };
        let a = p.args().join(" ");
        assert_eq!(
            a,
            "--server 127.0.0.1:1 --account acc --password-env SEIHAI_LAB_PASSWORD --headless --seconds 2 --walk random --walk-ms 100 --seed 5 --nodelay"
        );
    }
}
