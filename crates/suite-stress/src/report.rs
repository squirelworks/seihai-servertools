//! The run's results: one row per second for the timeline, and the
//! Markdown summary.

use crate::bot::{End, Outcome};
use crate::rc::RcProbe;
use crate::sample::ProcSample;
use crate::stats::{slope, summary};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Duration;

/// One second of the run.
#[derive(Debug, Clone, Default)]
pub struct Tick {
    pub t: f64,
    pub online: usize,
    pub logins_ok: usize,
    pub logins_failed: usize,
    pub dropped: usize,
    pub sent_msgs: u64,
    pub frames_in: u64,
    /// (process name, sample) for each server process that could be read.
    pub procs: Vec<(&'static str, ProcSample)>,
}

/// Everything a report is made from.
pub struct Run<'a> {
    pub title: String,
    pub settings: Vec<(String, String)>,
    pub outcomes: &'a [Outcome],
    pub ticks: &'a [Tick],
    pub rc: &'a [RcProbe],
    /// Server processes that exited during the run, with their status.
    pub exited: &'a [(&'static str, String)],
    /// (log name, trouble lines, first few).
    pub logs: &'a [(String, usize, Vec<String>)],
    /// The external clients' section, already written (empty for none).
    pub external: &'a str,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn fmt4(s: Option<[f64; 4]>) -> String {
    match s {
        Some([a, b, c, d]) => format!("{a:.1} / {b:.1} / {c:.1} / {d:.1}"),
        None => "-".into(),
    }
}

/// The timeline as tab-separated values, one row per tick.
pub fn timeline_tsv(ticks: &[Tick]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for t in ticks {
        for (n, _) in &t.procs {
            if !names.contains(n) {
                names.push(n);
            }
        }
    }
    let mut s = String::from("t\tonline\tlogins_ok\tlogins_failed\tdropped\tsent_msgs\tframes_in");
    for n in &names {
        let _ = write!(s, "\t{n}_rss_kib\t{n}_cpu_s\t{n}_threads\t{n}_fds");
    }
    s.push('\n');
    for t in ticks {
        let _ = write!(
            s,
            "{:.1}\t{}\t{}\t{}\t{}\t{}\t{}",
            t.t, t.online, t.logins_ok, t.logins_failed, t.dropped, t.sent_msgs, t.frames_in
        );
        for n in &names {
            match t.procs.iter().find(|(p, _)| p == n) {
                Some((_, p)) => {
                    let _ = write!(
                        s,
                        "\t{}\t{:.2}\t{}\t{}",
                        p.rss_kib, p.cpu_secs, p.threads, p.fds
                    );
                }
                None => s.push_str("\t\t\t\t"),
            }
        }
        s.push('\n');
    }
    s
}

/// One row per bot, tab-separated.
pub fn bots_tsv(outcomes: &[Outcome]) -> String {
    let mut s = String::from(
        "bot\tversion\tlist_ms\tlist_players\tlogin_ms\tlogin_error\tprobe_p50_ms\tprobes\tprobes_lost\twarp_p50_ms\twarps\twarps_lost\tsent\tframes\tonline_s\tend\n",
    );
    for (i, o) in outcomes.iter().enumerate() {
        let (list_ms, list_players) = match &o.list {
            Some(Ok((d, p))) => (
                format!("{:.1}", ms(*d)),
                p.map_or("-".into(), |p| p.to_string()),
            ),
            Some(Err(e)) => ("ERR".into(), e.replace('\t', " ")),
            None => ("-".into(), "-".into()),
        };
        let (login_ms, login_err) = match &o.login {
            Ok(d) => (format!("{:.1}", ms(*d)), String::new()),
            Err(e) => ("-".into(), e.replace('\t', " ")),
        };
        let p: Vec<f64> = o.probes.iter().map(|d| ms(*d)).collect();
        let p50 = crate::stats::percentile(&p, 50.0).map_or("-".into(), |v| format!("{v:.1}"));
        let w: Vec<f64> = o.warps.iter().map(|d| ms(*d)).collect();
        let w50 = crate::stats::percentile(&w, 50.0).map_or("-".into(), |v| format!("{v:.1}"));
        let end = match &o.end {
            End::Clean => "clean".to_string(),
            End::NoLogin => "no login".to_string(),
            End::Dropped(e) => format!("dropped: {}", e.replace('\t', " ")),
        };
        let _ = writeln!(
            s,
            "{i}\t{}\t{list_ms}\t{list_players}\t{login_ms}\t{login_err}\t{p50}\t{}\t{}\t{w50}\t{}\t{}\t{}\t{}\t{:.1}\t{end}",
            o.version,
            o.probes.len(),
            o.probes_lost,
            o.warps.len(),
            o.warps_lost,
            o.sent_msgs,
            o.frames_in,
            o.online_for.as_secs_f64()
        );
    }
    s
}

