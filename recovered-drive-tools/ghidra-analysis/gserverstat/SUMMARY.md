# Salvage analysis: gserverstat

- **Binary:** `_inbox/extracted/gserver_source/gserverstat`
- **Analyzed (UTC):** 20261031T205919Z · Ghidra 12.2 · static only
- **Format:** ELF32, dynamically linked, not stripped, egcs-era, for early GNU/Linux, ~513 KB, December 1999
- **sha256:** `b8ae1a13147567d6544fb30ab383359e6c1c19849deb9d87d219f7cf90af867d`

## Purpose

Offline HTML leaderboard generator for classic gserver account stats. It is not a network service.

What it does:

- Reads the on-disk account store and optional name lists that a CGI-style helper would have written beside the server tree.
- Emits a simple themed HTML table (a fixed green page, a background image, muted text and links) that points at a local startup script.
- A companion shell runs the binary once per board — kills, deaths, currency, high and low alignment, and time online — and redirects the HTML into a stats directory under the web tree.
- The image is unstripped and shares account parse, load, and save helpers with classic gserver, including temporary accounts, weapon name lists, and buffer compression. A server-frame type from that tree is present, but this program only formats stats.
- Imports are libc file and formatted I/O. There is no listen, bind, or socket use.

Build leftovers place the compile in the late-1990s gserver stats tree (the same folder as the classic server sources) with an egcs and glibc 2.1-era toolchain.

## What Seihai could learn

Lab status does not need this emitter. If a world folder already writes plain count files, `setup-core` can read those. Do not copy the HTML theme, the CGI paths, or this binary. Prefer the paired sources in `gserver_source/` over further decompilation.

## Recommended placement

| Placement | `analysis-only` |
| --- | --- |
| Why | Legacy account-dump tooling. Behaviour notes only. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | `statmaker` and the Server Lister stats script in the same era |
| Licence | Legacy gserver-era C++ — clean-room notes only |

## Program (counts only)

| Field | Value |
| --- | --- |
| Functions | 899 (internal 744) |
| Imports | 77 (stdio and libc; no sockets) |
| Exports / entries | 1296 (unstripped C++ helpers, not a network API) |
| Interesting strings | 223 |
| Decompiled locally | 50 — **not committed** |

Mangled names, import tables, and string text stay in the gitignored local analysis folder.
