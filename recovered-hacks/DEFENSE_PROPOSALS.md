# Defense proposals — recovered Graal hacks / trainers

**Date:** 2026-10-31 (third pass; inventory + taxonomy + Ghidra RE)  
**Audience:** operators of Seihai-stack worlds (`seihai-gserver` / `seihai-gserver-v2`, `seihai-listserver`, staff RC/NC) and maintainers of `seihai-servertools`.  
**Method:** static analysis only (Drive listing, archive inventory, readmes, PE/UPX/strings, Ghidra headless). No hacks executed; no exploit PoCs.  
**Companions:** [`THREAT_TAXONOMY.md`](THREAT_TAXONOMY.md) (Groups 1–5 + RE addendum) · [`defense-checklist.md`](defense-checklist.md) · [`INVENTORY.md`](INVENTORY.md) · [`TRAINER_RE_NOTES.md`](TRAINER_RE_NOTES.md) · [`ghidra-analysis/`](ghidra-analysis/)

**Intended long-term home:** mirror under `seihai-servertools/docs/security/` (local draft prepared; that repo returned 403 to this environment’s push token).

## Confidence and gaps

| Item | Level | Notes |
|---|---|---|
| Overall threat picture for Drive `Hacks/Trainers` | **High** | `INVENTORY.md` + `THREAT_TAXONOMY.md`: 18/20 archives, 24 unique PEs, readmes + VB6/AutoIt strings agree |
| Taxonomy Groups 1–3, 5 | **High** | AFK bots, AutoIt macros, chat/appearance spam, capslock |
| Group 4 (`Graal_Miner.zip`) | **Unknown payload** | Drive-blocked; treat hostile until RE — **skipped this pass** |
| Prayer / `data.exe` / GTS / AFK Bot (static RE) | **High** | `TRAINER_RE_NOTES.md` + `ghidra-analysis/*/SUMMARY.md` |
| Digger (truncated UPX) | **Medium** | Hash + AutoIt v2.63 cues; imports/script not recoverable |
| Version faker / CRC / packet logger (adjacent archives) | **Medium** | Not in this Hacks tree |
| Classic login / RC / stealer ListServer | **High** | Seihai docs + `Graal_analysis` |
| Network injectors / memory editors beyond these archives | **Unknown** | Not in Hacks listing |

**Sources used**

- Owner private Drive folder `Hacks` → `Trainers/` (+ `AutoPrograms by RpgWarrior/`) — re-listed 2026-10-31 (Drive URL omitted)
- [`INVENTORY.md`](INVENTORY.md) / local static extracts (UPX AutoIt `SEND`/`keybd_event`; VB6 AFK/GTS/GraalLoad strings)
- [`THREAT_TAXONOMY.md`](THREAT_TAXONOMY.md) Groups 1–5 + RE addendum
- [`TRAINER_RE_NOTES.md`](TRAINER_RE_NOTES.md) — Ghidra/static IOC synthesis
- Adjacent client archives: v4 packet-logger, v5 CRC / “Version 5.120 Faker Hack”
- `seihai-servertools` go-public / `setup-core` checks; `AGENTS.md` stealer ListServer warning

---

## Threat summary (operator view)

1. **Group 1 — AFK chat bots** — VB6 timers into Graal chat (Zorg AFK / Scroll). Normal chat packets, unnatural periodicity (~3s).
2. **Group 2 — RpgWarrior / AutoIt gameplay macros** — Water/Dig/Pray/Walk/Key + autoprogs; AutoIt `SEND` loops.
3. **Group 3 — Chat / appearance script spam** — GTS scrollers; GraalLoad + `bomb.txt` via `SendMessageA`.
4. **Group 4 — Suspected malware (Miner)** — Drive `cannotDownloadAbusiveFile`; host risk unknown until obtained.
5. **Group 5 — Benign / misfiled** — `capslock.exe` (ASPack CapsLock toggler).
6. **Stack-adjacent (outside Hacks/)** — version faker / packet logger; cleartext classic passwords; stealer ListServer; `localhost=true`; open gserver registration; hostile world content.

---

## Technique cards

### T1 — AFK / chat bots (Group 1; also GTS in Group 3, autoprogs idle in Group 2)

**Evidence**  
AFK Bot readme + RE strings: default phrases `I am AFK` / `I am using an AFK Bot`, UI `AFK Bot created by Zorg`, warning about illegality on playerworlds; ~3s timer after focusing chat. Scroll Bot: growing/scrolling string. GTS (see T3): Word Scroller / letter-by-letter shouts (`MSVBVM60.DLL`). Autoprogs share UPX/AutoIt markers with Group 2. **No Winsock** on AFK Bot static surface — client-side chat abuse only.

