# Salvage analysis: gserver.exe (Win32 from GServersource)

- **Binary:** `/workspace/recovered-drive-tools/_inbox/extracted/GServersource/gserver.exe`
- **Analyzed (UTC):** prior headless · SUMMARY rewritten 20261031T211700Z — **prefer paired C++ source**
- **Ghidra:** 12.2

## Purpose guess

Windows console PE of the **same classic gserver** as `gserver-linux` (same tree, same era, sha256 `ad146b971eb1af76471d439f3f48b843e44c03b9b85f0ec9995877d6ba91bc01`). Symbols mirror Linux: `startServer`, parse/chat, account send, NW level load, RC player props, MySQL connect/bind/listen/accept, password encrypt.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | Duplicate of Linux ELF behaviour; source tree authoritative. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | `gserver-linux`, `gserver_pe_GServersource` (same PE analyzed under alternate dir name) |
| Licence risk | Same as Linux twin — clean-room only |

## Identity

```
PE32 executable (console) Intel 80386, for MS Windows, 6 sections · ~2.8M
```

## Ghidra program

| Field | Value |
| --- | --- |
| Functions | 1051 (internal 1007) |
| Imports | 141 |
| Interesting strings | 1762 |
| Decompiled funcs | ~29–39 |

## Note

`gserver_pe_GServersource/` analyzes the **same binary path/hash**. Keep one behavioural story; avoid duplicate deep RE.
