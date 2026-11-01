# seihai-servertools map (for recovered Drive tools)

Date: 2026-10-31. Search covered `/workspace`, `/home/ubuntu/seihai-work`, GitHub `squirelworks/*`, and Drive `[Grok/[Projects`.

## Exact location

| | |
|---|---|
| **Local path (cloned)** | `/workspace/seihai-servertools` |
| **Clone URL** | `https://github.com/squirelworks/seihai-servertools` |
| **Default branch** | `main` (HEAD `35b40c3`, setup track through Host a listserver) |
| **Licence** | `MIT OR Apache-2.0` (must not take GPL code from GServer-v2 / gs2lib / gs2-parser) |
| **Was present before this run?** | No. Not under the client tree, not in `seihai-work`. Cloned here for placement work. |

**Important:** the clone sits inside the `squirelworks/seihai` working tree as an **untracked nested repo**. Do not commit it into the client. Prefer treating it as a sibling (e.g. `/home/ubuntu/seihai-work/seihai-servertools`) for long-lived work; `/workspace/seihai-servertools` is fine for this recovery pass.

### Sibling stack (not this repo)

| Repo | Role | Local if present |
|---|---|---|
| `squirelworks/seihai` | Client + offline level editor | `/workspace` (= `/home/ubuntu/seihai-work/seihai`) |
| `squirelworks/seihai-gserver-v2` | GPL fork of GServer-v2 (ships first) | `/home/ubuntu/seihai-work/seihai-gserver-v2` |
| `squirelworks/seihai-gserver` | Rust gserver (scaffold) | not cloned here |
| `squirelworks/seihai-listserver` | Rust listserver | not cloned here |
| `squirelworks/seihai-rc` | RC / NC staff client | not cloned here |
| `squirelworks/seihai-npcserver` | NPC server | not cloned here |
| `squirelworks/Graal_analysis` | Protocol / tool notes (incl. classic tool chain) | `/home/ubuntu/seihai-work/Graal_analysis` |

Client `tools/` (`/workspace/tools`) is **client-side** helpers (captures, asset generators, offline `level-editor`). Do **not** put live-server LAT tools there.

---

## Repo structure

```
seihai-servertools/
├── Cargo.toml                 workspace; pins seihai-core by git tag (v0.0.3)
├── AGENTS.md                  stack-wide rules (licence split, loopback-only, no real hosts)
├── README.md                  build/run, command table
├── crates/
│   ├── setup-core/            lab start, process group, serveroptions/listserver config,
│   │                          profiles, safety checks, lights, net (NO UI)
│   ├── lat-core/              LAT logic: RemoteFiles, level/gani open, npc contract,
│   │                          world check (NO UI)
│   ├── app/                   servertools-app: egui window, one module per page
│   └── seihai-servertools/    binary CLI; only place that prints / reads env / exit codes
├── scripts/lab/               start.sh / start.command / start.bat (moved from the fork)
├── docs/
│   ├── ARCHITECTURE.md        where new work goes
│   ├── PLAN.md / NEXT.md      tracks and ordered TODOs
│   ├── design/go-public.md    setup wizard design
│   ├── design/lat-tools.md    LAT editors + checker design
│   └── lab/                   loopback lab write-ups + HOWTO
└── .github/workflows/         ci.yml (fmt/clippy/test ×3 OS), release.yml on v* tags
```

Binary name: `seihai-servertools`. Window via `gui` feature (default on); `--no-default-features` for headless CLI-only.

### Commands today

| Command | State |
|---|---|
| `lab start\|plan` | working |
| `setup status\|public\|lab\|undo` | CLI working |
| `setup host-listserver\|listserver-lab\|listserver-undo` | working |
| `setup start` / lights / `test-connection` | working (stand-in; real-fork lab run still open) |
| `check WORLD` | first checks (parse / missing links / gmap holes) |
| `gui [--page lab\|setup\|levels\|npcs\|ganis\|check]` | pages laid out; Levels/NPCs/Ganis planned stubs |

Build: `cargo build --release` · `cargo test` · CI requires `fmt`, `clippy -D warnings`, tests on Linux/Windows/macOS.

---

## Where salvaged Drive tools should go

Map recovered material by **job**, not by original zip name. Rewrite clean-room into this repo; never paste GServer-v2 / gs2lib / decompiled Delphi into MIT crates (`AGENTS.md`).

### 1. Lab start / process / world layout → `crates/setup-core/`

