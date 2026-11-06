# Next steps

Work in this order, one branch and pull request each. Tick the box in the
same PR. Rules: `AGENTS.md`. Layout: `docs/ARCHITECTURE.md`.

## Handed to the team (Squirrel, 2026-10-31)

Squirrel left these for the other agents (Cursor, Grok, Chiaki); the Claude
session that built the setup did not do them:

- [ ] **Real-fork lab run** of the whole setup: build `seihai-gserver-v2`,
      run it with `seihai-listserver` through `lab start`, take `default`
      public with `setup public` (127.0.0.2 as the outside interface on
      Linux), check `setup start` shows the gserver listening and linked, and
      that a Seihai client gets it in the server list. Steps: "Step 4 lab
      run" below and `docs/lab/HOWTO.md`. So far the gserver side was only a
      stand-in script (`docs/lab/2026-10-31-lab-start.md`,
      `docs/lab/2026-10-31-host-listserver.md`).

## Suite stress (`crates/suite-stress`, `docs/lab/suite-stress.md`)

- [x] The rig: listserver + gserver + NPC-server + RC on loopback, bots for
      every version profile, a report per run. First runs:
      `docs/lab/2026-11-04-suite-stress.md`.
- [ ] Rerun `compat` and a 92-bot `run` as each gserver stage lands (2.2
      levels, 2.3 other players, 2.4 warps, 2.5 gmaps, 2.7 files), and add a
      bot behaviour for each (warp, cross a gmap edge, fetch a real file).
- [ ] RC against the gserver once it serves RC logins (stage 4).
- [ ] Real 2.x clients under Wine next to the bots (`--base-port 14800`).
- [x] Other client programs next to the bots (`--external`); first used
      with graal-delphi's headless 2.05r2.
- [x] Bump seihai-core once its client sets `TCP_NODELAY` (seihai #360):
      pinned to seihai main `5195744` by rev in #29; switch to the `v0.0.6`
      tag once it is published. A stress rerun should lose the ~40 ms
      file-probe tail the bots added.
- [ ] The 1000-player mixed-version run, once the suite parts are further
      along (Squirrel, 2026-11-04: a later side project).

## Resource packages (`lat-core::gpak`, `docs/design/gpak.md`)

- [x] `gpak pack|unpack`: plain `.gpak`/`.zip` and encrypted `.gpake` with
      the client's bit-reversed DES; the key goes into a gserver folder's
      `config/gpakkeys.txt`, which seihai-gserver-v2 reads.
- [ ] A Packages page in the window (same calls as the command line).
- [ ] `check`: warn about a `.gpake` in `files/` with no key, and a plain
      `.gpak` when 4.20 clients are allowed.

## Done

- [x] Scaffold: workspace, licences, `AGENTS.md`, docs, CI, release workflow.
- [x] One-step lab start: `seihai-servertools lab start|plan`, `scripts/lab/`
      (`docs/lab/2026-10-31-lab-start.md`).
- [x] World checker, first checks: `seihai-servertools check`.

## Setup track (go-public), `docs/design/go-public.md`

Squirrel asked for all three parts in the first version (2026-10-31): the
window wizard, the `setup` command line, and Host a listserver.

- [x] **Step 1** `config::options_txt` writer (`set`, `remove`, the
      `# Changed by Seihai server setup, YYYY-MM-DD` comment) and
      `config::backup` (dated backup, temp + rename, undo). Round-trip tests:
      comments, unknown keys, CRLF and order survive.
- [x] **Step 2** `profile` (detect Lab / Public, Back to lab in the
      `GSERVER_KEYS` order, `localhost = true` last) and `checks` (every
      safety-check rule and refused combination, one test each).
- [x] **Step 3** `seihai-servertools setup --public | --lab` on the command
      line: the wizard's steps as prompts, `--yes` plus options for scripts,
      a before/after diff before saving.
- [x] **Step 4** Start and check: `setup start` runs the server with `run`,
      reads its console and shows the three lights (`setup_core::lights`;
      "reachable from outside" always says it cannot be proven from here);
      `setup test-connection HOST[:PORT]` (`setup_core::net`). Tested with a
      stand-in gserver that prints the fork's console lines.
- [ ] **Step 4 lab run** (needs a built fork; handed to the team, see the top): `seihai-listserver` on
      `127.0.0.1`, the fork's `default` folder taken public with
      `setup public --interface 127.0.0.2 --listserver 127.0.0.1` on Linux,
      `setup start` shows listening on 127.0.0.2 and the listserver linked,
      a Seihai client gets the server in the list. Write it up in
      `docs/lab/`. If the fork prints its lines differently, fix
      `lights.rs` and its table.
- [x] **Step 5** Window: the Server setup page (`crates/app/src/pages/setup.rs`):
      the seven steps on one page, Test connection on a worker thread,
      Review Join a listserver / Back to lab with the diff and findings,
      Save only when nothing blocks, Undo, Start and check with the lights.
      All through `setup_core::folder::ServerFolder`, which the command line
      uses too. `seihai-servertools gui --world DIR --page setup` opens it.
- [x] Window polish, part 1: "Listen on" pick lists of this machine's
      addresses (`setup_core::net::interfaces`, `if-addrs`) on Server setup
      and Host a listserver, the same list before the command line's Listen
      on questions, and state-only tests of `Form::answers` and
      `Host::answers`. Lab run: `docs/lab/2026-11-04-listen-on-list.md`.
- [x] Window polish, part 2: a Browse... folder picker next to every world
      folder field (Lab world, Server setup, World check, the editors'
      folder). An in-window picker over `setup_core::browse`, not a native
      dialog: no extra crate, same on every system, world folders marked.
      Lab run: `docs/lab/2026-11-04-folder-picker.md`.
- [x] **Step 6** Host a listserver: `config::listserver_toml::ListserverToml`
      (on `toml_edit`: comments, order and unknown keys kept),
      `folder::ListserverConfig`, `setup host-listserver | listserver-lab |
      listserver-undo` on the command line, the Host a listserver half of the
      Server setup page, and the first account through
      `seihai-listserver add-account` (password only in the child's
      environment). Lab run: `docs/lab/2026-10-31-host-listserver.md`.
- [x] "Who may register": the `gservers` allow-list from
      squirelworks/seihai-listserver#6 (`ListserverToml::set_gservers`,
      `--allow-gserver`, `--hq-password-env`, `--any-gserver`, the rows on
      the window page). The warning stays only while the list is empty.
