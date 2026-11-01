# Salvage analysis: remotecontrol (Custom / Agret)

- **Binary:** `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Custom RemoteControl.rar.extracted/Custom RemoteControl/remotecontrol.exe`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), **Borland Delphi**, ~1.0 MB

## Purpose (plain behaviour)

Community-modified remote control (Agret branding, a blue theme, and a text config with nickname and UI colour keys). Classic Delphi visual library rather than the GTK RemoteControl3 line. Companion **`Agret_RC.exe`** in the same folder is a **UPX-packed** PE; a static unpack yields a different hash from this `remotecontrol.exe` (variant build, not a byte-identical pack).

Observed behaviour:

- Classic remote-control window title. Winsock connect to a server chosen from a server list, with connecting, failure, and disconnect messages.
- Tracks the player count and whether the NPC server is connected.
- Preferences live in the application registry, next to a local startup script, a flag to avoid saving the password, and toggles for the player list, the NPC list, and local NPCs.
- NPC tooling: attributes, script and flags taken from an NPC, an NPC level list, local NPCs, and a delete confirmation.
- Account and player editors.
- Same general staff RC role as stock Delphi RC packs (`rc.rar` / `rc2.rar` lineage) — useful comparative sample for `seihai-rc` wire behaviour.

**Do not run** against public servers.

## Recommended placement

| Tag | `seihai-rc` |
| --- | --- |
| Why | Staff RC session client; protocol/UI behaviour for clean-room RC. |
| Also | Keep PE staged; `analysis-only` for raw dumps. |
| Not | `setup-core` / `lat-core` / trainer scope |
| Licence | Legacy Delphi client — notes only, no decompile paste |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~3707 (internal ~3241) |
| Imports | ~471 (typical Delphi + WSOCK32/advapi/comctl) |
| Exports | PE entry stubs only (~3) |
| Interesting strings | ~159 |
| Decompiled (local only) | 30 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
