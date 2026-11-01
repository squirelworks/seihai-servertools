# Threat taxonomy & attack-surface map — Drive `Hacks`

**Generated:** 2026-10-31 (static analysis only; no code execution)  
**Drive source:** Owner private Google Drive folder named `Hacks` (URL and folder id omitted from this repo)  
**Local staging:** `/workspace/recovered-hacks/`  
**Scope note:** Google-drive MCP `list_folder` on the Hacks root currently returns **only** `Trainers/`. Parent Drive folder also holds `Tools`, `Graal clients`, `Graal playerworlds`, etc., but those are outside this Hacks tree. Classification below covers everything present in Hacks + local downloads as of this write.

## Inventory status

| Source | Result |
|--|--|
| Drive MCP early list | `Trainers/` + `Trainers/AutoPrograms by RpgWarrior/` (20 archive entries) |
| Local download | 18/20 archives complete; 2 blocked by Google Safe Browsing |
| Sibling extract | Most archives already unpacked under `Trainers/extracted/` and `…/AutoPrograms by RpgWarrior/extracted/` |
| Static work product | `/workspace/recovered-hacks/_static_extract/` (PE notes, UPX-unpacked copies, string dumps) — **do not execute** |

### Archive checklist

| Path | Size | Status | Contents (static) |
|--|--:|--|--|
| `Trainers/AFKBOT.7z` | 5001 | OK | `AFK Bot.exe` + readme (Zorg) |
| `Trainers/AFKSCROLLBOT.7z` | 5668 | OK | `AFK Scroll Bot.exe` + readme (Zorg) |
| `Trainers/autoprogs.zip` | 111200 | OK | `AutoD.EXE`, `AutoIdle.exe`, `AutoS.exe` (UPX AutoIt) |
| `Trainers/capslock.zip` | 126297 | OK | `capslock.exe` + `readme.htm` (RJL Freeware) |
| `Trainers/Graal_Auto_Key.zip` | 38024 | OK | `Graal Auto Key.exe` (UPX AutoIt v2.64) |
| `Trainers/Graal_Digger.zip` | 39297 | OK | `Graal Digger.exe` (corrupt UPX overlay) + howto text in zip quirk |
| `Trainers/Graal_Miner.zip` | 39156 | **BLOCKED** | Drive 403 `cannotDownloadAbusiveFile` (malware/spam) |
| `Trainers/Graal_Prayer.zip` | 76631 | OK | `Graal Prayer.exe` + `data/data.exe` (both UPX AutoIt) |
| `Trainers/Graal_Random_Walker.zip` | 38442 | OK | `Graal Random Walker.exe` |
| `Trainers/Graal_Water_Get.zip` / `Graal_WaterGet.zip` | 38746 / 37327 | OK | Water-can macros (v1.0-style + older Howto) |
| `Trainers/GraalLoad.zip` | 18438 | OK | `Graal Load.exe` + `bomb.txt` |
| `Trainers/GTS.zip` | 8763 | OK | `GTS.exe` — Graal Text Sender (VB6) |
| `Trainers/AutoPrograms by RpgWarrior/*` | (dup set) | OK except Miner | Same RpgWarrior binaries; Miner also **BLOCKED** |

Duplicates under `AutoPrograms by RpgWarrior/` match Trainers hashes for the shared tools (same SHA-256 for Auto Key, Prayer, Water Get, Random Walker, Digger, etc.).

---

## Attack-class overview

| Class | Items | Risk to Seihai / game servers | Present? |
|--|--|--|--|
| Game / server cheat — AFK chat bots | AFKBOT, AFKSCROLLBOT | Economy/online-time abuse; chat noise | Yes |
| Game / server cheat — input macros (AutoIt) | RpgWarrior suite, autoprogs | Resource farming; repetitive actions | Yes |
| Game / server cheat — chat / appearance script spam | GTS, GraalLoad + `bomb.txt` | Chat flood; cosmetic / social disruption | Yes |
| Suspected malware / unknown PE | Graal_Miner.zip (×2) | Host compromise **if** Safe Browsing correct | Yes (blocked) |
| Benign utility (misfiled) | capslock (RJL) | Negligible | Yes |
| Web exploit / auth bypass / priv-esc / network protocol / supply-chain / crypto / misconfig / RAT (confirmed) | — | — | **Not observed** in current Hacks tree |

---

## Group 1 — AFK chat bots (social / presence farming)

### Items
- `Trainers/AFKBOT.7z` → `AFK Bot.exe` (Zorg, “Version 1.0”, ~2014 archive stamp; PE timestamp ~2002-era VB6)
- `Trainers/AFKSCROLLBOT.7z` → `AFK Scroll Bot.exe` (sequel; scroll-chat variant)

