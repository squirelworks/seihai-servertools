# Defense RE — `Graal Prayer.exe`

**Static analysis only — binary was never executed.**  
**Analyzed (UTC):** 2026-10-31  
**Ghidra:** 12.2 headless via `/workspace/tools/ghidra/analyze_tool.sh`  
**Input analyzed:** UPX-unpacked copy  
`/workspace/recovered-hacks/_static_extract/upx_unpacked/Graal_Prayer_Graal_Prayer.exe`  
**Packed source:** `/workspace/recovered-hacks/Trainers/extracted/Graal_Prayer/Graal Prayer.exe`

## Identity

| Field | Value |
|--|--|
| Packed SHA-256 | `6959bcb8066efdee0dd842b6ee9d769cf93230d90a4ebca04786bfddb7f88860` |
| Unpacked SHA-256 | `a348ecafc3fdf3069dcaf9dd6d776c3f83c6f6d4f547ccf09d94f3694d952074` |
| Packed size | 43420 |
| Unpacked size | 86940 |
| PE TimeDateStamp | `1040239050` (2002-12-18T19:17:30Z) — shared with sibling RpgWarrior builds |
| Format | PE32 GUI, Intel 80386 |
| Packer / runtime | UPX → **Compiled AutoIt v2.64** |

## Package role

Ships with sibling `data/data.exe` in the same zip (`Graal_Prayer.zip`). Both are independent AutoIt v2.64 EXEs (identical import sets; identical PE timestamp). No plaintext `data.exe` / path string was found in either image — dual layout is packaging, not an obvious PE dropper string. Encrypted script overlays differ in length (this EXE ~924 B vs `data.exe` ~180 B), so they are **two scripts**, not byte-identical copies.

## Defense-relevant capabilities (imports / strings)

**Input / window automation (USER32):** `keybd_event`, `mouse_event`, `SendMessageA`, `PostMessageA`, `SendMessageTimeoutA`, `FindWindowA`, `GetForegroundWindow`, `SetForegroundWindow`, `EnumWindows`, `GetWindowTextA`, `SetTimer` / `KillTimer`, `BlockInput`, `GetKeyState`, `VkKeyScanA`.

**Process / memory (KERNEL32):** `OpenProcess`, `CreateProcessA`, `VirtualAlloc` / `VirtualProtect` / `VirtualFree`, `TerminateProcess`, `Sleep`.  
**Not imported:** `WriteProcessMemory`, `CreateRemoteThread`, `SetWindowsHook*`, `SendInput`, any Winsock / WinINet / WinHTTP API.

**Other:** `RegOpenKeyExA` family (ADVAPI32); `Shell_NotifyIconA`; `ExitWindowsEx` (AutoIt runtime capability — shutdown). Profile APIs `Get/WritePrivateProfileStringA` present (INI-style config capability).

**AutoIt markers:** `Compiled AutoIt v2.64 Script.`, `>AUTOIT SCRIPT<`, `Powered by AutoIt - the AutoIt vendor site`, `SETKEYDELAY`, `HIDEAUTOITWIN`, `A_SCRIPTDIR` / `A_SCRIPTNAME`.

## Behavioral tells (for detection)

- UPX section names `UPX0`/`UPX1` on disk sample; after unpack, classic AutoIt v2 string cluster.
- Likely focuses Graal client then synthesizes keyboard/mouse via `keybd_event` / `mouse_event` (not raw `SendInput`).
- Timer-driven loop (`SetTimer`) typical of AFK prayer/emote macros.
- No network import surface — host C2 unlikely from this PE alone; risk is **game automation**.

## Script recovery status

Embedded AutoIt script body is **encrypted** in the PE overlay (high-entropy blob after `.rsrc`, shared 16-byte family header `a3484bbe…` across RpgWarrior unpacks). Plaintext script lines were **not** recovered; only runtime keyword tables after `>AUTOIT SCRIPT<` (e.g. `RWINUP`, key names). Treat payload as “AutoIt input macro” from packaging/name + API set.

## Ghidra notes

| Metric | Value |
|--|--|
| Functions | 303 (301 internal) |
| Imports | 158 |
| Interesting strings (export filter) | 34 |

Decompilation is mostly CRT / AutoIt interpreter glue, not a readable user script. Prefer strings + imports for defense IOCs over deep decomp.

## Artifacts

- `imports.txt`, `exports.txt`, `functions.txt`, `strings_interesting.txt`
- `decompiled/`, `prelim/`, `logs/`, `summary_stats.txt`
- Sibling notes: `../Graal_Prayer_data/SUMMARY.md`