| Module | Put here when salvaging… |
|---|---|
| `lab.rs` | Finding `gs2emu` / `seihai-listserver`, start `Plan`, world folder layout |
| `run.rs` | Process group, console logs, stop-together |
| `config/options_txt.rs` | Line-preserving `serveroptions.txt` editors |
| `config/listserver_toml.rs` | `seihai-listserver.toml` editors (`toml_edit`) |
| `config/backup.rs` | Dated backup + temp/rename + undo |
| `profile.rs` / `checks.rs` | Lab vs public key tables, refused combinations |
| `lights.rs` / `net.rs` | Console “listening / linked” lights; owner-typed test connect |
| `folder.rs` | Facade used by CLI and GUI |

CLI wiring: `crates/seihai-servertools/src/lab.rs`, `src/setup.rs`.  
GUI: `crates/app/src/pages/lab.rs`, `setup.rs`, `host.rs`.  
Shell helpers that owners drop next to a world: `scripts/lab/` (keep `*.bat` CRLF, `*.sh`/`*.command` LF + executable).

### 2. LAT live-world editors / checker → `crates/lat-core/`

| Module | Put here when salvaging… |
|---|---|
| `remote.rs` | File list/get/put trait; `LocalFolder` now; later `RcFiles` over `seihai-rc` |
| `level.rs` | Level open/edit helpers via **`seihai-core`** parsers (no second parser) |
| `gani.rs` | Same for ganis |
| `npc.rs` | Level NPC scripts + (later) NC database objects |
| `check.rs` | World/config checker (`check_world` and extensions) |

GUI pages (stubs until shared drawing lands):

- Levels → `crates/app/src/pages/` new `levels.rs` (wire into `Tab::Levels`)
- NPCs → `npcs.rs` / `Tab::Npcs`
- Ganis → `ganis.rs` / `Tab::Ganis`
- Checker UI → already `pages/check.rs`

Design: `docs/design/lat-tools.md`. Order: `docs/NEXT.md` LAT track.

### 3. Do **not** put these in seihai-servertools

| Recovered thing | Correct home |
|---|---|
| Offline level editor / Document / tile paint | `squirelworks/seihai` → `tools/level-editor` (later shared `seihai-edit`-style crate, then pin tag here) |
| Tile/sprite drawing | `seihai` (`seihai-mae` today → shared crate first) |
| Protocol / level / gani / GS1 parsers | `seihai-core` in `squirelworks/seihai`; bump pinned tag in this workspace `Cargo.toml` in its own PR |
| RC/NC session, file browser packets | `squirelworks/seihai-rc` (pin by tag when ready) |
| Gserver / listserver / NPC-server runtime | `seihai-gserver`, `seihai-listserver`, `seihai-npcserver` (or fork for GPL) |
| Ash Python gserver / Ember / GServerStory | Reference only → `Graal_analysis` / Drive archive; rewrite into Rust stack, do not vendor |
| Classic GraalEditor / GraalShop / Guildmaker / stealer ListServer | Behaviour notes in `Graal_analysis/tools/`; **do not run** old ListServer.exe (password stealer — see `server-tools-2001.md`) |
| Client capture / stress / asset generators | `/workspace/tools` in `seihai` |
| Decompile dumps, binary offsets, Ghidra DBs | Stay under `recovered-drive-tools/` or private analysis; never into MIT crates |

### Recommended placement cheat-sheet (Drive → target)

| Drive / recovered class | Target path under `seihai-servertools` |
|---|---|
| Lab one-click start scripts, process wrappers | `scripts/lab/` + logic in `crates/setup-core/{lab,run}.rs` |
| `serveroptions.txt` / public-vs-lab wizards | `crates/setup-core/config/` + `profile`/`checks` + CLI `setup` + `app` Setup page |
| Listserver host/config helpers | `setup-core/config/listserver_toml.rs` + Host page |
| Optional lab status-file reader (`playercount.txt` etc.) | `crates/setup-core/` lights/status only — behaviour note first (`SALVAGE_CANDIDATES.md`) |
| World validators (missing levels, bad ganis, link holes) | `crates/lat-core/check.rs` (+ CLI `check`, Check page) |
| Live level/NPC/gani edit via staff link | `lat-core/{level,npc,gani,remote}.rs` + new app pages; RC I/O waits on `seihai-rc` |
| **RC / NC** staff session & RemoteFiles (from `RC [` / `NC [` archives) | **Not** this repo’s crates — land in `squirelworks/seihai-rc` (+ npcserver); pin tag here |
| **LAT** offline editors / converters / generators | **`seihai`** first; only live helpers into `lat-core` after drawing + RC |
| **NAT** / GS2 compilers / script editors | analysis-only notes — no Delphi/GS2 paste into MIT |
| Pure format parsers found in zips | Upstream into `seihai` `seihai-core`, then consume by tag — **not** duplicated here |
| Docs / lab run notes for a salvage | `docs/lab/YYYY-MM-DD-….md` or `docs/design/` |