### Likely attacker goals
- Keep a Graal client “active” / farm online-time on playerworlds without being at the keyboard.
- Inject rotating or scrolling chat phrases into the focused chat box.

### Prerequisites
- Windows host with classic Graal client UI focused.
- User must click into Graal chat within a few seconds of start (documented for both tools).
- No server credential theft required; uses the already-logged-in client.

### Blast radius
- **Playerworld / gserver:** policy violation, jail/ban risk; chat clutter; inflated online-time / AFK rewards if any.
- **Seihai client:** does not inject into Seihai; pattern still informs anti-AFK and chat rate design for servers Seihai connects to / servertools operates.
- **Host OS:** VB6 runtime (`MSVBVM60.DLL`) GUI apps; no network imports observed in static import dir (VB late-bound APIs may still call user32 SendKeys-style paths).

### Static IoCs
| Indicator | Detail |
|--|--|
| Filenames | `AFK Bot.exe`, `AFK Scroll Bot.exe` |
| UI strings | `AFK Bot created by Zorg`, `I am using an AFK Bot`, `I am AFK`, `Scroll Text Here`, `Sent 0 AFK Messages` |
| Warning string | `WARNING : AFK Bots are illegal on some playerworlds` |
| SHA-256 | `35dbd87c…ff35` (AFK Bot), `fdab4020…2b84` (Scroll Bot) |
| PE | PE32 GUI, 3 sections, imports only `MSVBVM60.DLL` |

### Affected assets
- Classic Graal Windows client chat UI; playerworld online-time / anti-AFK rules.
- Indirect: any Seihai-compatible gserver that rewards presence or lacks chat rate limits (`seihai-servertools` ops).

### Review mode
**Doc-first** (readmes are explicit). Light Ghidra only if validating whether SendKeys vs SendMessage is used — low priority vs Group 4/5.

---

## Group 2 — RpgWarrior / Selzar AutoIt input macros (gameplay automation)

### Items
| Tool | Author cue | Intended action (from name/howto) |
|--|--|--|
| `Graal Auto Key.exe` | RpgWarrior set | Hold/repeat a key |
| `Graal Digger.exe` | Howto signed **Selzar** | Equip shovel → dig grass loop |
| `Graal Water Get.exe` / `Graal WaterGet.exe` | Howto **RpgWarrior** | Water-can at well, face down |
| `Graal Prayer.exe` + `data/data.exe` | dual-EXE package | Prayer / emote loop (name-based) |
| `Graal Random Walker.exe` | — | Random movement keys |
| `autoprogs`: `AutoD.EXE`, `AutoIdle.exe`, `AutoS.exe` | same AutoIt lineage | Dig / idle / (S)hortcut macros |
| `Graal_Miner.zip` | — | **Not downloaded** — see Group 4 |

### Likely attacker goals
- Automate repetitive classic-Graal actions (dig, fill water, pray, walk, idle) for resource/AFK gains.
- Run unattended while the Graal window is focused (howto: start tool → focus Graal → confirm).

### Prerequisites
- Windows + Graal client window; correct in-game item equipped / position (shovel, water can, well).
- Compiled **AutoIt v2.60–v2.64** runtimes packed with **UPX** (except Digger’s UPX stream is corrupt).

### Blast radius
- **Game economy / anti-bot:** farming bots; movement anomalies; identical key-timing patterns.
- **Client integrity:** no memory injection strings observed post-UPX unpack; primarily `keybd_event` / `mouse_event` / `SendMessageA` / `PostMessageA` / `OpenProcess` (AutoIt runtime capabilities).
- **Seihai:** macros target Win32 message/input APIs against the *legacy* client, not Seihai’s Rust input path. Still a threat model input for server-side detection if operators run classic + Seihai populations together.

### Static IoCs
| Indicator | Detail |
|--|--|
| Packer | UPX (`UPX0`/`UPX1` sections); `$Info: … UPX …` |
| Runtime | `Compiled AutoIt v2.64 Script.` / `AutoIt v2.60` / `v2.61` / `v2.63` |
| Vendor URL | `the AutoIt vendor site` |
| Marker | `>AUTOIT SCRIPT<` or `>>>> AutoIt Script Start <<<<` (script body not plaintext after marker) |
| APIs (unpacked) | `keybd_event`, `mouse_event`, `SendMessageA`, `PostMessageA`, `FindWindowA`, `OpenProcess`, `RegOpenKeyExA`, `BlockInput`, `SETKEYDELAY` |
| Howto paths | `Graal_WaterGet-Howto.txt`, `Graal_Digger-Howto.txt` |
| Dual-bin | `Graal_Prayer/data/data.exe` (SHA-256 `c331eee0…6145`) alongside wrapper `6959bcb8…8860` |
| Shared PE TimeDateStamp | `1040239050` across several Mar 2003 RpgWarrior builds |
| Digger anomaly | Zip local vs central filename mismatch (`Graal_Digger-Howto.txt` vs `Graal Digger.exe`); `upx -d` → `OverlayException: invalid overlay size` |

