//! setup-core: everything `seihai-servertools` knows about starting and
//! configuring servers, with no UI. The command line (`seihai-servertools`)
//! and the window (`servertools-app`) both call into this crate, so every
//! rule is written and tested once.
//!
//! | Module | What | State |
//! |---|---|---|
//! | [`lab`] | Find the binaries of a lab world and plan the one-step start | working |
//! | [`run`] | Start a set of processes, log their consoles, stop them together | working |
//! | [`config`] | Read and (later) write `serveroptions.txt` and the listserver TOML | reader only |
//! | [`profile`] | The lab / public / host-listserver key tables, as data | tables only |
//! | [`checks`] | Safety check and the combinations the wizard refuses | types only |
//! | [`net`] | Interfaces, "test connection", opt-in UPnP | planned |
//!
//! The plan for each module is in `docs/ARCHITECTURE.md` and the order of
//! the work in `docs/NEXT.md`. Rules: `AGENTS.md`.

pub mod checks;
pub mod config;
pub mod lab;
pub mod net;
pub mod profile;
pub mod run;