Staging raw salvage before clean-room rewrite: keep under `/workspace/recovered-drive-tools/<archive-name>/` (this folder). Promote only rewritten MIT-clean code into the clone.

---

## Conventions (must follow when adding tools)

1. **Logic in cores, never in UI.** Rules/formats/checks → `setup-core` or `lat-core` with tests. CLI prints; `app` holds UI state and calls cores on worker threads for long work.
2. **One parser per format**, from `seihai-core` (`Cargo.toml` `[workspace.dependencies]`, tag pin). Missing feature → PR in `seihai`, tag, bump here separately.
3. **Config files are the owner’s.** Edit line-by-line; keep comments, order, unknown keys; temp file + rename; dated backup.
4. **No real public hostnames/IPs** in code, docs, tests, or defaults. Use `127.0.0.1`, lab ports (`14802`/`14803`, listserver `14900`, clients `14921`–`14923`), placeholders `<listserver-host>` / `<public-host>`.
5. **No outbound calls** the owner did not ask for (no telemetry, no built-in server lists). Passwords only from env or prompt, never argv.
6. **Tests:** unit next to code; crate `tests/`; CLI via `crates/seihai-servertools/tests/cli.rs`; lab write-ups only on loopback.
7. **Git:** one change per branch/PR; no push to `main` without CI; no tags/releases without Squirrel.
8. **Window toolkit:** egui/eframe **0.29** (same as Seihai client / level editor). Add a page: `crates/app/src/pages/<name>.rs` with `Page` + `show`, register in `Tab` / `App::update` (see crate docs).
9. **CLI:** hand-written dispatch in `main.rs` today (clap OK later); keep documented option names.

---

## Integration requirements for LAT salvages

Before a live level/NPC/gani editor is useful here:

1. Shared drawing + level `Document` crate in **`seihai`** (move out of `seihai-mae` / `tools/level-editor`), tag, pin in this workspace.
2. **`seihai-rc`** stage 4 file browser → implement `RemoteFiles` as `RcFiles`; stage 5 NC for database NPCs/classes/weapons.
3. Editors must work on `LocalFolder` first, then fake RC, then lab fork on `127.0.0.1` with a LAT account — never against a public server.

World checker can grow now (missing assets, GS1 syntax, `serveroptions` safety, `--json`) entirely inside `lat-core` / CLI.

---

## Related Drive archives (reference only)

Under the owner private Drive folder **Projects** (ids omitted):

| Archive | Relevance to servertools |
|---|---|
| `Graal gServers.zip` | Ash / Ember / dumps — **gserver/listserver** reference, not LAT UI; see client `documentation/server-reference.md` |
| `Graal 39sters gServer.zip` | Older protocol / museum world |
| `Graal Scripting.zip` | NPC event model → client script engine / later NPC editor checks |
| `Graal Enigma Playerworld.zip` | Levels/scripts for checker and editor fixtures |
| `Graal Client Analysis.zip` / `Client Reform.zip` | Client/editor behaviour notes → `Graal_analysis`, not this binary |

Classic tool notes already extracted: `Graal_analysis/tools/{graaleditor,graalshop,remotecontrol,npc-control,server-tools-2001}.md`.

---

## Summary for agents placing recovered tools

- **Repo:** `/workspace/seihai-servertools` ← `https://github.com/squirelworks/seihai-servertools`
- **New setup/lab tooling** → `crates/setup-core/` (+ CLI `setup`/`lab`, app Setup/Lab pages, optional `scripts/lab/`)
- **New LAT/live editors & world checks** → `crates/lat-core/` (+ CLI `check` / later editor commands, app Levels/NPCs/Ganis/Check pages)
- **Parsers, offline editor, drawing, RC wire** → sibling `seihai` / `seihai-rc` first, consume by tag
- **Raw Drive dumps** → stay in `/workspace/recovered-drive-tools/`; only clean-room rewrites land in the MIT repo

**Operational bridge (decision tree, no-raw-binaries rule, inventory/ghidra checklist):**
[`SALVAGE_PLACEMENT.md`](./SALVAGE_PLACEMENT.md). Prefer that file when assigning Placement for a new drop; keep this map as the structure reference.
