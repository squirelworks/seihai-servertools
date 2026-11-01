# Salvage analysis: RemoteControl-NW

- **Binary:** `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New World/GraalNW2.rar.extracted/GraalNW2/$Graal-NW/RemoteControl-NW.exe`
- **Also in:** `NC [NPC Control]/nc7132001 [V2].rar.extracted/…/RemoteControl-NW.exe` (same size class)
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), **Borland Delphi**, ~588 KB

## Purpose (plain behaviour)

**New World–era Remote Control** Delphi client bundled with NW client packs and the NC V2 pack.

Same family as classic `rc.rar` / Custom Agret RC:

- Reads the same kind of text config as the other Delphi remote controls. Account and player editors, and a database account list that can search attributes such as admin level, admin worlds, and block state.
- Staff actions: admin message and broadcast, edit account, update level, disconnect, ban, mark a player away when the list closes, and a toggle for HTML in private messages.
- Winsock connect path; zlib deflate embedded.
- Comparative sample for NW protocol quirks vs v1 RC / GTK RC3.

**Do not run** on public networks.

## Recommended placement

| Tag | `seihai-rc` |
| --- | --- |
| Why | Staff RC binary (NW variant). |
| Also | Staged PE / dumps = `analysis-only` |
| Not | `setup-core` / trainers |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~2667 (internal ~2263) |
| Imports | ~418 |
| Interesting strings | ~106 |
| Decompiled (local only) | 30 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
