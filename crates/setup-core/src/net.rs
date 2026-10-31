//! Network helpers for the setup wizard. **Planned** (`docs/NEXT.md`).
//!
//! - `interfaces()`: this machine's addresses, for the "Listen on" list.
//! - `test_connection(addr, timeout)`: one TCP connect to the address the
//!   owner typed, reporting open / refused / timed out. It never connects
//!   anywhere else, and nothing in this crate makes a call the owner did not
//!   ask for (no telemetry, no update checks).
//! - UPnP: opt-in only, off by default, explained in the wizard. Never needs
//!   admin rights.
//!
//! Tests only touch `127.0.0.1` (and `127.0.0.2` on Linux as the stand-in
//! "outside" address).
