//! One bot client: optionally fetch the server list, log in to the gserver
//! as one version profile, then walk, chat and probe with file requests
//! until the run ends.
//!
//! Everything a bot sends is what that build may send
//! ([`seihai_core::net::Connection::can_send`]); nothing is padded or
//! invented.

use seihai_core::net::{login_gserver, random_key, walk_message_for, wantfile_message, Connection};
use seihai_core::protocol::{ClientVersion, SessionState};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const PLI_PLAYERPROPS: u8 = 2;
const PLI_WANTFILE: u8 = 23;
const PLO_FILESENDFAILED: u8 = 30;

/// Counters every bot updates while the run goes, read once a second for
/// the timeline.
#[derive(Default)]
pub struct Shared {
    pub online: AtomicUsize,
    pub logins_ok: AtomicUsize,
    pub logins_failed: AtomicUsize,
    pub dropped: AtomicUsize,
    pub sent_msgs: AtomicU64,
    pub frames_in: AtomicU64,
    pub stop: AtomicBool,
}

/// What one bot does.
#[derive(Debug, Clone)]
pub struct Plan {
    pub index: usize,
    pub version: ClientVersion,
    pub account: String,
    pub password: String,
    pub gserver: String,
    /// Fetch the list from here first, as a real client does.
    pub listserver: Option<String>,
    /// The name the gserver registers as, to find its row in the list.
    pub server_name: String,
    /// How long to stay once logged in.
    pub stay: Duration,
    pub walk_every: Duration,
    pub chat_every: Duration,
    pub probe_every: Duration,
}

/// How a bot's session ended.
#[derive(Debug, Clone, PartialEq)]
pub enum End {
    /// Stayed for the whole plan and closed its own link.
    Clean,
    /// Never got in.
    NoLogin,
    /// The server closed the link or sent a disconnect message.
    Dropped(String),
}

/// What one bot saw.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub version: &'static str,
    /// The list fetch: time taken and the player count our server showed,
    /// or why it failed. `None` when the plan had no list step.
    pub list: Option<Result<(Duration, Option<u16>), String>>,
    /// Connect to level-and-board, or why the login failed.
    pub login: Result<Duration, String>,
    /// File-request round trips that came back.
    pub probes: Vec<Duration>,
    /// File requests with no answer within 10 s.
    pub probes_lost: u32,
    pub sent_msgs: u64,
    pub frames_in: u64,
    pub online_for: Duration,
    pub end: End,
}

/// Read until `done`, `limit` or a disconnect.
fn read_until(
    rx: &Receiver<io::Result<Vec<u8>>>,
    st: &mut SessionState,
    limit: Duration,
    frames: &mut u64,
    done: impl Fn(&SessionState) -> bool,
) -> Result<(), String> {
    let end = Instant::now() + limit;
    while !done(st) {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("timed out waiting for the level".into());
        }
        match rx.recv_timeout(left) {
            Ok(Ok(body)) => {
                *frames += 1;
                st.apply_body(&body);
            }
            Ok(Err(e)) => return Err(format!("connection: {e}")),
            Err(_) => return Err("timed out waiting for the level".into()),
        }
        if let Some(m) = &st.disconnect_message {
            return Err(format!("disconnected: {m}"));
        }
    }
    Ok(())
}

/// The list step. Returns the time and the player count shown for
/// `server_name`.
fn fetch_list(plan: &Plan, addr: &str) -> Result<(Duration, Option<u16>), String> {
    let t = Instant::now();
    let reply = Connection::fetch_server_list(addr, plan.version, &plan.account, &plan.password)
        .map_err(|e| e.to_string())?;
    if let Some(e) = reply.error {
        return Err(format!("listserver said: {e}"));
    }
    let players = reply
        .servers
        .iter()
        .find(|s| s.name.contains(&plan.server_name))
        .and_then(|s| s.players);
    Ok((t.elapsed(), players))
}

