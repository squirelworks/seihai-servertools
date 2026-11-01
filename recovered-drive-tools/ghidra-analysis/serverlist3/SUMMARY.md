# Salvage analysis: serverlist3

- **Binary:** `/workspace/recovered-drive-tools/_extracted/server-lister/Server Lister/slist/serverlist3` (also under `_inbox/extracted/Server Lister/slist/` and Developer Tools Server Lister extract)
- **Analyzed (UTC):** 20261031T204000Z (artifacts) · SUMMARY rewritten 20261031T211700Z
- **Ghidra:** 12.2
- **Purpose focus:** recover CLI, protocols, config paths, and rebuild/rehost notes for seihai-servertools (ops/server tooling — not malware inventing).

## Purpose guess

Classic **list / login server** (ELF32, statically linked, not stripped). One process listens for client and game-server connections, serves the public server directory, and backs account auth against a database. It takes a full path to a configuration file as its only argument and refuses to start if that file is missing. The console reports that the lister has started, with the client and game-server port context, player-limit accounting, and disconnect notices.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | Legacy GPL-adjacent listserver binary; behaviour notes only. Prefer the recovered C# listserver rewrite tree under `_extracted/listserver-nested/graal-listserver/` for any modern rehost design. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | Win32 `ListServer` (Delphi), Server Lister package scripts (`gserverstats.pl`) |
| Licence risk | Legacy/static glibc+MySQL client — never paste into MIT crates |

## Identity

```
# file
ELF 32-bit LSB executable, Intel 80386, version 1 (SYSV), statically linked, for GNU/Linux 2.2.5, not stripped

# size
~3.3M

# sha256
eb5272ab1653f93fc00b41368caa52d7af565f16e3d575a18e4c1ba1e1da87c3
```

## Ghidra program

| Field | Value |
| --- | --- |
| Format | Executable and Linking Format (ELF) |
| Language | x86:LE:32:default |
| Compiler | gcc |
| Functions | 2197 (internal 2197) |
| Imports | 0 (static link) |
| Exports / entries | 4486 |
| Interesting strings | 2375 |
| Decompiled funcs | 50 |

## Behaviour (plain prose)

The config covers the client listen port, the game-server listen port, and a player cap. Side files track the free and used player budget: a maximum, a live count, and a small stats file.

Database shapes (behaviour only): account rows hold an encrypted password, email, block and ban fields, and admin level and worlds. Directory rows hold the world name, language, description, a homepage, client version, player count, address, and port, and they go stale if the last update is older than about ten minutes. Refusals are a wrong password or a source that is not allowed to connect. Game servers register as a world connection.

The network stack is Berkeley sockets plus a bundled database client (TCP and a local socket path, with SSL option names). Compression helpers are present. The missing-config notice is misspelled, which marks this era build.

## Prefer source / rewrite over deep RE

- `_extracted/listserver-nested/graal-listserver/` — C# listserver rewrite (primary reference for seihai-facing design).
- Server Lister package companions for ops HTML/stats scripts — notes only.

## Rebuild / rehost checklist

- [x] Identify language/runtime: static ELF32 C++/gcc + MySQL client + zlib.
- [x] CLI: config file path required as sole parameter.
- [x] Config: client port, game-server port, player cap; side files for player budgets.
- [x] Protocols: client list/login + gserver registration; MySQL account/serverlist tables.
- [ ] Prefer clean-room / C# rewrite notes over binary reuse.

## Artifacts

- `prelim/`, `functions.txt`, `functions.csv`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt` (local; do not commit decompiled C)
