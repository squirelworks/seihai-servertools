//! Per-process samples: resident memory, CPU time, threads and open files.
//!
//! Read from `/proc` on Linux. Elsewhere [`read`] returns `None` and a run
//! reports no server samples (the bot side still works).

/// One reading of one process.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProcSample {
    /// Resident set, KiB.
    pub rss_kib: u64,
    /// User plus system CPU time so far, seconds.
    pub cpu_secs: f64,
    pub threads: u32,
    pub fds: u32,
}

/// Read process `pid` now.
#[cfg(target_os = "linux")]
pub fn read(pid: u32) -> Option<ProcSample> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let field = |name: &str| -> Option<u64> {
        status
            .lines()
            .find(|l| l.starts_with(name))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let cpu_secs = parse_stat_cpu(&stat)? as f64 / clock_ticks();
    let fds = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .map(|d| d.count() as u32)
        .unwrap_or(0);
    Some(ProcSample {
        rss_kib: field("VmRSS:").unwrap_or(0),
        cpu_secs,
        threads: field("Threads:").unwrap_or(0) as u32,
        fds,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn read(_pid: u32) -> Option<ProcSample> {
    None
}

/// The kernel's clock ticks per second. 100 on every Linux the rig is
/// meant for; `getconf` is asked once in case it is not.
#[cfg(target_os = "linux")]
fn clock_ticks() -> f64 {
    use std::sync::OnceLock;
    static HZ: OnceLock<f64> = OnceLock::new();
    *HZ.get_or_init(|| {
        std::process::Command::new("getconf")
            .arg("CLK_TCK")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(100.0)
    })
}

/// utime + stime (fields 14 and 15) of a `/proc/PID/stat` line, in ticks.
/// The command name in field 2 may hold spaces, so count from its `)`.
pub fn parse_stat_cpu(stat: &str) -> Option<u64> {
    let rest = &stat[stat.rfind(')')? + 1..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    // After ")": state is field 3, so utime (14) is index 11 here.
    Some(f.get(11)?.parse::<u64>().ok()? + f.get(12)?.parse::<u64>().ok()?)
}

/// The pid of a process whose parent is `parent` and whose command line
/// mentions `needle`, if any. Linux only.
#[cfg(target_os = "linux")]
pub fn find_child(parent: u32, needle: &str) -> Option<u32> {
    for e in std::fs::read_dir("/proc").ok()?.flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let ppid = stat
            .rfind(')')
            .and_then(|i| stat[i + 1..].split_whitespace().nth(1))
            .and_then(|s| s.parse::<u32>().ok());
        if ppid != Some(parent) {
            continue;
        }
        let cmd = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if String::from_utf8_lossy(&cmd).contains(needle) {
            return Some(pid);
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn find_child(_parent: u32, _needle: &str) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_cpu_with_spaces_in_name() {
        let line =
            "1234 (seihai gserver) S 1 1234 1234 0 -1 4194560 100 0 0 0 250 50 0 0 20 0 9 0 1 1 1";
        assert_eq!(parse_stat_cpu(line), Some(300));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reads_itself() {
        let s = read(std::process::id()).unwrap();
        assert!(s.rss_kib > 0);
        assert!(s.threads >= 1);
        assert!(s.fds >= 1);
    }
}