/// Run one bot to its end.
pub fn run(plan: Plan, shared: &Shared) -> Outcome {
    let mut out = Outcome {
        version: plan.version.label(),
        list: None,
        login: Err("not tried".into()),
        probes: Vec::new(),
        probes_lost: 0,
        sent_msgs: 0,
        frames_in: 0,
        online_for: Duration::ZERO,
        end: End::NoLogin,
    };
    if let Some(addr) = &plan.listserver {
        out.list = Some(fetch_list(&plan, addr));
    }
    let v = plan.version;
    let t0 = Instant::now();
    let mut sess = match login_gserver(
        &plan.gserver,
        v,
        random_key(),
        &plan.account,
        &plan.password,
        false,
    ) {
        Ok(s) => s,
        Err(e) => {
            out.login = Err(e.to_string());
            shared.logins_failed.fetch_add(1, Ordering::Relaxed);
            return out;
        }
    };
    let rx = match sess.conn.spawn_reader() {
        Ok(rx) => rx,
        Err(e) => {
            out.login = Err(format!("reader: {e}"));
            shared.logins_failed.fetch_add(1, Ordering::Relaxed);
            return out;
        }
    };
    let mut st = SessionState::for_version(v);
    st.file_has_modtime = v.file_has_modtime();
    if let Err(e) = read_until(
        &rx,
        &mut st,
        Duration::from_secs(20),
        &mut out.frames_in,
        |s| s.board.is_some() && s.current_level_name().is_some(),
    ) {
        out.login = Err(e);
        shared.logins_failed.fetch_add(1, Ordering::Relaxed);
        let _ = sess.conn.shutdown();
        return out;
    }
    out.login = Ok(t0.elapsed());
    shared.logins_ok.fetch_add(1, Ordering::Relaxed);
    shared.online.fetch_add(1, Ordering::Relaxed);
    let online_at = Instant::now();

    let conn = &mut sess.conn;
    let walks = conn.can_send(PLI_PLAYERPROPS);
    let probes =
        conn.can_send(PLI_WANTFILE) && v.profile().protocol.plo.contains(PLO_FILESENDFAILED);
    // Stagger the timers so bots do not all act on the same tick.
    let jitter = Duration::from_millis((plan.index as u64 * 37) % 1000);
    let mut next_walk = Instant::now() + plan.walk_every.min(jitter);
    let mut next_chat = Instant::now() + plan.chat_every + jitter;
    let mut next_probe = Instant::now() + jitter;
    let mut pending: Option<(String, Instant)> = None;
    let mut probe_n = 0u32;
    let mut step = 0u32;
    let (x0, y0) = (30.0f32, 30.5f32);
    let end_at = Instant::now() + plan.stay;
    let send = |conn: &mut Connection, m: Vec<u8>, out: &mut Outcome| -> Result<(), String> {
        conn.send_messages(&[m]).map_err(|e| format!("send: {e}"))?;
        out.sent_msgs += 1;
        shared.sent_msgs.fetch_add(1, Ordering::Relaxed);
        Ok(())
    };

    out.end = loop {
        if shared.stop.load(Ordering::Relaxed) || Instant::now() >= end_at {
            break End::Clean;
        }
        let now = Instant::now();
        let mut wake = end_at.min(next_probe).min(next_chat).min(next_walk);
        if let Some((_, at)) = &pending {
            wake = wake.min(*at + Duration::from_secs(10));
        }
        match rx.recv_timeout(wake.saturating_duration_since(now)) {
            Ok(Ok(body)) => {
                out.frames_in += 1;
                shared.frames_in.fetch_add(1, Ordering::Relaxed);
                st.apply_body(&body);
                if let Some(m) = &st.disconnect_message {
                    break End::Dropped(format!("disconnected: {m}"));
                }
                if let Some((name, at)) = &pending {
                    let want = format!("FAIL:{name}");
                    if st.file_requests.iter().any(|r| r == &want) {
                        out.probes.push(at.elapsed());
                        pending = None;
                    }
                }
                st.file_requests.clear();
                continue;
            }
            Ok(Err(e)) => break End::Dropped(format!("connection: {e}")),
            Err(RecvTimeoutError::Disconnected) => break End::Dropped("reader ended".into()),
            Err(RecvTimeoutError::Timeout) => {}
        }
        let now = Instant::now();
        if let Some((_, at)) = &pending {
            if now >= *at + Duration::from_secs(10) {
                out.probes_lost += 1;
                pending = None;
            }
        }
        let r = (|| -> Result<(), String> {
            if walks && now >= next_walk {
                step = step.wrapping_add(1);
                let a = step as f32 * 0.2;
                let (x, y) = (x0 + 4.0 * a.cos(), y0 + 4.0 * a.sin());
                send(conn, walk_message_for(v, x, y, (step % 4) as u8), &mut out)?;
                next_walk = now + plan.walk_every;
            }
            if walks && now >= next_chat {
                let text = format!("stress {} says {}", plan.index, step);
                conn.send_chat(&text).map_err(|e| format!("send: {e}"))?;
                out.sent_msgs += 1;
                shared.sent_msgs.fetch_add(1, Ordering::Relaxed);
                next_chat = now + plan.chat_every;
            }
            if probes && pending.is_none() && now >= next_probe {
                probe_n += 1;
                let name = format!("stressprobe{}_{probe_n}.png", plan.index);
                send(conn, wantfile_message(&name), &mut out)?;
                pending = Some((name, Instant::now()));
                next_probe = now + plan.probe_every;
            } else if now >= next_probe {
                next_probe = now + plan.probe_every;
            }
            Ok(())
        })();
        if let Err(e) = r {
            break End::Dropped(e);
        }
    };
    // A probe still in flight when the run ends is neither answered nor lost.
    out.online_for = online_at.elapsed();
    shared.online.fetch_sub(1, Ordering::Relaxed);
    if matches!(out.end, End::Dropped(_)) {
        shared.dropped.fetch_add(1, Ordering::Relaxed);
    }
    let _ = sess.conn.shutdown();
    out
}
