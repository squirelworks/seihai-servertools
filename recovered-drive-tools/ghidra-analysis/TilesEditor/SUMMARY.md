# Salvage analysis: TilesEditor

- **Binary:** `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/TilesEditorrelease.rar.extracted/TilesEditor.exe`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32+ GUI (x86-64), MSVC/Qt-style resource tree, ~966 KB

## Purpose (plain behaviour)

Offline **tile editor** in the Reborn line (Luke Graham / 39ster, with a ports credit). Qt resources hang off the main window.

Observed editor capabilities:

- Edit levels and tiles, links, and chests, export a screenshot or map image, and preload overworld levels.
- NPC editing: anonymous and external classes, a verse played when a player is hurt, and prompts when closing a dirty NPC.
- Painting, layers, grid snap, and translucency.
- A script-ish side panel (script icons and a C++ highlighting rule) — still an offline document editor, not a live remote-control session.

Prefer promoting format/Document behaviour into **`seihai`** level-editor / shared edit crates; live LAT once RC exists belongs in **`lat-core`**.

## Recommended placement

| Tag | `seihai` (offline editor) · later `lat-core` for live edit |
| --- | --- |
| Why | Tile/NPC/chest Document editing — shared with client level editor. |
| Staging | Raw PE + Ghidra dumps remain here (`analysis-only` artifacts). |
| Not | Paste Qt/decompile into MIT crates; not `setup-core` |
| Licence | Community editor — behaviour notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Language | x86:LE:64 |
| Functions | ~2243 (internal ~1377) |
| Imports | ~1721 |
| Interesting strings | ~217 |
| Decompiled (local only) | ~20–25 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
