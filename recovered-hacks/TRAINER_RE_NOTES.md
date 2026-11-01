# Trainer reverse-engineering notes (defense)

**Date:** 2026-10-31  
**Scope:** Static analysis of highest-risk Graal trainer PEs under `/workspace/recovered-hacks/`  
**Method:** `file` / SHA-256 / PE parse / `strings` / `objdump` / `upx -t` on copies / Ghidra 12.2 headless (`tools/ghidra/analyze_tool.sh`)  
**Constraint:** **No execution** of trainers/bots. `Graal_Miner.zip` skipped (Safe Browsing block).

Per-binary writeups: `/workspace/recovered-hacks/ghidra-analysis/*/SUMMARY.md`

---

## Binaries analyzed

| Target | Path (primary) | Ghidra out dir | Result |
|--|--|--|--|
| Graal Prayer.exe | `Trainers/extracted/Graal_Prayer/Graal Prayer.exe` (analyzed UPX-unpacked twin) | `ghidra-analysis/Graal_Prayer/` | AutoIt v2.64 input macro runtime |
| data/data.exe | `Trainers/extracted/Graal_Prayer/data/data.exe` (unpacked twin) | `ghidra-analysis/Graal_Prayer_data/` | Second AutoIt v2.64 script PE |
| Graal Digger.exe | `Trainers/extracted/Graal_Digger/Graal Digger.exe` | `ghidra-analysis/Graal_Digger/` | Truncated UPX; AutoIt v2.63 cues only |
| GTS.exe | `Trainers/extracted/GTS/GTS.exe` | `ghidra-analysis/GTS/` | VB6 chat spam; FindWindow/OpenProcess/GetAsyncKeyState |
| AFK Bot.exe | `Trainers/extracted/AFKBOT/AFKBOT/AFK Bot.exe` | `ghidra-analysis/AFK_Bot/` | VB6 AFK chat bot (Zorg) |

---

## Cross-cutting defense IOCs

### Hashes (SHA-256)

| Sample | SHA-256 |
|--|--|
| Graal Prayer.exe (packed) | `6959bcb8066efdee0dd842b6ee9d769cf93230d90a4ebca04786bfddb7f88860` |
| data.exe (packed) | `c331eee0e985c903db933432a62a4972b835aefe9a4832d9ce5705c694a86145` |
| Graal Prayer.exe (UPX-unpacked) | `a348ecafc3fdf3069dcaf9dd6d776c3f83c6f6d4f547ccf09d94f3694d952074` |
| data.exe (UPX-unpacked) | `b72e36502b2638ceeb9642ecb8d9da3c08c880175e60ec782a48b9ae93172a4c` |
| Graal Digger.exe (truncated carve) | `876723732378b36e8fd31d09120ec5a1b584cf46fb672ab9d5cf7a0a4f8b56d4` |
| GTS.exe | `36e264ff1b9bd441b278ec4afc265c7991dc2f9ae065f721d178f59c74d2d0ab` |
| AFK Bot.exe | `35dbd87ca7560712aae0e1bb3c4b4861e5233eac32299170af25eaa71c13ff35` |

### Filenames / paths

- `Graal Prayer.exe`, `data\data.exe` (paired)
- `Graal Digger.exe`, `Graal_Digger-Howto.txt`
- `GTS.exe` / UI title **Graal Text Sender**
- `AFK Bot.exe` (and sibling `AFK Scroll Bot.exe`)

### Packer / runtime signatures

| Family | Markers |
|--|--|
| RpgWarrior AutoIt macros | UPX `UPX0`/`UPX1`; `Compiled AutoIt v2.64 Script.`; `>AUTOIT SCRIPT<`; `the AutoIt vendor site`; PE TimeDateStamp `1040239050` on Prayer pair |
| Digger (damaged) | UPX + AutoIt **v2.63** splash fragment; PE TimeDateStamp `1012939264` |
| GTS / AFK | `MSVBVM60.DLL`; VB6 `.vbp` absolute build paths under `...\VB98\...` |

### API / behavioral signatures (host)

