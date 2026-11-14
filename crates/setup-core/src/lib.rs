//! setup-core: everything `seihai-servertools` knows about starting and
//! configuring servers, with no UI. The command line (`seihai-servertools`)
//! and the window (`servertools-app`) both call into this crate, so every
//! rule is written and tested once.
//!
//! | Module | What | State |
//! |---|---|---|
//! | [`browse`] | The folder picker's listing: subfolders, and which look like world folders | working |
//! | [`folder`] | One server folder: open, preview a change, check, save with backup, undo | working |
//! | [`lab`] | Find the binaries of a lab world and plan the one-step start | working |
//! | [`lab_run`] | Run a lab plan on a worker thread for the window, with status and stop | working |
//! | [`run`] | Start a set of processes, log their consoles, stop them together | working |
//! | [`config`] | `serveroptions.txt` (read, set, diff), backups and undo; listserver TOML (scan) | working; TOML editor in step 6 |
//! | [`profile`] | Key tables, detect, Back to lab, apply the public answers | working |
//! | [`checks`] | Safety check and the combinations the wizard refuses | working |
//! | [`lights`] | The three "Start and check" lights, read from a gserver console | working |
//! | [`stats`] | Leaderboard pages from a seihai-gserver world's accounts (gserverstat's job) | working |
//! | [`net`] | "Test connection" and the "Listen on" address list (working); opt-in UPnP (planned) | partly |
//!
//! The plan for each module is in `docs/ARCHITECTURE.md` and the order of
//! the work in `docs/NEXT.md`. Rules: `AGENTS.md`.

pub mod browse;
pub mod capacity;
pub mod checks;
pub mod config;
pub mod folder;
pub mod lab;
pub mod lab_run;
pub mod lights;
pub mod net;
pub mod profile;
pub mod run;
pub mod stamp;
pub mod stats;
