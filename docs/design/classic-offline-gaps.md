# Classic offline tool jobs still open here

Date: 2026-11-16. Placement rules:
[salvage-placement.md](salvage-placement.md). Ordered work:
[NEXT.md](../NEXT.md). Behaviour notes that back these rows live in
Graal_analysis (`tools/`, especially `generators-222/`, `gif2mng.md`,
`levelfiller.md`, `classic-tools/gserverstat.md`); this page does not
list private archives.

Most classic converter / viewer / filler / generator **jobs** already
have a Seihai command (`level convert|render|fill`, `gmap make|render`,
`gen`, `gif2mng`, `stats`, movie paint). What remains is the short list
below — own implementation from measured notes only.

## Ready when evidence is enough

| Gap | Command / module | Evidence | Notes |
| --- | --- | --- | --- |
| Dungeon styles beyond maze (onion, spiral, rogue, snake, square spiral) | `gen dungeon`, `lat-core::generate` | Graal_analysis dungeon style shapes (clean-room; not DungeonMaker 0.9) | Prefer `--style` layouts next to `--maze` / `--layout` |
| `TILES` and `NPC` lines from a `GDUNG001` options file | same | Place names and 8×11 stamps in the generator notes | After styles; do not invent the cave sheet |
| GraalLevelGen water / mountain / sand edge stamps | `gen image` | Colour table known; edge blits not measured | Hold |
| gif2mng `-b` snaps palette to seven body slots | `gif2mng`, `lat-core::anim` | Transparent index 2 is known; **slot RGB not listed** | Keep `-b` as index-2 transparency until RGB is measured |
| Assimilator-style batch convert / checklist | none yet | Teardown describes a batch UI, not a convert matrix | Do not wrap a recovered EXE; document only |
| LevelFiller border pass vs a runnable classic copy | `level fill --across` | #82 shipped our border rule | Lab compare only; not a fidelity claim |
| Movie CHAR colours, chat, horse, params; movie page in the gani editor | `gani movie`, gani editor | Partial CHAR/TEXT paint already shipped | Confirm units from a saved `GANI0002` first |
| `stats` modes `bombpower` and `onlinetime` | `stats` | Classic mode table known | Hold until seihai-gserver accounts store the fields |

## Route elsewhere

| Job | Why not here |
| --- | --- |
| NPC-server objects in the NPC editor (DB NPCs, classes, weapons) | Needs `seihai-rc` stage 5; no second NC |
| GS2 compile / bytecode viewer | GS2 toolchain / gserver tests — not a binary wrap of a recovered compiler |
| Full offline level IDEs, tileset “suits” boards | Seihai client offline editor + `seihai-draw` |
| Script beautify helpers | Optional later via `seihai-core` parsers |
| Character designer, shop, clock, messenger, sound fix, guild maker | Client / product fluff |
| Live map of players, website player counts, RC chat→web, discord-gs2 | Website / RC integrations |
| Classic listserver EXE, trainers, packet stealers, client archives under “developer tools” | Forbidden or not tools gaps |

## Never

- Commit recovered executables, archives, or decompiled source.
- Put lab hosts, drive letters, or machine names in this repo.
- Invent body-slot RGB values or Assimilator batch rules without a
  measured note.