### Affected assets
- Classic Graal client input surface; playerworld resource nodes (dig spots, wells).
- Detection hooks in gserver / servertools: repetitive action, impossible AFK activity, identical timing.

### Review mode
| Item | Mode |
|--|--|
| Howtos / names | **Doc-only** |
| Unpacked AutoIt family | **Ghidra optional** — recover encoded script; confirm no secondary payload |
| `Graal Prayer.exe` + `data/data.exe` | **Ghidra recommended** — dual binary unusual for this set |
| `Graal Digger.exe` | **Ghidra recommended** — corrupt UPX overlay / zip trick |

---

## Group 3 — Chat & appearance script spam (social / grief)

### Items
- `Trainers/GTS.zip` → `GTS.exe` — **Graal Text Sender by DarkShadowz v1.3.5** (VB6)
- `Trainers/GraalLoad.zip` → `Graal Load.exe` (Hosler / the author's site) + payload script `bomb.txt`

### Likely attacker goals
- **GTS:** automated chat spam / word scroller / two-word repeat into Graal.
- **GraalLoad:** type `load<file.txt>` in-game; tool feeds scripted lines (appearance commands + countdown taunt in `bomb.txt`).

### Prerequisites
- Focused Graal chat; script files beside `Graal Load.exe`.
- `bomb.txt` uses classic client commands: `setnick`, `setbody`, `sethead`, `setcoat`, etc., then spam countdown to `!!BOOM!!`.

### Blast radius
- Chat flood / harassment; temporary nick/body spam if server accepts those client commands.
- Low host-compromise signal from docs; GTS imports `OpenProcess`, `GetAsyncKeyState`, `SendMessageA` (via strings / late bind) — worth verifying no keylogging beyond hotkeys.

### Static IoCs
| Indicator | Detail |
|--|--|
| GTS strings | `Graal Text Sender`, `Word Scroller`, `Word Stay`, `Two Words Repeat`, `SendKeys1` |
| GTS SHA-256 | `36e264ff…d0ab` |
| GraalLoad strings | `Graal Load - By Hosler`, `load<bomb.txt>`, `the author's site` |
| GraalLoad SHA-256 | `812a0d32…216e` |
| `bomb.txt` tokens | `setnick Bomb`, `You are going to die in`, `!!BOOM!!`, `setshield`, `setbody body5.png` |

### Affected assets
- Chat subsystem on playerworlds / listservers; Seihai chat UX parity (`documentation/chat-keys.md`) — servers should rate-limit / filter scripted floods.
- Appearance command surface if still honored server-side.

### Review mode
- `bomb.txt`, GTS UI strings: **doc-only**.
- `GTS.exe`: **light Ghidra / VB decomp** — confirm `GetAsyncKeyState` / `OpenProcess` usage (hotkey vs keylog).
- `Graal Load.exe`: **doc + light RE** if load-protocol surprises appear.

---

## Group 4 — Suspected malware (download blocked)

### Items
- `Trainers/Graal_Miner.zip` (private Drive file — id omitted)
- `Trainers/AutoPrograms by RpgWarrior/Graal_Miner.zip` (private Drive file — id omitted)

Both: **403 `cannotDownloadAbusiveFile`** — *“identified as malware or spam”*; reported size **39156** bytes each (likely duplicates).

### Likely attacker goals (inferred only)
- Name fits the RpgWarrior farming suite (mine resources), **or** Google’s classifier saw a real trojan/dropper (AutoIt malware families are frequently abused).

### Prerequisites / blast radius
- Unknown until a Safe Browsing override or alternate clean copy is obtained in an isolated lab.
- Treat as **host-compromise candidate** until proven otherwise — highest residual risk in this folder.

### Static IoCs (metadata only)
| Indicator | Detail |
|--|--|
| Placeholder | `Graal_Miner.zip.BLOCKED.txt` (both tree locations) |
| Size | 39156 bytes (zip) |
| MIME | `application/zip` |

### Review mode
**Ghidra / full malware RE required** once bytes are available in an isolated environment. Do **not** run. Until then: metadata-only.

---

## Group 5 — Benign utility (noise)

### Item
- `Trainers/capslock.zip` — RJL Software CapsLock v1.00 freeware (`readme.htm`, Aspack-packed PE).

### Assessment
- Turns Caps Lock on/off; not a Graal cheat.
- IoCs: `a vendor site named in the binary`, `keybd_event`, Aspack sections (`.aspack`).
- **Doc-only**; ignore for game threat model except “Trainers folder is a mixed junk drawer.”

---

## Attack-surface map (Seihai / servertools lens)

```
                  ┌─────────────────────────────────────────┐
                  │  Operator / player Windows desktop       │
                  │  (untrusted "Trainers" EXEs)             │
                  └───────────────┬─────────────────────────┘
                                  │ keybd_event / SendMessage
                                  │ / AutoIt SEND / VB SendKeys
                                  ▼
                  ┌─────────────────────────────────────────┐
                  │  Classic Graal Win32 client (focus)      │
                  │  chat box · movement · item use          │
                  └───────────────┬─────────────────────────┘
                                  │ normal game protocol
                                  ▼
                  ┌─────────────────────────────────────────┐
                  │  gserver / playerworld                   │
                  │  ← AFK presence, farm loops, chat flood  │
                  └───────────────┬─────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          ▼                       ▼                       ▼
   Seihai clients          seihai-servertools        Listservers
   (not directly           (ops / detection          (accounts /
    driven by these         policy interested)        bans)
    macros today)
```

**Not in this corpus:** packet crafting against GServer, credential phishing kits, Seihai binary patching, supply-chain of Seihai crates, or web panel exploits.

---

## RE triage matrix

| Priority | Item | Why | Mode |
|--:|--|--|--|
| P0 | `Graal_Miner.zip` (both) | Google malware verdict; unknown payload | Full Ghidra/malware lab when obtainable |
| P1 | `Graal Prayer.exe` + `data/data.exe` | Dual-EXE layout; hidden second stage risk | Ghidra after UPX unpack (already unpacked under `_static_extract/upx_unpacked/`) |
| P1 | `Graal Digger.exe` | Corrupt UPX overlay + zip name mismatch | Ghidra / manual unpack |
| P2 | `GTS.exe` | `OpenProcess` + `GetAsyncKeyState` strings | Light Ghidra / VB decomp |
| P2 | `autoprogs` AutoD / AutoIdle / AutoS | Encoded AutoIt scripts; confirm payload = macros only | Ghidra or AutoIt script recovery |
| P3 | Remaining RpgWarrior AutoIt single EXEs | Same family as P2 | Batch script recovery; skip deep RE if scripts clean |
| P3 | `Graal Load.exe` | Simple script typer; known `bomb.txt` | Doc + optional light RE |
| — | AFK Bot readmes, howtos, `bomb.txt`, capslock readme | Fully explained in text | **Doc-only** |

---

## Top risks (summary ranking)

1. **Graal_Miner.zip** — Safe Browsing malware block; unreviewed binary.
2. **Graal Prayer dual EXE** — second PE in `data/` elevates packing/dropper suspicion.
3. **Graal Digger** — intentional or accidental corrupt packer stream; treat carefully.
4. **GTS (Graal Text Sender)** — process/open + key-state APIs; chat weaponization.
5. **AFK Bot / Scroll Bot + GraalLoad/bomb** — high operational impact on playerworlds (presence fraud + harassment) even though host risk looks lower.

---

## Method notes

- Prefer local `/workspace/recovered-hacks` once populated; Drive MCP used for early listing and blocked-file confirmation.
- Analysis: `file`, `sha256sum`, PE header parse, `strings`, `upx -d` on **copies**, archive listing (`unzip -l` / `7z l`), readme/howto text.
- **No execution** of recovered PE/AutoIt/VB binaries.
- Re-run this taxonomy if the Hacks Drive root gains folders beyond `Trainers/`.

---

## RE addendum (2026-10-31)

Headless Ghidra + static PE work on P1/P2 trainers is filed under:

- **Synthesis (IOCs / behavioral tells):** [`TRAINER_RE_NOTES.md`](./TRAINER_RE_NOTES.md)
- **Per-binary defense summaries:** [`ghidra-analysis/`](./ghidra-analysis/) — `Graal_Prayer/`, `Graal_Prayer_data/`, `Graal_Digger/`, `GTS/`, `AFK_Bot/`

**Confirmed statically:** Prayer/`data.exe` are twin AutoIt v2.64 UPX macros (no network / no classic injection imports; encrypted script overlays differ). GTS is VB6 chat spam with string-visible `OpenProcess` + `GetAsyncKeyState` + `FindWindow*`. AFK Bot is string-rich VB6 presence spam. **Digger remains blocked** by truncated zip/UPX (hash still usable; imports not recoverable). Miner still skipped.