#[derive(Default)]
struct PerVersion {
    bots: usize,
    list_ok: usize,
    list_err: Vec<String>,
    login_ok: usize,
    login_err: Vec<String>,
    dropped: Vec<String>,
    login_ms: Vec<f64>,
    probe_ms: Vec<f64>,
    probes_lost: u32,
    warp_ms: Vec<f64>,
    warps_lost: u32,
}

/// The Markdown report.
pub fn markdown(run: &Run) -> String {
    let mut s = format!("# {}\n\n", run.title);
    s.push_str("| setting | value |\n|---|---|\n");
    for (k, v) in &run.settings {
        let _ = writeln!(s, "| {k} | {v} |");
    }

    let o = run.outcomes;
    let login_ok = o.iter().filter(|x| x.login.is_ok()).count();
    let dropped = o
        .iter()
        .filter(|x| matches!(x.end, End::Dropped(_)))
        .count();
    let all_login: Vec<f64> = o
        .iter()
        .filter_map(|x| x.login.as_ref().ok())
        .map(|d| ms(*d))
        .collect();
    let all_probe: Vec<f64> = o
        .iter()
        .flat_map(|x| x.probes.iter().map(|d| ms(*d)))
        .collect();
    let lost: u32 = o.iter().map(|x| x.probes_lost).sum();
    let all_warp: Vec<f64> = o
        .iter()
        .flat_map(|x| x.warps.iter().map(|d| ms(*d)))
        .collect();
    let warps_lost: u32 = o.iter().map(|x| x.warps_lost).sum();
    let all_cross: Vec<f64> = o
        .iter()
        .flat_map(|x| x.crossings.iter().map(|d| ms(*d)))
        .collect();
    let cross_lost: u32 = o.iter().map(|x| x.crossings_lost).sum();
    let on_gmap = o.iter().filter(|x| x.on_gmap).count();
    let list_tries = o.iter().filter(|x| x.list.is_some()).count();
    let list_ok = o.iter().filter(|x| matches!(x.list, Some(Ok(_)))).count();
    let list_ms: Vec<f64> = o
        .iter()
        .filter_map(|x| match &x.list {
            Some(Ok((d, _))) => Some(ms(*d)),
            _ => None,
        })
        .collect();
    let peak_online = run.ticks.iter().map(|t| t.online).max().unwrap_or(0);
    let secs = run.ticks.last().map_or(0.0, |t| t.t).max(1.0);
    let sent = run.ticks.last().map_or(0, |t| t.sent_msgs);
    let frames = run.ticks.last().map_or(0, |t| t.frames_in);

    s.push_str("\n## Totals\n\n");
    let _ = writeln!(s, "- Bots: {}; logged in: {login_ok}; dropped before their end: {dropped}; peak online at once: {peak_online}.", o.len());
    let _ = writeln!(
        s,
        "- List fetches: {list_ok} of {list_tries} worked; time p50/p95/p99/max ms: {}.",
        fmt4(summary(&list_ms))
    );
    let _ = writeln!(
        s,
        "- Login (connect to level and board) ms p50/p95/p99/max: {}.",
        fmt4(summary(&all_login))
    );
    let _ = writeln!(
        s,
        "- File-request round trip ms p50/p95/p99/max: {} ({} answered, {lost} lost).",
        fmt4(summary(&all_probe)),
        all_probe.len()
    );
    if !all_warp.is_empty() || warps_lost > 0 {
        let _ = writeln!(s, "- Level warp (link sent to the new level's name) ms p50/p95/p99/max: {} ({} done, {warps_lost} lost).", fmt4(summary(&all_warp)), all_warp.len());
    }
    if on_gmap > 0 {
        let mut gmap_versions: Vec<&str> = Vec::new();
        for x in o.iter().filter(|x| x.on_gmap) {
            if !gmap_versions.contains(&x.version) {
                gmap_versions.push(x.version);
            }
        }
        let _ = writeln!(s, "- On the gmap: {on_gmap} bots ({}). Seam crossing (crossing sent to the new cell's level name) ms p50/p95/p99/max: {} ({} done, {cross_lost} lost).", gmap_versions.join(", "), fmt4(summary(&all_cross)), all_cross.len());
    }
    let _ = writeln!(s, "- Client to server: {sent} messages ({:.0}/s). Server to client: {frames} frames ({:.0}/s).", sent as f64 / secs, frames as f64 / secs);

    // List player counts against how many were online then.
    let counts: Vec<u16> = o
        .iter()
        .filter_map(|x| match &x.list {
            Some(Ok((_, Some(p)))) => Some(*p),
            _ => None,
        })
        .collect();
    if let Some(max) = counts.iter().max() {
        let _ = writeln!(s, "- Highest player count the list showed for the gserver: {max} (peak online {peak_online}).");
    } else if list_ok > 0 {
        s.push_str(
            "- The list never showed a player count for the gserver (not listed, or no count).\n",
        );
    }

    s.push_str("\n## By version\n\n| version | bots | list ok | login ok | dropped | login ms p50/p95/p99/max | probe ms p50/p95/p99/max | probes lost | warp ms p50/p95/p99/max | warps lost | first error |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    let mut by: BTreeMap<&str, PerVersion> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for x in o {
        if !order.contains(&x.version) {
            order.push(x.version);
        }
        let e = by.entry(x.version).or_default();
        e.bots += 1;
        match &x.list {
            Some(Ok(_)) => e.list_ok += 1,
            Some(Err(m)) => e.list_err.push(m.clone()),
            None => {}
        }
        match &x.login {
            Ok(d) => {
                e.login_ok += 1;
                e.login_ms.push(ms(*d));
            }
            Err(m) => e.login_err.push(m.clone()),
        }
        if let End::Dropped(m) = &x.end {
            e.dropped.push(m.clone());
        }
        e.probe_ms.extend(x.probes.iter().map(|d| ms(*d)));
        e.probes_lost += x.probes_lost;
        e.warp_ms.extend(x.warps.iter().map(|d| ms(*d)));
        e.warps_lost += x.warps_lost;
    }
    for v in order {
        let e = &by[v];
        let first = e
            .login_err
            .first()
            .or(e.dropped.first())
            .or(e.list_err.first())
            .cloned()
            .unwrap_or_default()
            .replace('|', "/");
        let list = if e.list_ok + e.list_err.len() == 0 {
            "-".to_string()
        } else {
            e.list_ok.to_string()
        };
        let _ = writeln!(
            s,
            "| {v} | {} | {list} | {} | {} | {} | {} | {} | {} | {} | {first} |",
            e.bots,
            e.login_ok,
            e.dropped.len(),
            fmt4(summary(&e.login_ms)),
            fmt4(summary(&e.probe_ms)),
            e.probes_lost,
            fmt4(summary(&e.warp_ms)),
            e.warps_lost
        );
    }

    s.push_str(run.external);
    s.push_str("\n## Server processes\n\n");
    let mut names: Vec<&str> = Vec::new();
    for t in run.ticks {
        for (n, _) in &t.procs {
            if !names.contains(n) {
                names.push(n);
            }
        }
    }
    if names.is_empty() {
        s.push_str("No samples (they are read from /proc, so Linux only).\n");
    } else {
        s.push_str("| process | RSS start → peak → end MiB | RSS slope MiB/min (second half) | CPU s used | mean CPU % | peak threads | peak fds |\n|---|---|---|---|---|---|---|\n");
        for n in names {
            let pts: Vec<(f64, ProcSample)> = run
                .ticks
                .iter()
                .filter_map(|t| {
                    t.procs
                        .iter()
                        .find(|(p, _)| *p == n)
                        .map(|(_, x)| (t.t, *x))
                })
                .collect();
            let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
                continue;
            };
            let mib = |k: u64| k as f64 / 1024.0;
            let peak = pts.iter().map(|p| p.1.rss_kib).max().unwrap_or(0);
            let half = &pts[pts.len() / 2..];
            let sl = slope(
                &half
                    .iter()
                    .map(|p| (p.0 / 60.0, mib(p.1.rss_kib)))
                    .collect::<Vec<_>>(),
            );
            let cpu = last.1.cpu_secs - first.1.cpu_secs;
            let span = (last.0 - first.0).max(1.0);
            let _ = writeln!(
                s,
                "| {n} | {:.1} → {:.1} → {:.1} | {} | {cpu:.1} | {:.0} | {} | {} |",
                mib(first.1.rss_kib),
                mib(peak),
                mib(last.1.rss_kib),
                sl.map_or("-".into(), |v| format!("{v:+.2}")),
                100.0 * cpu / span,
                pts.iter().map(|p| p.1.threads).max().unwrap_or(0),
                pts.iter().map(|p| p.1.fds).max().unwrap_or(0),
            );
        }
    }
    if run.exited.is_empty() {
        s.push_str("\nNo server process exited during the run.\n");
    } else {
        for (n, st) in run.exited {
            let _ = writeln!(s, "\n**{n} exited during the run: {st}.**");
        }
    }

    if !run.rc.is_empty() {
        s.push_str("\n## RC probes\n\n| probe | runs | ok | time ms p50/max | last note |\n|---|---|---|---|---|\n");
        for what in ["servers", "players list"] {
            let p: Vec<&RcProbe> = run.rc.iter().filter(|r| r.what == what).collect();
            if p.is_empty() {
                continue;
            }
            let t: Vec<f64> = p.iter().map(|r| ms(r.took)).collect();
            let ok = p.iter().filter(|r| r.ok).count();
            let _ = writeln!(
                s,
                "| {what} | {} | {ok} | {:.0} / {:.0} | {} |",
                p.len(),
                crate::stats::percentile(&t, 50.0).unwrap_or(0.0),
                crate::stats::percentile(&t, 100.0).unwrap_or(0.0),
                p.last()
                    .map(|r| r.note.replace('|', "/"))
                    .unwrap_or_default()
            );
        }
    }

    s.push_str("\n## Server logs\n\n");
    for (name, n, ex) in run.logs {
        let _ = writeln!(s, "- `{name}`: {n} line(s) with panic/error/refused/full.");
        for l in ex {
            let _ = writeln!(s, "  - `{}`", l.replace('`', "'"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(v: &'static str, ok: bool) -> Outcome {
        Outcome {
            version: v,
            list: Some(Ok((Duration::from_millis(5), Some(3)))),
            login: if ok {
                Ok(Duration::from_millis(20))
            } else {
                Err("refused".into())
            },
            probes: vec![Duration::from_millis(2)],
            probes_lost: 0,
            warps: vec![Duration::from_millis(9)],
            warps_lost: 0,
            on_gmap: false,
            crossings: vec![],
            crossings_lost: 0,
            sent_msgs: 10,
            frames_in: 4,
            online_for: Duration::from_secs(1),
            end: if ok { End::Clean } else { End::NoLogin },
        }
    }

    #[test]
    fn report_has_each_version() {
        let o = vec![outcome("2.05", true), outcome("1.21", false)];
        let ticks = vec![Tick {
            t: 1.0,
            online: 1,
            procs: vec![(
                "gserver",
                ProcSample {
                    rss_kib: 2048,
                    cpu_secs: 0.5,
                    threads: 4,
                    fds: 9,
                },
            )],
            ..Default::default()
        }];
        let md = markdown(&Run {
            title: "t".into(),
            settings: vec![("bots".into(), "2".into())],
            outcomes: &o,
            ticks: &ticks,
            rc: &[],
            exited: &[],
            logs: &[],
            external: "",
        });
        assert!(md.contains("| 2.05 | 1 |"));
        assert!(md.contains("| 1.21 | 1 |"));
        assert!(md.contains("refused"));
        assert!(md.contains("| gserver |"));
        let tsv = bots_tsv(&o);
        assert_eq!(tsv.lines().count(), 3);
        assert!(timeline_tsv(&ticks).contains("gserver_rss_kib"));
    }
}
