# Salvage analysis: Graal Auto Key

- **Binary:** `_extracted/unpacked/Graal_Auto_Key.unpacked.exe` (static unpack of the UPX sample; the PE was never executed)
- **Analyzed (UTC):** 20261031T210030Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), ~85 KB unpacked
- **sha256:** `a99410a422694d84eb5ad1b93e2406c15f14a83a9d9d818bad3ab6f289ebf0f0`

## Purpose

Client trainer that sends keyboard and mouse input to the focused window. The shipped sample is UPX-packed. The unpacked image is an AutoIt 2.64 script host: script line errors, a file picker, registry access, token privileges, and a vendor banner. Named symbols are C runtime heap and locale helpers, not game logic. Exports are runtime entry stubs.

It does not speak a server protocol. It is the same class of tool as other recovered trainers.

## What Seihai could learn

Nothing for server tools. A host indicator is the file hash plus AutoIt-on-UPX packaging. Do not reimplement the macro, and do not run the sample.

## Recommended placement

| Placement | `out-of-scope` |
| --- | --- |
| Why | Client trainer. Decision tree: do not place in seihai-servertools. |
| Defence note | Same class as recovered-hacks trainers. |
| Licence | Untrusted third-party binary |

## Program (counts only)

| Field | Value |
| --- | --- |
| Functions | 303 (internal 301) |
| Imports | 158 (file, registry, process, and input-capable Win32 surface) |
| Exports | 13 runtime stubs |
| Interesting strings | 34 |
| Decompiled locally | 30 — **not committed** |

Raw import and string lists stay in the gitignored local analysis folder.