**How it works (high level)**  
External Windows programs inject keystrokes into the focused Graal chat field on a timer to farm online-time / stay “active”.

**Detection** — fixed inter-arrival chat; phrase/scroll patterns (`I am AFK` cluster); online-time with near-zero movement/combat. Host: `AFK Bot.exe` + Zorg UI strings.  
**Prevention** — messages/min caps; mute→kick; AFK rewards require movement/interaction, never chat alone; NPC scripts must not trust client timers.  
**Response** — mute/warn/kick; snapshot chat timestamps; rebalance farmed NPC rewards.  
**seihai-servertools** — policy docs here; optional offline periodic-chat log parsers later; enforcement in gserver.

### T2 — Activity automation (Group 2; Group 4 Miner blocked)

**Evidence**  
Howtos: equip → stand on resource → start EXE → focus Graal. **RE (Prayer / `data.exe`):** twin AutoIt v2.64 UPX macros — APIs `keybd_event`, `mouse_event`, `SendMessageA`, `PostMessageA`, `FindWindowA`, `OpenProcess`, `VirtualAlloc`; **no** `WSAStartup` / WinINet / Winsock and **no** classic injection (`WriteProcessMemory` / `CreateRemoteThread`). Encrypted AutoIt overlays differ (~924 B vs ~180 B) — keep both packed + unpacked hashes. **Digger:** truncated zip PE (36111 / 41893 bytes); UPX overlay corrupt (`upx` refuses unpack); Ghidra import table empty; still classifiable as AutoIt v2.63 + UPX via strings + howto — hash `876723732378b36e…` usable, behaviour not recoverable until complete PE. Miner ×2 Drive-blocked (~39156 bytes) — **skipped**; taxonomy keeps Group 4 host-risk open.

**Detection** — sustained interact/dig rates; low movement entropy at resource tiles; single-action-type sessions; shared IP timing fingerprints. Host: UPX + `Compiled AutoIt v2.6` + `>AUTOIT SCRIPT<` + `keybd_event` (no network imports).  
**Prevention** — server-authoritative gather/prayer/dig/water cooldowns; yield caps per account/IP; soft anti-macro (facing/inventory/movement between gathers).  
**Response** — soft yield nerf; hard kick/temp ban; wipe ill-gotten stacks if needed.  
**seihai-servertools** — `lat-core/check.rs` lint for unbounded resource scripts; go-public warn on bot-farm lab leftovers.

### T3 — Chat / appearance spam (Group 3: GTS + GraalLoad + `bomb.txt`)

**Evidence**  
**GTS RE:** VB6 DarkShadowz **Graal Text Sender v1.3.5**; late-bound `FindWindowA`/`Ex`, `OpenProcess`, `GetAsyncKeyState`, `SendMessageA`, module `SendKeys1`; timer UI (Word Scroller / shout / to-all); **no Winsock**. Host combo IOC: VB6 + those APIs + product strings and the author's site. GraalLoad: `load<file.txt>`, `SendMessageA`. `bomb.txt`: setnick/setbody + countdown chat (not an explosive).

**Detection** — sub-human nick/appearance/chat bursts; prop churn without movement; GTS flood modes (scroller / to-all / shout). Host: window title `Graal Text Sender`.  
**Prevention** — nick/appearance rate limits; T1 chat flood controls; rate-limit shout/to-all.  
**Response** — mute; revert nick/appearance; escalate harassment.

### T4 — Version fakery + CRC patch (stack-adjacent)

**Evidence** — named v5 CRC / “Version 5.120 Faker” archives outside this Hacks folder.

**Detection** — version token vs behaviour mismatch; prefer `SEI00001` secure login on Seihai-only worlds.  
**Prevention** — **do not trust version string alone**; behaviour allowlists; optional capability challenge.  
**Response** — generic “outdated or modified client” disconnect; ban repeats.

### T5 — Packet logger / forged packets (stack-adjacent)

**Evidence** — v4 “+Packet Logger” client archives.

**Prevention (highest priority for a real gserver)**

| Control | Where |
|---|---|
| Server-authoritative movement / inventory / chests / combat | gserver (+ NPC server) |
| Reject RC/NC-only message ids on player sessions | gserver |
| Unknown-id budget, max frame size, max messages/sec | gserver |
| No default `super`/`super`; separate RC/NC auth | gserver + servertools checks |
| Secure login `SEI00001` | gserver + listserver |
| No secrets in client-visible scripts | LAT / world content |

