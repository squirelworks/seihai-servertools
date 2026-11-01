# Salvage analysis: gserver13

- **Binary:** `/workspace/recovered-drive-tools/_inbox/extracted/gserver_source/gserver13`
- **Analyzed (UTC):** prior headless · SUMMARY rewritten 20261031T211700Z — **prefer source/tree notes**
- **Ghidra:** 12.2

## Purpose guess

Earlier classic **Linux gserver** ELF32 (not stripped, ~Dec 1999, dynamically linked). Sibling sources/objects in `gserver_source/` (`gserver.cpp`, makefile, zlib). Symbols cover account parse/apply, level enter/update, guild name verify, ban-IP checks, inflate helpers, RC-related NPC/server string types — same product family as later GServersource, older build.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | Historical gserver; compare against later source tree rather than paste. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | `gserverstat` / `gserverstat2` / `statmaker` in same folder |
| Licence risk | Legacy/GPL-adjacent |

## Identity

```
ELF 32-bit LSB executable, Intel 80386, dynamically linked, for GNU/Linux 2.0.0, not stripped
~870K · sha256 be1c397d2c21049c7848a7c922455861ee0a4b413967c867c2e7efa90a7c9997
```

## Ghidra program

| Field | Value |
| --- | --- |
| Functions | 1128 (internal 915) |
| Imports | 106 |
| Interesting strings | 448 |
| Decompiled funcs | 40 |

## Prefer source

`_inbox/extracted/gserver_source/` — do not deep-decompile further for salvage docs.
