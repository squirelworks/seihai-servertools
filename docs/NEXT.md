# Next steps

Work in this order, one branch and pull request each. Tick the box in the
same PR. Rules: `AGENTS.md`. Layout: `docs/ARCHITECTURE.md`.

## Handed to the team (Squirrel, 2026-10-31)

Squirrel left these for the other agents (Cursor, Grok, Chiaki); the Claude
session that built the setup did not do them:

- [x] **Real-fork lab run** of the whole setup (2026-11-12,
      `docs/lab/2026-11-12-real-fork-lab-run.md`): build `seihai-gserver-v2`,
      run it with `seihai-listserver` through `lab start`, take `default`
      public with `setup public` (a dummy-interface address as the outside
      interface; 127/8 is refused for a public server), check `setup start` shows the gserver listening and linked, and
      that a Seihai client gets it in the server list. Steps: "Step 4 lab
      run" below and `docs/lab/HOWTO.md`. So far the gserver side was only a
      stand-in script (`docs/lab/2026-10-31-lab-start.md`,
      `docs/lab/2026-10-31-host-listserver.md`). This one run also covers
      "Step 4 lab run" (Setup track) and "Lab run with a real `gs2emu`"
      (Lab track) below.
- [x] **Real 1.41 rev 1 gets the list** (2026-11-12, same run note) from a listserver written by Host a
      listserver (`silent_list_after_ms = 500`). Left over from issue #18,
      whose setup and capacity parts are done (#21, #41); so far only
      Seihai's 1.41 profile was run (`docs/lab/2026-11-04-suite-stress.md`).

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
      pinned to seihai main `5195744` by rev in #29, now on the `v0.0.6`
      tag. A stress rerun should lose the ~40 ms
      file-probe tail the bots added.
- [ ] The 1000-player mixed-version run, once the suite parts are further
      along (Squirrel, 2026-11-04: a later side project).

## Resource packages (`lat-core::gpak`, `docs/design/gpak.md`)

- [x] `gpak pack|unpack`: plain `.gpak`/`.zip` and encrypted `.gpake` with
      the client's bit-reversed DES; the key goes into a gserver folder's
      `config/gpakkeys.txt`, which seihai-gserver-v2 reads.
- [x] Issue #49: a Packages page in the window (`gui --page packages`),
      through the same `lat_core::gpak::pack` as the command line.
- [x] Issue #49: `check` warns about a `.gpake` with no key in
      `config/gpakkeys.txt`, a key file in a served folder (`world/`,
      `files/`), and a plain `.gpak` when `allowedversions.txt` may let in
      4.20r8. Tests only; no lab run of the page yet.

## Offline LAT chain (`lat-core::{worldmap,render}`)

The classic converters, map makers, viewers and generators in Rust. Tool
coverage and ranking: Graal_analysis `tools/tools-coverage-2026-11-14.md`.

- [x] `level convert IN OUT.nw`: `.graal` and `.zelda` to `GLEVNW01`
      (TextConvert, GRAALtoNWconverter, nw-converter).
- [x] `level render` and `gmap render`: the board as PNG, a gmap in one
      picture with `--scale` (NW2PNG, graalmapview, the level viewers).
- [x] `gmap make`: a grid from a list, from generator-style names, or by
      walking a linked world's edge links (Graal Map Maker, MapGenerator).
- [x] `level convert IN OUT.graal|OUT.zelda`: GR-V1.03 and Z3-V1.04 out,
      through seihai-core 0.2.3's binary writer; what a format cannot hold
      is printed as a warning.
- [x] Generators (`gen terrain|image|dungeon`, `lat-core::generate`), from
      the behaviour notes: a height field with `HEIGHTS` blocks, levels
      from a picture in the twelve GraalLevelGen colours, a dungeon from an
      `X`/`.` layout or a seeded maze. Not byte-for-byte the originals.
- [ ] Dungeon styles beyond the maze (onion, spiral, rogue, snake), NPC
      and `TILES` blocks from `dungeonoptions.txt`, and GraalLevelGen's
      water, mountain and sand edge pieces (not described yet).
- [x] `gif2mng [-b] GIF...` (`lat-core::anim`): one frame to PNG, more to
      MNG, through seihai-core 0.2.5's frame reader and MNG writer.
- [x] `level fill` and `theme make` (`lat-core::levelfill`, #77):
      LevelFiller's `.nwt` themes, quantities and placement on the
      layer-0 board with every other line kept, and LFThemeMaker's `.tm`
      patch themes.
- [x] LevelFiller complex objects: the object language in
      `lat-core::lfscript` (#77).
- [x] LevelFiller border objects and filling across neighbouring levels
      (`level fill --across GMAP`). The border rule is ours (LevelFiller's
      border pass was never traced): one roll per outer edge cell of a
      rule's patches, the border placed just outside it where it fits.
- [ ] Compare the border pass with LevelFiller v0.7 in the lab, if a
      copy runs, and adjust the rule.
- [ ] gif2mng `-b`'s seven body colour slots (RGB not described yet; `-b`
      only makes index 2 transparent), and Assimilator's batch modes.
- [x] `render --npcs`: level NPC images (PNG, GIF, first MNG frame) on
      the board, found by name in any letter case.
- [ ] Draw signs, chests and NPCs shown as a gani in `render`.
- [x] `stats ACCOUNTS OUTDIR [MODE...]` (`setup-core::stats`): leaderboard
      pages from seihai-gserver accounts, gserverstat's modes that the
      accounts can fill (no bomb power or online time yet); no passwords,
      no account names, load-only accounts skipped.

## Client caches (`lat-core::code_cache`, `docs/design/code-caches.md`)

- [x] Issue #37 `code show|crc|dump|pack` for `GWEBL001` level caches and
      `GRESCR01` script caches, reusing `gpak`'s DES (#39). Round-trip
      tests on synthetic files only.
- [x] Issue #37 tool audit: explicit stored/decrypted CRC domains, full GCHAR
      level-header lengths and independent known-byte regression fixtures
      (#43). See `docs/design/code-caches.md`; no native-cache acceptance claim.
- [x] Issue #37 original evidence: real 6.113 GRESCR01 weapon caches decrypted
      with keys read from packet 197; the 197 CRC is the stored-body CRC and
      the body is GS2 bytecode. `code crc` now defaults to `stored`.
- [ ] Class caches and packet 140 on a real client (not seen yet).

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
- [x] **Step 4 lab run** (2026-11-12, `docs/lab/2026-11-12-real-fork-lab-run.md`): `seihai-listserver` on
      `127.0.0.1`, the fork's `default` folder taken public with
      `setup public --interface 192.0.2.2 --listserver 127.0.0.1` (a dummy
      interface in a network namespace) on Linux,
      `setup start` shows listening on 192.0.2.2 and the listserver linked,
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
- [x] Issue #18 remaining player-capacity control (#41): explicit `setup capacity`
      for the fork's `maxplayers` and Rust's `max_players`, with preview,
      confirmation, comment-preserving save, backup and undo. The listserver's
      `max_connections` stays in its existing TOML host wizard. Model/CLI
      regression tests cover each format; no real-client or scale claim.

## Lab track

- [x] Lab page in the window: start and stop the plan on a worker thread
      (`setup_core::lab_run::LabRun`), show each console's tail. Lab run:
      `docs/lab/2026-11-04-lab-page.md`.
- [x] Issue #48: Rust gserver as a `lab start` kind (`--rust DIR`,
      `--rust-gserver PATH`, the Lab page's optional field). Its
      `config/server.txt` is read, never edited; a `listserver` line that
      misses the lab listserver gets a note naming the change. Run:
      `docs/lab/2026-11-12-lab-start-rust.md`.
- [ ] The same with a listserver and Seihai `--list-only` seeing the Rust
      world (needs the lab machine; this run was gserver only).
- [x] Lab run with a real `gs2emu` from a fork release (2026-11-12,
      `v4.0.0-seihai.1`, `docs/lab/2026-11-12-real-fork-lab-run.md`; the earlier run used a stand-in,
      `docs/lab/2026-10-31-lab-start.md`).
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

- [x] **Shared drawing** is `seihai-draw`, pinned here at seihai tag
      `v0.0.6`, with `seihai-core` on that same tag. The level *document* (line-preserving save) is still only in
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

- [x] Issue #38 image-reference slice: `check --client-reach` inventories PNG-backed
      level NPC images and fixed GS1 image references without GIF counterparts.
      Merged in #40; tests and three-platform CI passed.
- [x] Issue #38 gani inventory slice: parsed gani files list the shared
      profiles without `ui.ganis` for conditional fallback review. Presence
      does not prove use; no nearest built-in animation or runtime parity
      is inferred. Synthetic regression coverage only.
- [x] Issue #38 gmap and level-format slice: `--client-reach` lists, per
      parsed gmap, the shared profiles without `levels.gmap` (linked-level
      fallback review), and per level the profiles that cannot open its
      format (read from the magic: NW, GR-V1, Z3-V1). Inventory only.
- [x] Issue #38 opt-in GIF copies: `check --make-gif` writes a GIF next
      to each PNG `--client-reach` reports, pixel for pixel, never replacing
      a file; art over 256 colours is skipped with the reason. Not drawn by
      a real pre-2.1 client yet.
- [x] Issue #38 script-era slice: `--client-reach` lists level NPC and
      weapon scripts that are GS2 only or mix GS1 and GS2, with the shared
      profiles without GS2 (3.153 and later run it, per seihai #430). Read
      from the script text, not compiled; the client lists come from
      seihai-core's `VersionProfile::scripts.gs2` (#60).
- [x] Issue #38 tile slice: levels with tiles past column 116 of
      `pics1.png`, listed with the profiles that ship the 1856-wide sheet
      (1.28b2 to 1.41, 2.00 to 2.05). 1.21 and 2.1x left out (no evidence).
- [x] Issue #38 weapon images: a weapon file's `IMAGE` and fixed script
      image names join the GIF inventory and `--make-gif`.
- [x] Issue #38 run-time image names: script image names with `#` codes
      are matched as patterns against the world's PNG and GIF names; the
      PNGs they can be join the GIF inventory and `--make-gif`, and names
      that match nothing or say too little are listed to check by hand.
      This is not a fidelity certificate.

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
