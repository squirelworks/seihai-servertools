# Defense RE — `GTS.exe` (Graal Text Sender)

**Static analysis only — binary was never executed.**  
**Analyzed (UTC):** 2026-10-31  
**Ghidra:** 12.2 headless  
**Input:** `/workspace/recovered-hacks/Trainers/extracted/GTS/GTS.exe`

## Identity

| Field | Value |
|--|--|
| SHA-256 | `36e264ff1b9bd441b278ec4afc265c7991dc2f9ae065f721d178f59c74d2d0ab` |
| Size | 40960 |
| PE TimeDateStamp | `1043589876` (2003-01-26T14:04:36Z) |
| Format | PE32 GUI, 3 sections (`.text`/`.data`/`.rsrc`) — **not UPX** |
| Runtime | Visual Basic 6 (`MSVBVM60.DLL`) |
| Title / author | **Graal Text Sender by DarkShadowz v1.3.5** |

## Imports (defense view)

**PE import directory (Ghidra):** only `MSVBVM60.DLL` (VB runtime + ordinals).  
**Late-bound / string-visible Win32 APIs** (via VB `DllFunctionCall` / declare):

| API | Likely use |
|--|--|
| `FindWindowA` / `FindWindowExA` | Locate Graal (or chat) HWND |
| `GetWindowThreadProcessId` | Resolve PID for window |
| `OpenProcess` | Open target process handle |
| `GetAsyncKeyState` | Hotkey / key-state polling |
| `SendMessageA` | Inject chat / UI messages |
| `Shell_NotifyIconA` | Tray icon |
| `ShellExecuteA` | Launch URL/help |
| `BitBlt` / `TransparentBlt` | UI blitting |
| `GetTickCount` | Timing |
| `CloseHandle` | Handle cleanup |

Module name string: `SendKeys1` — VB-style SendKeys helper. UI frames: Word Scroller, Two Words Repeat, Letter-by-Letter shout/to-all, timer controls (`Set Timer`, `Timer1`).

**Not observed:** Winsock / WinINet imports; AutoIt markers; UPX sections.

## Window / branding IOCs

| String | Notes |
|--|--|
| `Graal Text Sender` | Window / product title |
| `Graal Text Sender by DarkShadowz v1.3.5` | Version banner |
| `Graal` | Standalone title token (FindWindow target candidate) |
| `This lil Graal Proggy was made by DarkShadowz` | About text |
| `the author's site` | Author URL |
| `C:\Program Files\Microsoft Visual Studio\VB98\Graal\Project1.vbp` | Build path leftover |
| `(To-All Messages Sent)` | Feature string |

## Behavioral tells

- Timer-driven chat spam / scroller modes into focused or found Graal window.
- Combination **`OpenProcess` + `GetAsyncKeyState` + `FindWindow*` + `SendMessageA`** is the strongest host signature for this tool (VB6 + those API names).
- Social harm: shout / to-all / letter spam — playerworld chat abuse; not host ransomware-class from static view.

## Ghidra notes

| Metric | Value |
|--|--|
| Functions | 34 (30 internal) |
| Imports listed | 76 (all MSVBVM60) |
| Interesting strings | 22 |

Decompiled VB thunks are low-signal; prefer string/API IOCs.

## Artifacts

- `imports.txt`, `strings_interesting.txt`, `decompiled/`, `prelim/`, `logs/`
- Full string dump also at `../../_static_extract/autoit_scripts/GTS.exe.allstrings.txt`
