# Salvage analysis: gserverstat2

- **Binary:** `_inbox/extracted/gserver_source/gserverstat2`
- **Analyzed (UTC):** 20261031T205935Z · Ghidra 12.2 · static only
- **Format:** ELF32, dynamically linked, not stripped, same era and size class as `gserverstat`, ~513 KB, December 1999
- **sha256:** `966feaab46fea8b93e8b6c8a23cc333b9161f8f08f246b376cf4bfdb375fedbd`

## Purpose

Admin sibling of `gserverstat`. Same offline account scan and the same libc file I/O. It does not listen.

The companion shell drives weapon and power boards — sword, gloves, bombs, shield, maximum power, and high alignment — and writes them under an admin stats directory. The page theme matches the public leaderboards: a fixed green table, a background image, and a pointer at a local startup script. Account input is the same store and the same optional CGI-style name lists.

Symbols match the public tool: account parse, load, and save, plus list and compression helpers shared with classic gserver. Imports are formatted stdio only.

## What Seihai could learn

Same as `gserverstat`: optional lab status can read plain count files if a profile emits them. Do not rehost this HTML generator and do not paste its strings. Prefer `gserver_source/` over decompilation.

## Recommended placement

| Placement | `analysis-only` |
| --- | --- |
| Why | Admin weapon and power leaderboards from the same gserver-era tree. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | `gserverstat`, `statmaker` |
| Licence | Legacy gserver-era C++ — behaviour notes only |

## Program (counts only)

| Field | Value |
| --- | --- |
| Functions | 899 (internal 744) |
| Imports | 77 (stdio and libc; no sockets) |
| Exports / entries | 1296 |
| Interesting strings | 223 |
| Decompiled locally | 50 — **not committed** |

Raw exports and strings stay in the gitignored local analysis folder.
