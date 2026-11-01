# Salvage analysis: GChar (character generator)

- **Binary:** `_inbox/extracted/charactergenerator/charactergenerator/GChar.exe`
- **Analyzed (UTC):** 20261031T210011Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~407 KB
- **sha256:** `fe73e10fff6db1a9db88fc054315df05623a36f4fedc6ff6cf9c0f92038a60ea`

## Purpose

Offline character generator, same Bruges lineage as the Character Designer and labelled as a 1.0 sibling. Same job: edit classic character and head files without a network session.

Screens and files:

- Designer forms for appearance, plus an about box and a prompt for the client install directory.
- Looks for the client executable, a heads configuration, and head and gif asset folders. Missing heads configuration is an error.
- Keeps a private ini and can write a local error log. A log-errors switch is present.
- Image lists, open and save dialogs, and the usual Delphi drawing surface. Gif loads can fail. No sockets.

## What Seihai could learn

Same as the Character Designer: client-folder discovery and offline character files. Useful only as a note next to client appearance editors. Not a servertools target.

## Recommended placement

| Placement | `out-of-scope / seihai (client)` |
| --- | --- |
| Why | Client character tooling. |
| Not | `setup-core` / `lat-core` / `seihai-rc` |
| Licence | Legacy third-party tool — notes only |

## Program (counts only)

| Field | Value |
| --- | --- |
| Compiler | Borland Delphi |
| Functions | 1934 (internal 1562) |
| Imports | 375 (same Delphi GUI shape as the Character Designer) |
| Interesting strings | 90 |
| Decompiled locally | 30 — **not committed** |

Raw imports and strings stay in the gitignored local analysis folder.
