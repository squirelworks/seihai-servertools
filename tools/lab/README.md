# seihai-lab

One-step lab start: starts the listserver and the gservers of a lab world (an unpacked seihai-gserver-v2 release), logs each console, and stops them all on Ctrl+C.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools lab ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-lab    # target/release/seihai-lab
cargo test -p seihai-lab
target/release/seihai-lab start --world path/to/world
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- setup-core (lab plan, process runner), ctrlc
- seihai-core: None. setup-core does not use seihai-core, so this tool builds without it.

It does not depend on any other tool in `tools/`.

## Usage

```text
lab start [--world DIR] [--listserver PATH | --no-listserver]
          [--gserver PATH] [--rust DIR]... [--rust-gserver PATH] [SERVER...]
lab plan  (same options; prints what start would run)

start: starts the listserver (if one is found) and then the gservers of a
lab world (default: servers/default, classic and modern, those that exist),
each console in a log file, and stops them all on Ctrl+C. --world defaults
to the current folder. --rust DIR also starts seihai-gserver on that world
folder (config/server.txt; relative to --world), found beside the world,
this tool or on PATH unless --rust-gserver is given. With --rust and no
SERVER named, the fork's folders start too when gs2emu is there.
```
