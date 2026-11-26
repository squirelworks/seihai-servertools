# tools/

Every seihai-servertools tool is its own crate and its own program here.
Each one builds, tests and runs on its own; `seihai-servertools` (in
`crates/seihai-servertools`) is only an all-in-one front door that hands
`seihai-servertools <tool> ...` to the same code.

| Tool | Program | What |
|---|---|---|
| [guide](guide/README.md) | `seihai-guide` | How the servers fit together, every tool, setup, RC, troubleshooting |
| [lab](lab/README.md) | `seihai-lab` | One-step lab start: listserver and gservers, stopped together |
| [setup](setup/README.md) | `seihai-setup` | A server from scratch (`setup new`), go public and back, host a listserver, player limit, HQ password |
| [check](check/README.md) | `seihai-check` | World and config checker (`--json` for CI) |
| [code](code/README.md) | `seihai-code` | Client `.code` caches: show, CRC, dump, pack |
| [gpak](gpak/README.md) | `seihai-gpak` | Resource packages `.gpak` / `.gpake` |
| [level](level/README.md) | `seihai-level` | Level convert, render, LevelFiller fill |
| [theme](theme/README.md) | `seihai-theme` | LFThemeMaker patch themes |
| [gmap](gmap/README.md) | `seihai-gmap` | Gmap make and render |
| [gani](gani/README.md) | `seihai-gani` | GraalShop movie mode: show, render |
| [gen](gen/README.md) | `seihai-gen` | World generators: terrain, 3D, picture, dungeon |
| [stats](stats/README.md) | `seihai-stats` | Leaderboard pages from accounts |
| [gif2mng](gif2mng/README.md) | `seihai-gif2mng` | GIF to PNG or MNG |
| [gui](gui/README.md) | `seihai-servertools-gui` | The window |
| [suite-stress](suite-stress/README.md) | `seihai-suite-stress` | Stress rig for the whole server suite |

## Standalone: the rules for a tool

A new tool (or a tool moved out of another) follows these, and
`scripts/ci/each-tool.sh` checks them in CI:

1. **Its own crate** in `tools/<tool>/`. Package and binary are both
   `seihai-<tool>`. `src/lib.rs` has `pub const USAGE: &str` and
   `pub fn run(args: &[&str]) -> ExitCode` (the words after the program
   name; `--help` prints `USAGE`). `src/main.rs` only answers `--version`
   and calls `run`.
2. **Its own README.md**: what it does, the build, test and run commands,
   what it depends on (with its seihai-core pin) and its `--help`.
3. **Its own tests** in `tools/<tool>/tests/`, running its own program
   (`env!("CARGO_BIN_EXE_seihai-<tool>")`), at least `--version` and
   `--help`. A flow that needs two tools is tested in
   `crates/seihai-servertools/tests/workflows.rs` instead.
4. **No dependency on another tool.** Logic, formats and rules live in
   `crates/lat-core` or `crates/setup-core` (AGENTS.md: logic in the core
   crates); code two tools need goes there, never into one tool for the
   other to use.
5. **seihai-core pinned once.** A tool that uses seihai-core directly
   writes `seihai-core.workspace = true`; the one pin is in the root
   `Cargo.toml` under `[workspace.dependencies]`, a tag of
   `squirelworks/seihai-core`, never a branch. One pin keeps one copy of
   the parsers in the tree, so a `Level` from seihai-core is the same type
   in every crate.
6. **One line in the front door.** Add `seihai-<tool>` to the root
   `[workspace.dependencies]`, to `crates/seihai-servertools/Cargo.toml`
   and to `dispatch` in its `main.rs`, so `seihai-servertools <tool>` works
   too (a rig such as suite-stress can stay out of it).

Build or test one tool alone with `cargo build -p seihai-<tool>` /
`cargo test -p seihai-<tool>`; with resolver 2 that builds only the tool and
the crates it names, with only the features it asks for. CI runs that for
every tool (`scripts/ci/each-tool.sh`), next to the whole-workspace build.
