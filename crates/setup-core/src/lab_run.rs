//! Run a lab [`Plan`] on a worker thread, for the window's Lab page: the
//! same order as `seihai-servertools lab start` (listserver first, wait for
//! its gserver port, then the gservers, then a short check that none died
//! straight away), with a [`Status`] the page reads every frame and a stop
//! that ends them all together.

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::lab::Plan;
use crate::run::{start, tail, Group, ProcessSpec};

/// Where a run is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Running,
    Stopping,
    /// Every process has stopped, after a stop or on its own.
    Stopped,
    /// Starting failed; whatever had started was stopped again.
    Failed(String),
}

/// One process of the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcStatus {
    pub label: String,
    pub pid: u32,
    /// Its console file (see [`tail`]).
    pub console: PathBuf,
    /// How it ended, once it has.
    pub exited: Option<String>,
}

/// What the worker thread reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub procs: Vec<ProcStatus>,
    /// What happened, oldest first ("started listserver, pid 12").
    pub log: Vec<String>,
}

/// Wait until something accepts on `127.0.0.1:port`, up to `limit`, or
/// until `stop` is set.
pub fn wait_for_port(port: u16, limit: Duration, stop: &AtomicBool) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline && !stop.load(Ordering::SeqCst) {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

/// A lab world running on a worker thread. Dropping it stops the servers.
pub struct LabRun {
    status: Arc<Mutex<Status>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

/// How long a server has to die straight away (port in use, bad config)
/// before the run counts as started.
pub const SETTLE: Duration = Duration::from_secs(2);

/// How long servers get to stop on their own before they are killed.
pub const GRACE: Duration = Duration::from_secs(5);

impl LabRun {
    /// Start the plan's processes on a new thread and return at once.
    pub fn start(plan: Plan) -> LabRun {
        let status = Arc::new(Mutex::new(Status {
            phase: Phase::Starting,
            procs: Vec::new(),
            log: Vec::new(),
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let status = status.clone();
            let stop = stop.clone();
            thread::spawn(move || work(plan, &status, &stop))
        };
        LabRun {
            status,
            stop,
            worker: Some(worker),
        }
    }

    /// A copy of the current status.
    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }

    /// Ask every server to stop. [`Self::status`] goes to
    /// [`Phase::Stopped`] once they have.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// True once the worker thread has ended (stopped or failed).
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// The last `lines` lines of each process's console, in start order.
    pub fn tails(&self, lines: usize) -> Vec<(String, String)> {
        self.status()
            .procs
            .iter()
            .map(|p| (p.label.clone(), tail(&p.console, lines)))
            .collect()
    }
}

impl Drop for LabRun {
    fn drop(&mut self) {
        self.stop();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

fn lock(s: &Mutex<Status>) -> std::sync::MutexGuard<'_, Status> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

fn say(status: &Mutex<Status>, line: String) {
    lock(status).log.push(line);
}

/// Copy each process's exit into the status. Returns how many have ended.
fn refresh(group: &mut Group, status: &Mutex<Status>) -> usize {
    let stopped = group.stopped();
    let mut s = lock(status);
    for (label, how) in &stopped {
        if let Some(p) = s.procs.iter_mut().find(|p| &p.label == label) {
            if p.exited.is_none() {
                p.exited = Some(how.to_string());
            }
        }
    }
    stopped.len()
}

fn launch(spec: &ProcessSpec, group: &mut Group, status: &Mutex<Status>) -> Result<(), String> {
    let r = start(spec).map_err(|e| e.to_string())?;
    {
        let mut s = lock(status);
        s.log
            .push(format!("started {}, pid {}", spec.label, r.id()));
        s.procs.push(ProcStatus {
            label: spec.label.clone(),
            pid: r.id(),
            console: spec.console.clone(),
            exited: None,
        });
    }
    group.push(r);
    Ok(())
}

fn fail(group: &mut Group, status: &Mutex<Status>, why: String) {
    lock(status).phase = Phase::Stopping;
    group.stop_all(GRACE);
    refresh(group, status);
    let mut s = lock(status);
    s.log.push(why.clone());
    s.phase = Phase::Failed(why);
}

fn work(plan: Plan, status: &Mutex<Status>, stop: &AtomicBool) {
    let mut group = Group::default();
    for n in &plan.notes {
        say(status, format!("note: {n}"));
    }
    if let Some(l) = &plan.listserver {
        if let Err(e) = launch(&l.spec, &mut group, status) {
            return fail(&mut group, status, e);
        }
        if l.settings.is_loopback()
            && !wait_for_port(l.settings.gserver_port, GRACE, stop)
            && !stop.load(Ordering::SeqCst)
        {
            let why = format!(
                "the listserver is not accepting on 127.0.0.1:{} after {} s",
                l.settings.gserver_port,
                GRACE.as_secs()
            );
            return fail(&mut group, status, why);
        }
    }
    for g in &plan.gservers {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if let Err(e) = launch(&g.spec, &mut group, status) {
            return fail(&mut group, status, e);
        }
    }

    // A server that dies straight away is a failed start, reported with
    // the end of its console.
    let settle = Instant::now() + SETTLE;
    while Instant::now() < settle && !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
    if refresh(&mut group, status) > 0 && !stop.load(Ordering::SeqCst) {
        let dead: Vec<String> = lock(status)
            .procs
            .iter()
            .filter_map(|p| {
                p.exited
                    .as_ref()
                    .map(|how| format!("{} stopped ({how}):\n{}", p.label, tail(&p.console, 15)))
            })
            .collect();
        return fail(&mut group, status, dead.join("\n"));
    }

    lock(status).phase = Phase::Running;
    say(status, "lab world running".into());
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if refresh(&mut group, status) == group.procs.len() {
            say(status, "every server has stopped".into());
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    lock(status).phase = Phase::Stopping;
    group.stop_all(GRACE);
    refresh(&mut group, status);
    let mut s = lock(status);
    s.log.push("stopped".into());
    s.phase = Phase::Stopped;
}
