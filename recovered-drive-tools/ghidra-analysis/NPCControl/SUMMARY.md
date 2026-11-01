# Salvage analysis: NPCControl

- **Binary:** `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001 [V2].rar.extracted/nc7132001/NPCControl.exe`
- **Same-size twin:** `nc7132001.exe` in the same pack / `nc7132001.rar`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), **Borland Delphi**, ~471 KB

## Purpose (plain behaviour)

Classic **NPC Control** staff client for an NPC server or world control port. The banner dates it to a July 2001 GServer2-era build.

Observed behaviour:

- Reads a small text config for the server name, port, and password, and mirrors those preferences in the application registry.
- The window is titled as NPC Control. It reports a connected port, a failed connect, and a decompression failure.
- Manages NPCs and weapons: delete confirms, take a script from a class, edit a weapon script, and a local-NPC list.
- Opens rich-text and plain script files, and references a local startup script plus sample level names (ignore any embedded world names).
- Ships deflate helpers for compressed wire or packed payloads.

Sibling pack also includes `RemoteControl-NW.exe` (see that SUMMARY) for the RC side of the same era.

**Do not run** against public hosts.

## Recommended placement

| Tag | `seihai-rc` |
| --- | --- |
| Why | Live NC session / NPC+weapon script admin over the wire. |
| Also | PE stays staged (`analysis-only` dumps). |
| Not | `setup-core`, `lat-core`, client trainers |
| Licence | Legacy Delphi — clean-room notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~2428 (internal ~2052) |
| Imports | ~394 |
| Interesting strings | ~102 |
| Decompiled (local only) | 30 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