| Tool | Notable APIs / tells |
|--|--|
| Prayer / data.exe | `keybd_event`, `mouse_event`, `SendMessageA`, `PostMessageA`, `FindWindowA`, `SetTimer`, `BlockInput`, `OpenProcess`, `CreateProcessA`, `VirtualAlloc` — **no** `WriteProcessMemory` / `CreateRemoteThread` / Winsock |
| GTS | Late-bound `FindWindowA`/`Ex`, `OpenProcess`, `GetAsyncKeyState`, `SendMessageA`, `SendKeys1`, multiple **Set Timer** spam modes; window title token `Graal` |
| AFK Bot | VB6 + `Timer1`; default phrases `I am AFK` / `I am using an AFK Bot`; warning string about illegality on playerworlds |
| Digger | Intact behavior not recoverable; howto implies shovel dig loop while Graal focused |

### Network

No `WSAStartup` / WinINet / WinHTTP imports observed on Prayer, data.exe, GTS, or AFK Bot static surfaces. Risk class is **client-side input/chat abuse**, not standalone C2 from these PEs.

### Process injection

AutoIt runtimes import `OpenProcess` + `VirtualAlloc` (capability noise). Classic injection APIs (`WriteProcessMemory`, `CreateRemoteThread`, hooks) **absent** from Prayer/data import tables. GTS `OpenProcess` pairs with window PID lookup — consistent with SendMessage automation, not proven shellcode inject.

---

## Findings by priority target

### 1. Graal Prayer + data/data.exe

- Dual-EXE package is **two compiled AutoIt scripts**, not a demonstrated PE dropper (no path strings linking them).
- Encrypted overlays differ (~924 B vs ~180 B) after a shared RpgWarrior overlay header — treat both hashes as IOCs.
- Script plaintext not recovered (AutoIt v2 encrypted overlay); defense value is API/string/hash cluster.

### 2. Graal Digger.exe

- Zip yields truncated deflate of the PE (36111 / 41893 bytes). `upx` refuses unpack; Ghidra sees only UPX stub.
- Still classifiable as AutoIt+UPX dig macro via strings + Selzar howto.
- **Blocker:** missing ~5.7 KiB prevents import recovery and script recovery.

### 3. GTS.exe

- Confirmed DarkShadowz **v1.3.5** Graal Text Sender.
- Strongest combo IOC: VB6 + `OpenProcess` + `GetAsyncKeyState` + `FindWindow*` + product strings / `the author's site`.
- Timer UI frames map directly to chat flood modes (scroller, to-all, shout).

### 4. AFK Bot.exe

- Representative AFK presence bot; string-rich, import-poor.
- Server/chat defenses: match default AFK phrases and periodic identical sends; host: filename + Zorg UI strings.

---

## Detection ideas (defensive only)

1. **Hash allow/deny** for the table above (packed + unpacked AutoIt where endpoints unpack UPX).
2. **YARA-ish concepts:** UPX section names + `Compiled AutoIt v2.6` + `>AUTOIT SCRIPT<`; or VB6 + `Graal Text Sender` / `GetAsyncKeyState` + `OpenProcess`.
3. **Window title:** `Graal Text Sender`, `AFK Bot`, AutoIt tray/debug window class behaviors.
4. **Server-side:** rate-limit shout/to-all; flag periodic identical AFK lines; dig/prayer action frequency caps on classic worlds.
5. **Do not** rely on Digger imports until a complete PE is obtained.

---

## Blockers

| Item | Blocker |
|--|--|
| Graal_Miner.zip | Safe Browsing — not downloaded/executed (per policy) |
| Graal Digger.exe | Truncated zip payload; corrupt UPX overlay; Ghidra import table empty |
| AutoIt script bodies (Prayer/data) | Encrypted overlays; no plaintext script recovered in this pass |
| GTS/AFK named Win32 imports in PE IDT | Hidden behind VB runtime / `DllFunctionCall` — use strings |

---

## Artifact index

```
/workspace/recovered-hacks/ghidra-analysis/Graal_Prayer/SUMMARY.md
/workspace/recovered-hacks/ghidra-analysis/Graal_Prayer_data/SUMMARY.md
/workspace/recovered-hacks/ghidra-analysis/Graal_Digger/SUMMARY.md
/workspace/recovered-hacks/ghidra-analysis/GTS/SUMMARY.md
/workspace/recovered-hacks/ghidra-analysis/AFK_Bot/SUMMARY.md
/workspace/recovered-hacks/TRAINER_RE_NOTES.md          (this file)
/workspace/recovered-hacks/_static_extract/upx_unpacked/  (Prayer/data unpacked inputs)
```
