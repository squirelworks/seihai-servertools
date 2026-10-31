//! setup-core: everything `seihai-servertools` knows about starting and
//! configuring servers, with no UI. The command line (`seihai-servertools`)
//! and the window (`servertools-app`) both call into this crate, so every
//! rule is written and tested once.
//!
//! | Module | What | State |
//! |---|---|---|
//! | [`lab`] | Find the binaries of a lab world and plan the one-step start | working |
//! | [`run`] | Start a set of processes, log their consoles, stop them together | working |
//! | [`config`] | `serveroptions.txt` (read, set, diff), backups and undo; listserver TOML (scan) | working; TOML editor in step 6 |
//! | [`profile`] | Key tables, detect, Back to lab, apply the public answers | working |
//! | [`checks`] | Safety check and the combinations the wizard refuses | working |
//! | [`lights`] | The three "Start and check" lights, read from a gserver console | working |
//! | [`net`] | "Test connection" (working); interfaces, opt-in UPnP (planned) | partly |
//!
//! The plan for each module is in `docs/ARCHITECTURE.md` and the order of
//! the work in `docs/NEXT.md`. Rules: `AGENTS.md`.

pub mod checks;
pub mod config;
pub mod lab;
pub mod lights;
pub mod net;
pub mod profile;
pub mod run;
pub mod stamp;
