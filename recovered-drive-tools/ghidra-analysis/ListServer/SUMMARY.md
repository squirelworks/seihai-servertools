# Salvage analysis: ListServer (UPX-unpacked)

- **Binary:** `_extracted/unpacked/ListServer.unpacked.exe` (static `upx -d` of recovered Win32 ListServer; PE never executed)
- **Analyzed (UTC):** 20261031T204840Z · SUMMARY placement pass 20261031T212200Z
- **Ghidra:** 12.2
- **Format:** PE32 GUI (x86), Borland Delphi

## Purpose guess

Windows **list / server-directory** helper for classic Graal. Delphi visual library plus Winsock listen, bind, connect, and close. It reads an ini list of servers, stores client preferences in the application registry, and shows a connect dialog with version and player-count fields. Bundled deflate. Companion to the ELF listserver — different packaging, same product niche.

**Do not run** this PE (or the packed original) against public networks.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | Legacy Delphi listserver UI; prefer C# rewrite notes under `_extracted/listserver-nested/graal-listserver/` and ELF `serverlist3` behaviour notes. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | `serverlist3` |
| Licence | Legacy third-party — clean-room only |

## Ghidra program

| Field | Value |
| --- | --- |
| Functions | 3299 (internal 2874) |
| Imports | 430 (ADVAPI32 registry, COMCTL32, GDI32, WSOCK32) |
| Exports | 3 |
| Interesting strings | 112 |
| Decompiled (local only) | 40 — **not committed** |

## Behaviour (plain prose)

Most symbols are anonymous Delphi stubs. Behaviour comes from the import surface and the string table: a socket listen and connect path, a local server-list config, a client preferences hive, and JPEG UI chrome. No database client (unlike the static ELF listserver) — this build looks like a lightweight Windows lister rather than the full Linux auth directory.

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
