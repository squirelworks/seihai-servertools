# Defense RE — `AFK Bot.exe` (representative AFK chat bot)

**Static analysis only — binary was never executed.**  
**Analyzed (UTC):** 2026-10-31  
**Ghidra:** 12.2 headless  
**Input:** `/workspace/recovered-hacks/Trainers/extracted/AFKBOT/AFKBOT/AFK Bot.exe`  
**Sibling (not fully re-run):** `AFK Scroll Bot.exe` — same Zorg/VB6 family

## Identity

| Field | Value |
|--|--|
| SHA-256 | `35dbd87ca7560712aae0e1bb3c4b4861e5233eac32299170af25eaa71c13ff35` |
| Size | 24576 |
| PE TimeDateStamp | `1032629018` (2002-09-21T17:23:38Z) |
| Archive stamp | ~2014 on extracted tree |
| Format | PE32 GUI, 3 sections |
| Runtime | Visual Basic 6 (`MSVBVM60.DLL` only in PE import dir) |
| Author cue | **Zorg** — “AFK Bot created by Zorg” |

## Imports / APIs

Ghidra lists **only** `MSVBVM60.DLL`. No explicit `user32`/`kernel32` names in the import table (unlike GTS). Chat send path is almost certainly VB intrinsic **SendKeys** / late-bound calls not listed as named imports — weaker host API IOC than GTS.

UI/timer: form control `Timer1`, buttons Start, status `Sent 0 AFK Messages`.

## String IOCs (high value)

| String |
|--|
| `AFK Bot` |
| `AFK Bot created by Zorg` |
| `I am using an AFK Bot` |
| `I am AFK` |
| `WARNING : AFK Bots are illegal on some playerworlds` |
| `AFK = Away from Keyboard` |
| `Not currently AFK` |
| `Sent 0 AFK Messages` |
| Build path: `C:\Program Files\Microsoft Visual Studio\VB98\Source\AFK Bot\Project1.vbp` |

## Behavioral tells

- Operator focuses Graal chat; bot timer emits rotating AFK phrases (readme-documented pattern in taxonomy Group 1).
- Presence / online-time farming + chat noise; **no network imports** observed.
- Server-side: identical periodic chat lines matching default phrases above.

## Ghidra notes

| Metric | Value |
|--|--|
| Functions | 6 (5 internal) |
| Imports | 45 (MSVBVM60 only) |
| Interesting strings | 8 |

Small VB GUI; decomp confirms thin wrapper around VB runtime — detection should key off **resources/strings**, not exotic imports.

## Artifacts

- `imports.txt`, `strings_interesting.txt`, `decompiled/`, `prelim/`, `logs/`
- Strings also: `../../_static_extract/autoit_scripts/AFK_Bot.exe.allstrings.txt`
