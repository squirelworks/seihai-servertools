# Salvage analysis: gserver (Linux ELF from GServersource)

- **Binary:** `/workspace/recovered-drive-tools/_inbox/extracted/GServersource/gserver`
- **Analyzed (UTC):** prior headless · SUMMARY rewritten 20261031T211700Z — **prefer paired C++ source**
- **Ghidra:** 12.2
- **Purpose focus:** recover CLI, protocols, config paths, and rebuild/rehost notes for seihai-servertools (ops/server tooling — not malware inventing).

## Purpose guess

Classic **Graal game server** daemon (ELF32, dynamically linked, not stripped, ~Jun 2002). Same tree as Windows `gserver.exe` and full sources (`gserver.cpp`, `npcs.cpp`, `levels.cpp`, MySQL account helpers, password crypto). Headless RE confirms symbols already named in source (`startServer`, `parse`, `enterLevel`, `SendAccount`, RC player-prop paths, DB load/save).

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | Canonical legacy gserver; licence/GPL-adjacent. Seihai already has its own server — use only as protocol/behaviour reference. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` (do not paste sources into MIT crates) |
| Related | `gserver-win`, `gserver_pe_GServersource`, `gserver13`, Matriark 1.39 build |
| Licence risk | High — source tree includes `passwords.h` / crypto; clean-room notes only |

## Identity

```
# file
ELF 32-bit LSB executable, Intel 80386, dynamically linked, for GNU/Linux 2.2.5, not stripped

# size
~1.3M

# sha256
56d9f0f8ba666c2e9ceb556932c01317374cf9d0e56c730b4d2643ce56947f9e
```

## Ghidra program

| Field | Value |
| --- | --- |
| Format | ELF |
| Language | x86:LE:32:default / gcc |
| Functions | 628 (internal 419) |
| Imports | 105 |
| Interesting strings | 641 |
| Decompiled funcs | 40 |

## Behaviour (source-led)

From `gserver.h` / `gserver.cpp`: listens on a configured port (`startServer`), accepts players and Remote Control sessions, parses client package types (level warp, board modify, player/NPC props, bombs, horses, arrows, flags, chests, weapons, PMs, …). Account persistence via MySQL (`connecttodb`, `LoadDBAccount` / `SaveDBAccount`) and/or file-era helpers; password encryption helper present. RC-facing paths update player/account props and write RC log lines. Level/NPC/weapon loaders (`LoadNW`, `LoadWeapons`, `enterLevel`) drive world state.

**Do not deep-decompile further** while this source tree is available.

## Prefer source trees

- `_inbox/extracted/GServersource/` (and Developer Tools `GServer Source/GServersource.rar.extracted/`)
- Older `gserver_source/` for `gserver13` / stats tools

## Artifacts

- Local Ghidra export retained for symbol cross-check; authoritative behaviour = source.
