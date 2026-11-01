# Salvage analysis: DClient setup bootstrapper

- **Binary:** `_extracted/dclient-setup/setup.exe`
- **Analyzed (UTC):** 20261031T210043Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Visual Basic 6 package-and-deployment bootstrapper, ~137 KB, March 2000
- **sha256:** `7052d75548d0f34e290baf29aa7281b44b4eb38327a9078354e15a3dc8749da4`

## Purpose

Small Windows installer bootstrap for a client package nicknamed DClient. It is not a game server and it does not speak the game protocol.

What it does:

- Reads a setup manifest beside the executable and pulls payload files from a cabinet in the same folder.
- Copies files into place, compares versions, registers libraries, and writes an uninstall entry plus an install log.
- Can ask for a reboot when a file is in use, and it adjusts process privileges enough to replace system files.
- Dialogs cover a missing disk, a file that cannot be read, a destination that is busy, a registration failure, and a log that cannot be written.

The companion cabinet in the recovered archive is truncated (a few hundred kilobytes of a multi-megabyte archive, and the zip itself has no end-of-central-directory record). The installed client and its runtime library cannot be reconstructed from this drop.

Imports are installer machinery: registry, file copy and search, process creation, version resources, a common file dialog, simple text drawing, and reboot privileges. There is no listen or connect role.

## What Seihai could learn

Nothing to reimplement. This is an incomplete client installer fragment. Leave it staged and out of scope for server tools. Do not run it.

## Recommended placement

| Placement | `stay-staged (bad archive)` → `out-of-scope` (client installer) |
| --- | --- |
| Why | Incomplete Drive zip; client-side bootstrap only. |
| Not | Any seihai-servertools crate |
| Licence | Unknown until a complete cabinet exists; treat as untrusted |

## Program (counts only)

| Field | Value |
| --- | --- |
| Functions | 435 (internal 426) |
| Imports | 183 |
| Interesting strings | 78 |
| Decompiled locally | 25 — **not committed** |

Raw imports, strings, and decompilation stay in the gitignored local analysis folder. This note is hand-written.
