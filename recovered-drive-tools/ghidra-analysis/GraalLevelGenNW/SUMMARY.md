# Salvage analysis: GraalLevelGenNW

- **Binary:** `Developer Tools/LAT Tool - [Level Administration Team]/Level Generators/GraalOnline Level Generator - NW - [2001-10-08].rar.extracted/GraalLevelGenNW/GraalLevelGenNW.exe`
- **Analyzed (UTC):** 20261031T211951Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~422 KB

## Purpose (plain behaviour)

Offline **New World level / world generator**. The window is branded as a level generator, and the output file starts with a short magic that marks a New World level. The operator loads a map image, then generates a world into a levels directory. The pack also holds a sample map image and a large map text table. Strings cover New World calendar fields (year, month, day, hour, and smaller units) and per-level options such as whether warping is allowed. Not a network service.

Sibling LAT generators exist for classic Graal, terrain, and dungeon variants — same offline-content role.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `lat-core` (content tooling) / `analysis-only` for the PE |
| --- | --- |
| Why | Offline world/level generation aids LAT workflows; promote behaviour into generators/docs, not binary reuse. |
| Not | `setup-core` / `seihai-rc` |
| Related | `GraalLevelGen.exe` (classic), TerrainGenerator, dungeongen, Graal Map Generator (2024 src) |
| Licence | Legacy Delphi — clean-room notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~2038 (internal ~1678) |
| Imports | ~375 |
| Interesting strings | ~118 |
| Decompiled (local only) | 20 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
