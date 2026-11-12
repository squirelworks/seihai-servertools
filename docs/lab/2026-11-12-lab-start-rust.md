# Lab start with a seihai-gserver world (2026-11-12)

Issue #48. Linux container, loopback only, no real clients.

## Setup

- `seihai-gserver` built from `main` (`cargo build --release`, v0.1.0).
- A world folder `w/` holding only `rust/`: a copy of seihai-gserver's sample
  `world/` with `port = 14806` (its default 14802 is `servers/default`'s).
- No fork `gs2emu`, no listserver (`--no-listserver`).

## Run

```
seihai-servertools lab plan  --world w --no-listserver --rust rust --rust-gserver <path>/seihai-gserver
rust        rust on 127.0.0.1:14806  (console w/logs/rust-console.txt)

seihai-servertools lab start --world w --no-listserver --rust rust --rust-gserver <path>/seihai-gserver
started rust, pid …
Lab world running. Stop with Ctrl+C.
```

A TCP connect to `127.0.0.1:14806` was accepted while it ran. Ctrl+C
(SIGINT) printed `Stopping...`, and the console ends with seihai-gserver's
clean shutdown line:

```
listserver link: off
Seihai Lab listening on 127.0.0.1:14806
stopping: saved 0 online player(s)
```

## Not covered

- A listserver in the same run, and Seihai `--list-only` seeing the Rust
  world: left for the lab machine (NEXT.md).
- The fork side by side with a real `gs2emu`: the plan and the worker are
  tested with stand-ins (`tests/lab_plan.rs`, `tests/lab_run.rs`).
