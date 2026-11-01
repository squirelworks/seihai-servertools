# Defense RE — `Graal Prayer` / `data/data.exe`

**Static analysis only — binary was never executed.**  
**Analyzed (UTC):** 2026-10-31  
**Ghidra:** 12.2 headless  
**Input analyzed:** UPX-unpacked copy  
`/workspace/recovered-hacks/_static_extract/upx_unpacked/Graal_Prayer_data_data.exe`  
**Packed source:** `/workspace/recovered-hacks/Trainers/extracted/Graal_Prayer/data/data.exe`

## Identity

| Field | Value |
|--|--|
| Packed SHA-256 | `c331eee0e985c903db933432a62a4972b835aefe9a4832d9ce5705c694a86145` |
| Unpacked SHA-256 | `b72e36502b2638ceeb9642ecb8d9da3c08c880175e60ec782a48b9ae93172a4c` |
| Packed size | 42676 |
| Unpacked size | 86196 |
| PE TimeDateStamp | `1040239050` (same as `Graal Prayer.exe`) |
| Format | PE32 GUI, Intel 80386 |
| Packer / runtime | UPX → **Compiled AutoIt v2.64** |

## Relationship to wrapper

- Same AutoIt runtime / identical import table as `Graal Prayer.exe`.
- PE image identical for the first **86107** bytes of the unpacked files; divergence is in the trailing encrypted overlay / resources (`overlay` ≈ **180 B** here vs ≈ **924 B** on the wrapper).
- No cross-references to `Graal Prayer.exe` or `data\` paths in strings.
- Defense interpretation: second compiled macro in the same distribution (helper/alternate script), **not** a confirmed loader for a third-stage PE. Still elevates dual-EXE packaging as an IOC worth hashing both files.

## Defense-relevant capabilities

Same as wrapper (AutoIt v2.64 stock imports):

- Input: `keybd_event`, `mouse_event`, `SendMessageA`, `PostMessageA`, `FindWindowA`, `SetTimer`/`KillTimer`, `BlockInput`
- Process: `OpenProcess`, `CreateProcessA`, `VirtualAlloc` (no `WriteProcessMemory` / `CreateRemoteThread`)
- Network APIs: **none** observed
- Markers: `Compiled AutoIt v2.64 Script.`, `>AUTOIT SCRIPT<`, HiddenSoft URL

## Behavioral tells

- Filename `data.exe` under `data/` is itself a weak host IOC (paired with `Graal Prayer.exe`).
- Smaller encrypted overlay ⇒ shorter/simpler script than the wrapper (cannot confirm actions without decrypt).
- Same PE timestamp cluster as other Mar-2003 RpgWarrior UPX AutoIt trainers.

## Ghidra notes

| Metric | Value |
|--|--|
| Functions | 303 (301 internal) |
| Imports | 158 |
| Interesting strings | 34 |

Same interpreter-shaped call graph as the wrapper; no unique plaintext Graal window title recovered.

## Artifacts

- `imports.txt`, `exports.txt`, `functions.txt`, `strings_interesting.txt`
- `decompiled/`, `prelim/`, `logs/`, `summary_stats.txt`
- Sibling: `../Graal_Prayer/SUMMARY.md`
