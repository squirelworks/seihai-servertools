//! Start a set of server processes, send each console to a file, and stop
//! them together. Used by the one-step lab start ([`crate::lab`]) and later
//! by the setup wizard's "Start and check" step.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// One process to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpec {
    /// Short name for messages (`listserver`, `default`, `classic`).
    pub label: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Working folder.
    pub cwd: PathBuf,
    /// Standard output and error go here (the folder is created).
    pub console: PathBuf,
}

/// A started process.
pub struct Running {
    pub spec: ProcessSpec,
    child: Child,
    exited: Option<ExitStatus>,
}

impl Running {
    /// The process id.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// The exit status, if it has stopped.
    pub fn poll(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.exited.is_none() {
            self.exited = self.child.try_wait()?;
        }
        Ok(self.exited)
    }
}

/// Start one process with its console redirected to `spec.console`.
pub fn start(spec: &ProcessSpec) -> io::Result<Running> {
    if let Some(dir) = spec.console.parent() {
        fs::create_dir_all(dir)?;
    }
    let out = File::create(&spec.console)?;
    let err = out.try_clone()?;
    let child = Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", spec.program.display())))?;
    Ok(Running {
        spec: spec.clone(),
        child,
        exited: None,
    })
}

/// The last `lines` lines of a console file (empty if it cannot be read).
pub fn tail(path: &std::path::Path, lines: usize) -> String {
    let Ok(mut f) = File::open(path) else {
        return String::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(16 * 1024)));
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Ask a process to stop: SIGTERM on Unix, so a gserver can save; a hard
/// stop on Windows, where console programs have no gentler signal from
/// outside.
fn ask_to_stop(r: &mut Running) {
    #[cfg(unix)]
    {
        // SAFETY: kill(2) with a pid we started and have not reaped.
        unsafe {
            libc::kill(r.child.id() as libc::pid_t, libc::SIGTERM);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = r.child.kill();
    }
}

/// A group of processes that stop together.
#[derive(Default)]
pub struct Group {
    pub procs: Vec<Running>,
}

impl Group {
    pub fn push(&mut self, r: Running) {
        self.procs.push(r);
    }

    /// The processes that have stopped, with their exit status.
    pub fn stopped(&mut self) -> Vec<(String, ExitStatus)> {
        let mut out = Vec::new();
        for r in &mut self.procs {
            if let Ok(Some(status)) = r.poll() {
                out.push((r.spec.label.clone(), status));
            }
        }
        out
    }

    /// Wait until `stop` is set or every process has stopped. Returns true
    /// when `stop` ended the wait.
    pub fn wait(&mut self, stop: &AtomicBool) -> bool {
        loop {
            if stop.load(Ordering::SeqCst) {
                return true;
            }
            if self.stopped().len() == self.procs.len() {
                return false;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Stop every process still running: ask, wait up to `grace`, then kill.
    pub fn stop_all(&mut self, grace: Duration) {
        for r in &mut self.procs {
            if matches!(r.poll(), Ok(None)) {
                ask_to_stop(r);
            }
        }
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline && self.stopped().len() < self.procs.len() {
            thread::sleep(Duration::from_millis(50));
        }
        for r in &mut self.procs {
            if matches!(r.poll(), Ok(None)) {
                let _ = r.child.kill();
                let _ = r.child.wait();
            }
        }
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        self.stop_all(Duration::from_secs(5));
    }
}
