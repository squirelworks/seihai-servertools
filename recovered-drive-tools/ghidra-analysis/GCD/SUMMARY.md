# Salvage analysis: Graal Character Designer

- **Binary:** `_inbox/extracted/Graal_Character_Designer_1.2.7/GCD.exe`
- **Analyzed (UTC):** 20261031T205951Z · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~399 KB, November 1999
- **sha256:** `d576bc534dc17425645a0adb46b401ad4a081b19f4315cf8054aef9529cfeed9`

## Purpose

Offline character-appearance editor from the Bruges / Andrew Bridges line, version 1.2.7. It edits classic colours, heads, and character files. It is not a network tool.

Screens and files:

- A main designer form, an about box, and an install-location prompt.
- It searches a few common client install directories, then asks the operator to point at the client folder and looks for the client executable.
- It expects a heads configuration and separate folders of head images and gif assets. A missing heads configuration is a hard error. Gif loads can fail on their own.
- Open and save dialogs, image-list controls for drag and draw, clipboard limits (icons are not accepted), and a help hook.
- Delphi runtime and locale data only besides that. No socket use.

## What Seihai could learn

The client needs a heads configuration and image folders next to the install, and character editing is an offline document workflow. That belongs in client appearance tooling if it is ever rebuilt. It does not inform server, listserver, or remote-control work. Do not paste the Delphi UI.

## Recommended placement

| Placement | `out-of-scope / seihai (client)` |
| --- | --- |
| Why | Offline character and sprite tooling. |
| Not | `setup-core` / `lat-core` / `seihai-rc` |
| Licence | Legacy third-party Delphi tool — notes only |

## Program (counts only)

| Field | Value |
| --- | --- |
| Compiler | Borland Delphi |
| Functions | 1884 (internal 1530) |
| Imports | 375 (registry, image lists, file dialogs, a large GDI surface) |
| Interesting strings | 88 |
| Decompiled locally | 30 — **not committed** |

Most symbols are anonymous Delphi stubs. Behaviour above is from the import shape and the string table, paraphrased. Raw dumps are not committed.
