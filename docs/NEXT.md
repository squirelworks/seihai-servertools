# Roadmap status (issue #50)

Working mirror of [issue #50](https://github.com/squirelworks/seihai-servertools/issues/50)
against `main` at `e662244` (after #95). Tick boxes when the matching PR
merges. Rules: `AGENTS.md`. Layout: `docs/ARCHITECTURE.md`. Offline gap
detail: `docs/design/classic-offline-gaps.md`.

## Where it stands

- **Working:** one-step `lab start` (default, classic, modern, Rust
  gserver kind), go-public setup on the command line and in the window,
  Host a listserver with the gserver allow-list and HQ passwords, player
  capacity, the world checker (with `--client-reach`, including run-time
  image names), the LAT level / gani / level-NPC editors with upload over
  the RC file link, the Packages page, `gpak` and `code`, and the suite
  stress rig (plus listserver-hard scenarios and a buddy/guild seeder,
  `#93` / `#94`).
- **Next items from #50 are done:** real-fork lab `#47`/`#58`; Rust lab
  kind `#48`/`#54` with list evidence `#81`; Packages `#49`/`#53`;
  pre-2.1 checker `#38`; `code` / real `GRESCR01` evidence `#37`.
- **Dependencies:** `seihai-core` and `seihai-draw` are paired at tag
  `v0.4.0` (`#91`). `seihai-rc` is pinned by rev
  `90cb87f31bfb9f81490aa7e6c8c6e082ecbab75d` (`#95`, after seihai-rc#61)
  so this workspace shares one core `v0.4.0` with RC
  (squirelworks/seihai-core#79). **No `seihai-rc` tag yet**
  (seihai-rc#27).
- Core's `Era::V140` registration is vocabulary only here; tools keep
  explicit `V217` script checks and make no native 1.40 claim.

## Open checkboxes

### Dependencies

- [ ] Move `seihai-rc` to a **tag** (seihai-rc#27). Still a rev pin on
      main after `#95`; core alignment is already one `v0.4.0`.

### Lab / suite

- [ ] Original Windows clients and login to the Rust gserver after the
      `#81` list-only evidence
      (`docs/lab/2026-11-14-lab-start-rust-listserver.md`).
- [ ] Suite stress: rerun `compat` and a 92-bot `run` as each
      seihai-gserver stage lands (stages 4, 5 and anti-cheat have landed;
      stage 6 is built — seihai-gserver#56), with stage-specific evidence.
      Warp-ring, gmap-crossing and file-probe behaviours already exist;
      do not recreate them.
- [ ] RC bots now that the gserver serves RC (stage 4 closed).
- [ ] Real 2.x clients under Wine next to the bots (`--base-port 14800`).
- [ ] The 1,000-player mixed-version run (later side project).
- [ ] LAT lab run against a live server with a LAT account (RC file link
      already has a fake-server test).

### LAT / product

- [ ] NPC editor: NPC-server database objects (classes, weapons,
      attributes write/reset/add) now that seihai-gserver has NPC Control
      (seihai-gserver#52; npcserver#22 still tracks completeness).
- [ ] Class caches and packet 140 on a real client (`code` tool; weapon
      caches are done under `#37`).
- [ ] Default to the Rust gserver once it reaches parity (seihai#353
      phase 7; seihai-gserver `docs/PARITY.md`). macOS polish waits for
      the owner's polishing phase.

### Offline classic tools

Open rows live in `docs/design/classic-offline-gaps.md` (dungeon styles,
gif2mng body slots, LevelFiller border lab compare, movie CHAR extras /
gani movie page, Assimilator batch, `stats` bombpower/onlinetime). Do
not duplicate that table here.

### Graal3D (held)

- [ ] Lab kind for the February 2000 Graal3D host (G3D-9), loopback only,
      no listserver registration.
- [ ] `.x` world texture-presence check (separate from the 2D checker).

## Done record (condensed)

- [x] Scaffold, lab start, go-public / Host a listserver, world checker
      first pass.
- [x] Real-fork lab + real 1.41 silent list (`#58`,
      `docs/lab/2026-11-12-real-fork-lab-run.md`).
- [x] Rust `lab start` kind + listserver list evidence (`#54`, `#81`).
- [x] Packages page and package checks (`#53`).
- [x] Checker `#38` slices through run-time image names (`#66`).
- [x] `code` / real 6.113 `GRESCR01` evidence (`#56`); `#37` closed.
- [x] Offline LAT chain through TMX, LevelFiller, movie CHAR/TEXT paint,
      signs/chests/gani NPCs in render, PARAM1 preservation (`#73`–`#90`).
- [x] Core/draw `v0.4.0` (`#91`); classic offline gaps doc (`#92`);
      listserver-hard scenarios + buddy/guild seeder (`#93`, `#94`);
      RC rev on core `v0.4.0` (`#95`).