**seihai-servertools** — extend public-profile checks (lab accounts, default staff passwords; `localhost=true` Block already exists); LAT denylist uploads; **never run classic ListServer.exe**.

### T6 — Credential theft & unsafe hosting (stack-adjacent)

Cleartext classic login; 2003 “Rick” ListServer stealer (`AGENTS.md`); `localhost=true` public bind; open gserver registration; lab accounts left public.

**Prevention** — hashed listserver accounts; firewall gserver registration port; pin `SEI00001` key; wipe lab accounts on publish.  
**Response** — rotate secrets; if stealer ListServer ran on admin PC, burn all saved Graal client passwords.

### T7 — Hostile world content (LAT)

Levels/NPCs that escalate rights when scripts are trusted. Filename ≠ proof.

**Prevention** — `seihai-servertools check`; GS1 lints; dual-control RC `w` uploads; put/delete logs.

---

## Priority patch / build order

1. Keep/extend go-public **Block**s (`setup-core/checks.rs`).
2. Gserver chat + gather rate limits / cooldowns (defeats Groups 1–2 / T1–T3).
3. Gserver movement / inventory / chest authority (defeats T5).
4. Listserver gserver allow-list + hashed accounts.
5. Offer `SEI00001` secure login; document tradeoff.
6. LAT world linter in `lat-core/check.rs`.
7. Ops runbooks — this doc set; mirror to `seihai-servertools/docs/security/` when writable.
8. Log queries for periodic chat, gather loops, version mismatches.

## Concrete controls mapped to `seihai-servertools`

| Defense | Where |
|---|---|
| Unsafe public bind / localhost / UPnP | `setup-core` checks (**exists**) |
| Host-listserver open accounts on outside bind | listserver checks (**exists**) |
| Lab leftovers / default accounts | `check_server_folder` |
| Operator hardening + checklist | this folder (and `docs/security/` in servertools) |
| World script / link / gmap safety | `lat-core/check.rs` |
| Packet/rate policy | **gserver** (not servertools) |

**Do not** commit trainer binaries, packet loggers, or CRC patches into MIT crates. Keep under `recovered-hacks/` (gitignored except `*.md`).

## Suggested monitoring signals

| Signal | Suggest alert |
|---|---|
| Chat msgs / account / min | > 20 sustained 5+ min |
| Identical message repeats | ≥ 10 in 5 min |
| Interact/use / min at fixed tile | > 40 for 10+ min |
| Speed > configured max | any |
| Unknown message ids | > 3 / session |
| Version token ∉ allowlist | info; kick if strict |
| RC login from non-staff IP | page owner |
| New gserver registration | page listserver host |
| Failed passwords from many IPs | lockout |

## Key host / string IOCs (from RE)

| Family | IOC cluster |
|---|---|
| Prayer / `data.exe` | UPX + AutoIt v2.64; `keybd_event` / `mouse_event` / `SendMessageA`; **no Winsock**; dual packed+unpacked SHA-256 in `TRAINER_RE_NOTES.md` |
| Digger | Truncated UPX PE; AutoIt v2.63 splash fragment; hash only until complete sample |
| GTS | VB6 + `FindWindow*` + `OpenProcess` + `SendMessageA` + `GetAsyncKeyState`; title `Graal Text Sender` |
| AFK Bot | Strings `I am AFK` / `I am using an AFK Bot` / Zorg UI; VB6 + `Timer1` |

Full tables: [`TRAINER_RE_NOTES.md`](TRAINER_RE_NOTES.md). Per-binary: [`ghidra-analysis/*/SUMMARY.md`](ghidra-analysis/).

## Residual gaps (Ghidra / RE siblings)

| Priority | Gap |
|---|---|
| P0 | Obtain `Graal_Miner.zip` (`acknowledgeAbuse` / mirror) + malware triage — **still skipped** |
| P1 | Prayer / `data.exe` AutoIt **script body** recovery (overlays still encrypted) |
| P1 | Digger truncated UPX — clean mirror or complete PE |
| P2 | ~~GTS `OpenProcess` / `GetAsyncKeyState`~~ — **done:** SendMessage/hotkey automation, not proven inject |
| P2 | autoprogs script recovery — macros only? |
| P3 | Remaining RpgWarrior SEND loops; GraalLoad confirm |
| Adjacent | Version-faker / packet-logger disassembly |

Inventory + taxonomy + RE notes are enough to ship operator defenses **now**; remaining RE refines AutoIt script plaintext and Group 4 host-risk, not the patch order above.
