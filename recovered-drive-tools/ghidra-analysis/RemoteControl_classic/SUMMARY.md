# Salvage analysis: RemoteControl (classic `rc.rar`)

- **Binary:** `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc.rar.extracted/RemoteControl.exe`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~888 KB

## Purpose (plain behaviour)

Stock **Graal Remote Control** from the classic `rc.rar` line (sibling of `rc2.rar` / Custom Agret builds).

Observed behaviour:

- Classic remote-control window title and a server-list connect screen (connecting, disconnect, and cannot-connect messages).
- Preferences live in the application registry. A local startup script and password fields appear.
- NPC tooling: attributes, script and flags from an NPC, an NPC level list, local NPCs, and a delete confirmation. It reports when the NPC server is connected, and it shows a player count.
- Account and player editors. Winsock connect. Embedded deflate.

Comparative baseline for `remotecontrol_custom/` (Agret) and `RemoteControl-NW/`.

**Do not run** against public hosts.

## Recommended placement

| Tag | `seihai-rc` |
| --- | --- |
| Why | Canonical Delphi RC client for clean-room staff-session behaviour. |
| Staging | PE/dumps remain `analysis-only` artifacts |
| Not | `setup-core`, trainers |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~3623 (internal ~3163) |
| Imports | ~466 |
| Interesting strings | ~137 |
| Decompiled (local only) | 20 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