- [x] Host a listserver sets `silent_list_after_ms` (500 by default; real
      1.41 rev 1 waits for the list) and `max_connections`
      (`docs/lab/2026-11-04-suite-stress.md`, findings 2 and 3). `lab plan`
      and the review note a listserver left at 0. The gserver side's player
      cap is the gserver's own setting.
- [x] Join a listserver: write the gserver's own `hq_password` (in
      `config/adminconfig.txt`, dated backup) so it matches the listserver
      row: `setup hq-password`, `setup public --hq-password-env`, and the
      HQ password field on the Server setup page.

## Lab track

- [x] Lab page in the window: start and stop the plan on a worker thread
      (`setup_core::lab_run::LabRun`), show each console's tail. Lab run:
      `docs/lab/2026-11-04-lab-page.md`.
- [ ] Rust gserver: a gserver kind in `LabOptions` once `seihai-gserver` can
      run a world folder.
- [ ] Lab run with a real `gs2emu` from a fork release (today's run used a
      stand-in; `docs/lab/2026-10-31-lab-start.md`).
- [ ] **Graal3D lab kind** (after Seihai plannedmilestones M3 / G3D-9): start
      and stop the Rust Graal3D host on loopback with a `levels/` folder of
      `.x` files; document the recipe next to the 2D lab. Not the fork and
      not `seihai-gserver`. No listserver registration (the 2000 protocol
      has none).

## Checker track (Graal3D)

Held until Seihai M4 or M5 asks for it:

- [ ] **`.x` world check**: every texture file named by a level mesh (and
      the default player mesh/texture set) exists beside the world; refuse
      binary/compressed `.x`. Separate from the 2D missing-image checker.

## LAT track, `docs/design/lat-tools.md`

- [x] **Shared drawing** is `seihai-draw` on seihai `main` (not in a tag
      yet). Pinned here at `72bb8d2`, with `seihai-core` on that same
      commit. The level *document* (line-preserving save) is still only in
      the client's offline editor; this editor writes `GLEVNW01` text from
      the parsed level.
- [x] Level editor on a local folder (`LocalFolder`) and through `RcFiles`:
      open, change one tile, encode, paint.
- [x] `RcFiles`: `RemoteFiles` over `seihai-rc`'s `list_dir` / `get_file` /
      `put_file` (seihai-rc `294f2c1`, no tag yet). Fake file server on
      `127.0.0.1` in `crates/lat-core/tests/rc_edit.rs`. A lab run against
      the fork with a LAT account is still open.
- [x] Level editor: show a diff against the downloaded bytes, and refuse
      the upload when the server's copy changed in between. The gani
      editor does the same. Lab run: `docs/lab/2026-11-04-lat-refuse-upload.md`.
- [x] NPC editor, level NPCs: pick an NPC, edit, Check, Apply, Show changes,
      Upload (`gui --page npcs`). Lab run: `docs/lab/2026-11-04-npc-editor.md`.
- [ ] NPC editor, NPC-server objects (database NPCs, classes, weapons)
      after `seihai-rc` stage 5.
- [x] Gani editor with preview: open, change one sprite frame, encode,
      paint one frame.

## Checker track

- [x] Missing images and sounds named by levels, NPCs and ganis.
- [x] GS1 syntax errors in level NPCs (and the images and sounds those
      scripts name, such as `showimg` and `play`): `lat_core::npc::check_script`,
      warnings in `check`. Lab run: `docs/lab/2026-11-04-npc-script-check.md`.
- [x] `serveroptions.txt` checks from `setup_core::checks`:
      `setup_core::folder::check_world_config` runs them on every server
      folder (accounts of public folders too) and on
      `seihai-listserver.toml`, in `check` and on the Check page.
- [x] `check --json`: one object (`format` 1, counts, findings with level,
      source, file, key, text) for CI.
- (Graal3D `.x` texture check: see "Checker track (Graal3D)" under Lab
  above; do not fold it into the 2D checker.)
