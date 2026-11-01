# Salvage analysis: PNGView

- **Binary:** `Developer Tools/GAT Tool - [Graphics Administration Team]/PNGView.rar.extracted/PNGView/PNGView.exe`
- **Analyzed (UTC):** 20261031T212141Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~800 KB

## Purpose (plain behaviour)

GAT-era **offline PNG / image viewer** helper. Graphics-admin utility for inspecting asset files — not a network service. Useful only as a behaviour/context note next to other GAT converters (`Assimilator`, tile editors); not a rewrite priority.

## Recommended placement

| Tag | `analysis-only` |
| --- | --- |
| Why | Offline viewer; no protocol or server placement. |
| Related | `Assimilator/`, `TilesEditor/` |
| Not | `setup-core`, `seihai-rc`, `lat-core` |
| Licence | Legacy Delphi — notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~3348 (internal ~2965) |
| Imports | ~388 |
| Exports / entries | ~3 |
| Interesting strings | ~120 |
| Decompiled (local only) | 20 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
