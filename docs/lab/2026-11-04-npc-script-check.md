# GS1 check of level NPC scripts, 2026-11-04

What: `seihai-servertools check WORLD` now parses every level NPC's script
with `seihai-core`'s GS1 parser (`lat_core::npc::check_script`) and reports
parse problems, and the images and sounds the script names with a fixed
name (`setimg`, `setgif`, `setimgpart`, `setgifpart`, `showimg`, `play`,
`playlooped`, `play2`) when they are not in the world. All are warnings.

## Runs

1. **Made-up world** (one level, three NPCs; the same shape as the
   `checks_level_npc_scripts` test): `showimg 1,lamp.png,...` and
   `play creak.wav` were reported missing, `setimg door.png` and
   `play chest.wav` (present) were not, `setimg #s(this.pic)` was left
   out (built at run time). An unclosed `if {` gave
   `GS1 script line 4: missing }`. An NPC with four broken statements
   listed three and then `1 more GS1 script problem`. Exit status 0.
2. **Seihai's default world** (`assets/default` at the pinned seihai
   commit): 7 levels, 10 ganis, 0 warnings before and after. Its levels
   have no NPCs.
3. **Seihai's NPC-script fixtures** (the five files under
   `crates/seihai-core/tests/fixtures/npc-scripts` at the pinned commit,
   each wrapped in a level; about 1,500 NPCs, 136,000 lines). Run locally
   only; nothing from them is in this repository.
   - 70 script findings (parse problems plus the "N more" lines), 216
     missing script images and sounds, and the NPC image warnings the
     checker already gave.
   - A first version also listed names the parser does not know as
     commands. That gave 267 more warnings, many of them real server-side
     commands (`addweapon`, `putnpc2`, `shoot`), because the parser's
     vocabulary is the client's. Those are no longer reported;
     `ScriptCheck::unknown` keeps them for the NPC editor.
   - One parse problem checked by hand was a real typo in the script (a
     stray `]`). Another sat on a `showimg2` line that looks fine, so a
     parse problem means "seihai-core's parser skips this", which is what
     the Seihai client does; it is not proof that another client fails.

## Not covered

- Scripts written for the newer language (GS2) give parse problems on many
  lines. The cap of three per NPC keeps one such NPC from burying the
  report.
- Heads, bodies, maps, palettes and letter sets named by scripts are not
  checked.
