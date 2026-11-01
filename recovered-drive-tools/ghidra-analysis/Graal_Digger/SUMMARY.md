# Salvage analysis: Graal Digger

- **Binary:** `_inbox/extracted/Graal_Digger/Graal Digger.exe`
- **Analyzed (UTC):** 20261031T210058Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), UPX-compressed, 3 sections, ~36 KB, December 2002
- **sha256:** `876723732378b36e8fd31d09120ec5a1b584cf46fb672ab9d5cf7a0a4f8b56d4`

## Purpose

Client auto-dig / away-from-keyboard shovel helper. A short howto ships in the pack. The file is the same truncated UPX image as the trainers copy (hash match). A static unpack fails because the overlay is corrupt. Ghidra sees only the packer stub: one internal function, no imports, and no application strings. Bytes that survive a string filter are packer noise, not UI text. Packed-stream hints point at an AutoIt 2.63 host, which matches the other trainer, but that script was not recovered.

## What Seihai could learn

Nothing. Out of scope. See also the trainers note under `recovered-hacks/ghidra-analysis/Graal_Digger/SUMMARY.md`. Do not run the PE.

## Recommended placement

| Placement | `out-of-scope` |
| --- | --- |
| Why | Client digger. Corrupt packer overlay. |
| Licence | Untrusted third-party binary |

## Program (counts only)

| Field | Value |
| --- | --- |
| Functions | 1 (internal 1) |
| Imports | 0 |
| Interesting strings | 0 readable |
| Decompiled locally | 1 stub — **not committed** |
