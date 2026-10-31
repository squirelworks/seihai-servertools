# Architecture

How `seihai-servertools` is put together and where new work goes. Read
`AGENTS.md` (the rules) first, then this, then `docs/NEXT.md`.

## Crates

```
crates/seihai-servertools   the binary: command line, `gui` opens the window
crates/app                  servertools-app: the egui window, one page per tool
   │            │
crates/setup-core           lab start, process runner, serveroptions.txt,
   │                        listserver TOML, profiles, safety checks, network
crates/lat-core             LAT: remote file link, levels, ganis, NPCs, world check
   │
seihai-core                 squirelworks/seihai, pinned tag: level/gmap/gani
                            parsers, GS1, protocol, framing
(later) shared drawing      squirelworks/seihai, pinned tag: tile and sprite
   + level document         drawing and the level editor's document
(later) seihai-rc           squirelworks/seihai-rc, pinned tag: RC session,
                            file browser, NC login
```

Rules for the layers:

- `setup-core` and `lat-core` have no UI and no printing. They return values
  and errors; tests call them directly.
- `seihai-servertools` (command line) is the only place that prints, reads
  the environment or sets exit codes.
- `app` holds UI state only. Long work (starting servers, a live RC link)
  runs on a worker thread and talks to the page through channels, so the
  window never blocks.
- The window is the `gui` feature of the binary (on by default).
  `--no-default-features` builds a command-line-only binary for headless
  servers, and the core crates never depend on egui.

## Code shared with Seihai

The rule: one copy of every parser and every drawing routine, in the
`seihai` repo, used here through a pinned git tag (`[workspace.dependencies]`
in `Cargo.toml`).

| What | Where today | Used here through |
|---|---|---|
| Level (`.nw`, `.graal`), gmap, gani, object set, tile types parsers | `seihai-core` | `seihai-core` tag `v0.0.3` |
| GS1 parser and engine, protocol framing, version profiles | `seihai-core` | `seihai-core` (when the NPC editor and RC link need them) |
| Tile and sprite drawing | `seihai-mae` (`seihai::render`), the client binary crate | **not yet**: needs a shared crate |
| Level editor document (line-preserving edit and save, objects, NPC pane) | `tools/level-editor` (a binary tool) | **not yet**: needs a shared crate |
| RC session, file browser, NC login | `seihai-rc` (stage 1 done; file browser is stage 4) | **not yet**: `seihai-rc` library by tag |

The two "needs a shared crate" rows are the first step of the LAT track
(`docs/NEXT.md`): in the `seihai` repo, move the drawing out of `seihai-mae`
and the `Document` out of `tools/level-editor` into a library crate (for
example `crates/seihai-edit`, depending on `seihai-core` and `egui` only),
keep the client and its offline editor using it, tag, and pin the tag here.
Do not depend on `seihai-mae` itself: it pulls in the game window and sound.

Bumping a tag is its own PR here, with `cargo test` and the lab checks.

## setup-core

| Module | What | State |
|---|---|---|
| `lab` | Lab world layout, finding `gs2emu` and `seihai-listserver`, the start `Plan` | working |
| `run` | `ProcessSpec`, `start`, `Group` (wait, stop together: SIGTERM then kill), `tail` | working |
| `config::options_txt` | `serveroptions.txt`, every line kept; `get`, `set` (in place, change comment), `diff`, `render` | working |
| `config::backup` | `save` (dated backup, temp + rename), `backups`, `undo` (undoable) | working |
| `stamp` | UTC `Stamp` for backup names and change comments, no dependency | working |
| `config::listserver_toml` | `ListserverLab::scan` (bind, gserver port, accounts, gserver_ip) | scan; editor is step 6 |
| `profile` | `Profile`, key tables, `detect`, `back_to_lab` (localhost last), `apply_public` (`PublicAnswers`), `staff_with` | working |
| `checks` | `check_gserver(options, target)` (refused combinations + safety check), `check_server_folder` (lab accounts, default account), `check_listserver`, `may_save` | working |
| `net` | Interfaces, test connection, UPnP opt-in | contract only (step 4/5) |

### The one-step lab start

`lab::plan(&LabOptions)` looks at the world folder and returns a `Plan`
(listserver spec, one spec per gserver, notes) without starting anything.
The command line prints it (`lab plan`) or starts it (`lab start`):

1. Start the listserver; wait until `127.0.0.1:<gserver_port>` accepts (5 s).
   The wait opens and closes one TCP link, which the listserver logs as a
   gserver that sent no `REGISTERV3`; that line is expected.
2. Start each gserver (`gs2emu --server <name>`, working folder = world).
3. After 2 s, any process that already stopped is reported with the end of
   its console, and the rest are stopped.
4. Wait for Ctrl+C (the `ctrlc` crate), then `Group::stop_all`: SIGTERM on
   Unix, a hard stop on Windows, then a kill after 5 s.

When the Rust `seihai-gserver` can run a world, `LabOptions` gets a gserver
kind (fork or Rust) and `plan` builds its arguments; nothing else changes.
The window's Lab page will run the same `Plan` on a worker thread.

## lat-core

| Module | What | State |
|---|---|---|
| `remote` | `RemoteFiles` trait (list, get, put), `LocalFolder`, `safe_relative` | working for local folders |
| `level` | `open` via `seihai_core::assets::Level` | open only |
| `gani` | `open` via `seihai_core::assets::Gani` | open only |
| `npc` | Level NPCs and NPC-server NPCs | contract only |
| `check` | `check_world`: parse errors, missing link targets, gmap holes | first checks |

Every editor reads and writes through `RemoteFiles`, so the same editor works
on a local folder (tests, owner on the same machine) and a live server
(`RcFiles`, once `seihai-rc` has the file browser).

## app (the window)

`App` has a side bar of `Tab`s and one module per page in `src/pages/`.
Today: Lab (shows the plan), World check (runs the checker). Server setup,
Level editor, NPC editor and Gani editor show what they will do. Each page
calls the core crates; a page never parses a file itself.

## Command line

`src/main.rs` dispatches; `src/lab.rs` is `lab start|plan`; `src/setup.rs` is
the setup wizard as prompts (`status`, `public`, `lab`, `undo`). Prompts go
through a small `Io` (input, output, `--yes`) so tests drive the wizard by
piping answers to the binary (`tests/cli.rs`). The parser is hand-written in `crates/seihai-servertools/src/main.rs`, like the other
stack repos. When `setup` grows its subcommands, switching to `clap`
(MIT/Apache) is fine; keep the documented option names.

Passwords (RC logins for the LAT tools, listserver accounts) only ever come
from an environment variable or a prompt, never from the command line.

## Testing

- Unit tests next to the code; integration tests in each crate's `tests/`.
- `setup-core/tests/run_group.rs` uses the system shell (`sh`, `cmd`) as a
  stand-in server so process handling is tested on all three OSes.
- `seihai-servertools/tests/cli.rs` runs the binary.
- Loopback lab runs (real `seihai-listserver`, the fork's `gs2emu` or a
  stand-in) are written up in `docs/lab/`. Only `127.0.0.1`.

## CI and releases

`.github/workflows/ci.yml`: fmt, clippy (`-D warnings`), test on Linux,
Windows and macOS, plus a command-line-only build. `.github/workflows/release.yml`:
on a `v*` tag, four targets (Linux x86_64, Windows x86_64, macOS x86_64 and
aarch64), each archive with the binary, `README.md`, the licences and
`scripts/lab/`. No tag without Squirrel's go-ahead.
