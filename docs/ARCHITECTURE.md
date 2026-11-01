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
seihai-core                 squirelworks/seihai, pinned commit: level/gmap/gani
                            parsers, GS1, protocol, framing
seihai-draw                 squirelworks/seihai, same commit: tile and gani
                            RGBA. The level document is still the client's
                            offline editor, not a library
seihai-rc                   squirelworks/seihai-rc, pinned commit: RC session
                            and the file link (list, get, put)
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
`seihai` repo, used here through a pinned git commit
(`[workspace.dependencies]` in `Cargo.toml`). A tag is preferred when one
contains the crate. `seihai-draw` is on seihai `main` and not in a tag yet
(`v0.0.5` is older), so `seihai-core` and `seihai-draw` are both pinned to
that merge (`72bb8d2`). `seihai-rc`'s file link is on its `main` (`294f2c1`)
and has no tag, so it is pinned to that commit too. Do not follow a branch.

| What | Where today | Used here through |
|---|---|---|
| Level (`.nw`, `.graal`), gmap, gani, object set, tile types parsers | `seihai-core` | `seihai-core` commit `72bb8d2` |
| GS1 parser and engine, protocol framing, version profiles | `seihai-core` | `seihai-core` (when the NPC editor needs them) |
| Tile and one gani frame, into an RGBA buffer | `seihai-draw` | `seihai-draw` commit `72bb8d2` |
| Level editor document (line-preserving edit and save, objects, NPC pane) | `tools/level-editor` (a binary tool) | **not yet**: the LAT editor writes `GLEVNW01` text from the parsed level |
| RC session and file link (list, get, put) | `seihai-rc` | `seihai-rc` commit `294f2c1` |

Do not depend on `seihai-mae`: it pulls in the game window and sound. The
client's offline level editor stays in the Seihai repo.

Bumping a tag is its own PR here, with `cargo test` and the lab checks.

## setup-core

| Module | What | State |
|---|---|---|
| `folder` | `ListserverConfig` (the listserver's TOML: preview host / lab, findings incl. a missing accounts file, save, undo) and `ServerFolder`: open, `profile`, `status_findings`, `preview_public` / `preview_lab` (`Preview` with diff and findings), `save` (refuses blocked), `undo`. The command line and the window both use it | working |
| `lab` | Lab world layout, finding `gs2emu` and `seihai-listserver`, the start `Plan` | working |
| `run` | `ProcessSpec`, `start`, `Group` (wait, stop together: SIGTERM then kill), `tail` | working |
| `config::options_txt` | `serveroptions.txt`, every line kept; `get`, `set` (in place, change comment), `diff`, `render` | working |
| `config::backup` | `save` (dated backup, temp + rename), `backups`, `undo` (undoable) | working |
| `stamp` | UTC `Stamp` for backup names and change comments, no dependency | working |
| `config::listserver_toml` | `ListserverLab::scan` for the lab start; `ListserverToml` editor (toml_edit), `HostAnswers`, `apply_host`, `listserver_back_to_lab` | working |
| `profile` | `Profile`, key tables, `detect`, `back_to_lab` (localhost last), `apply_public` (`PublicAnswers`), `staff_with` | working |
| `checks` | `check_gserver(options, target)` (refused combinations + safety check), `check_server_folder` (lab accounts, default account), `check_listserver`, `may_save` | working |
| `lights` | `Lights::read(console, public)`: listening on, listserver, reachable (always unknown) | working |
| `net` | `test_connection` (one TCP connect to a typed address); interfaces and UPnP planned | partly |

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
| `remote` | `RemoteFiles` (list, get, put), `LocalFolder`, `RcFiles` over `seihai-rc`, `safe_relative` | working |
| `level` | `LevelDoc`: open, `set_tile`, `encode` (`GLEVNW01` text), `paint` via `seihai-draw` | editing |
| `gani` | `GaniDoc`: open, `set_sprite_frame`, `encode`, `paint` via `seihai-draw` | editing |
| `preview` | RGBA buffer, stand-in tile colours and a stand-in sprite picture | working |
| `npc` | Level NPCs and NPC-server NPCs | contract only |
| `check` | `check_world`: parse errors, missing link targets, gmap holes, missing images and sounds | working |

Every editor reads and writes through `RemoteFiles`, so the same editor works
on a local folder and on a live server (`RcFiles`). The directory reply is
names only, so a listing does not say which names are folders.

## app (the window)

`App` has a side bar of `Tab`s and one module per page in `src/pages/`.
Today: Lab (shows the plan), Server setup (the Join a listserver wizard,
Back to lab, undo, Start and check), World check (runs the checker),
Level editor and Gani editor (open, edit, preview the RGBA buffer, upload).
The NPC editor still says what it will do. Each page calls the core crates;
a page never parses a file itself.

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
