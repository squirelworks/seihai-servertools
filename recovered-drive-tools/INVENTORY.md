# Recovered Drive tools — full corpus inventory

Staging root: `/workspace/recovered-drive-tools/` (~2.8 GiB on disk after download + unpack).

Placement: [`SALVAGE_PLACEMENT.md`](./SALVAGE_PLACEMENT.md) · map: [`SEIHAI_SERVERTOOLS_MAP.md`](./SEIHAI_SERVERTOOLS_MAP.md) · candidates: [`SALVAGE_CANDIDATES.md`](./SALVAGE_CANDIDATES.md) · Ghidra: `ghidra-analysis/`

Refreshed: **2026-10-31** after the full Tools Drive download completed.

**Rules:** markdown-only under this folder in git; never commit recovered binaries; static analysis only — **never execute** recovered PEs; do not fetch Google-blocked abusive files; omit Drive URLs and folder/file IDs from commits; promote behaviour prose only (no decompiler symbol dumps or raw instruction addresses).

## Summary

| Metric | Count |
| --- | ---: |
| Catalog entries | **316** |
| Downloaded | **308** |
| Malware blocked (`cannotDownloadAbusiveFile`) | **7** |
| Unavailable (Drive 404) | **1** |
| Archives unpacked (`*.extracted/` siblings) | **289** |
| Extract failed | **1** |
| Skipped huge (>200 MiB) | **1** |

On-disk top level: `Developer Tools/` (GServer, RC, NC, Main Client Archive, Server Lister, LAT, NAT, GAT, Gani, Discord, Website), `Tools/`, `_extracted/`, `_inbox/`, `ghidra-analysis/`.

## Placement by tool class

| Class | Local area | Placement |
| --- | --- | --- |
| **GServer** | `Developer Tools/.../GServer/` | `analysis-only` → `seihai-gserver` / GPL `seihai-gserver-v2`; never paste into MIT |
| **Server Lister** | `.../Server Lister.rar` | `analysis-only` → `seihai-listserver`; Perl/status helpers → optional `setup-core` behaviour note |
| **RC** | `.../RC [Remote Control]/` | **`seihai-rc`** (pin by tag); `lat-core` holds RemoteFiles trait + `LocalFolder` only |
| **NC** | `.../NC [NPC Control]/` | **`seihai-rc`** / `seihai-npcserver` — analysis-only until clean-room |
| **LAT** | `Developer Tools/LAT Tool.../` | Offline editors/parsers → **`seihai`**; live world LAT → **`lat-core`** after shared drawing + RC |
| **NAT** / GS2 / script editors | `NAT.../`, `Tools/scripteditor*` | `analysis-only` (no Delphi/GS2 paste) |
| **GAT** / Gani / viewers / NW2PNG | GAT, Gani, `Tools/GraalViewer`, `Tools/NW2PNG` | **`seihai`** client/assets; most binaries analysis-only |
| Lab / config / process / lights | gserverstat notes, listserver scripts | **`setup-core`** |
| Main Client Archive / DClient / designers | Main Client Archive, `Tools/DClient*`, character tools | `out-of-scope` / client; DClient → `stay-staged (bad archive)` |
| Trainers / cracked / packet loggers | various | `out-of-scope` (`recovered-hacks/`) |
| Website / Discord / account PHP | Website, Discord, Account Creator | `out-of-scope` / ops |

## Access blockers

### Malware blocked — not downloaded

- `Account Manager.rar`
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2/Graal v2.310[unThinstall][unPeTite-Bass.dll][CRC-Fix].rar`
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v4/Graal v4.20r8[Windows][Patch Only].rar`
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v5/Graal v5.24r4[Windows][Installer].rar`
- `GServer Stats.zip`
- `Graal v2.3[Unpacked][Cracked].rar`
- `apcalc.exe`

### Unavailable (404)

- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/vGraalWorlds/WorldsSetup64-551.rar` — WorldsSetup64 build missing on Drive

### Extract / integrity notes

- `GAT.../PaintShopPro Files.rar` (~682 MiB) present but **not** auto-unpacked.
- `Tools/DClientSetup1198.zip` truncated at source (no EOCD, 242 431 B); partial `setup.exe` under `_extracted/` for static notes only.
- Sibling `WorldsSetup64-518.rar` **did** download (~87 MiB); only `-551` is 404.

## Top Ghidra candidates

Deduped high-value PE/ELF targets. Several already have `ghidra-analysis/<name>/SUMMARY.md` (gserver family, ListServer/serverlist3, BytecodeViewer, GCD/GChar, DClient setup, trainers).

| Theme | Extracted path | Size | Kind | Placement |
| --- | --- | ---: | --- | --- |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/GServersource.rar.extracted/gserver.exe` | 2.7 MiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/GServersource.rar.extracted/gserver` | 1.3 MiB | ELF | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/Matriark's GServer/gserver` | 1.3 MiB | ELF | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/Matriark's GServer/GServer.exe` | 772 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/Gserver1.39(win)(MOD)/gserver.exe` | 336 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/GserverGUI.exe` | 332 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/Matriark's GServer/dbsetup.exe` | 264 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar.extracted/GServer 1.39/Matriark's GServer/GStats.exe` | 48 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/gserver source.rar.extracted/gserver13` | 869 KiB | ELF | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v1/Graal v1.30.rar.extracted/GServer13.exe` | 568 KiB | PE | analysis-only → seihai-gserver |
| GServer | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v1/Graal v1.28.rar.extracted/graal128/GServerv.exe` | 556 KiB | PE | analysis-only → seihai-gserver |
| GServer stats | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/gserver source.rar.extracted/gserverstat` | 513 KiB | ELF | analysis-only; optional setup-core status note |
| GServer stats | `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/gserver source.rar.extracted/gserverstat2` | 513 KiB | ELF | analysis-only; optional setup-core status note |
| GServer | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/# Graal3D #/Graal3D.rar.extracted/graal3d/GServer.exe` | 491 KiB | PE | analysis-only → seihai-gserver |
| Listserver | `Developer Tools/# Graal Client Tools And Sources/Server Lister.rar.extracted/Server Lister/slist/serverlist3` | 3.3 MiB | ELF | analysis-only → seihai-listserver |
| Listserver | `Developer Tools/# Graal Client Tools And Sources/Server Lister.rar.extracted/Server Lister/ListServer.exe` | 256 KiB | PE | analysis-only → seihai-listserver |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/windows_rc3_beta.rar.extracted/RemoteControl3.exe` | 2.1 MiB | PE | seihai-rc |
| Other | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/windows_rc3_beta.rar.extracted/libcairo-script-interpreter-2.dll` | 179 KiB | PE | TBD |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Custom RemoteControl.rar.extracted/Custom RemoteControl/remotecontrol.exe` | 1.0 MiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc.rar.extracted/RemoteControl.exe` | 888 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc2.rar.extracted/RemoteControl.exe` | 720 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Graal v1.39 - RC.rar.extracted/RemoteControl.exe` | 592 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001 [V2].rar.extracted/nc7132001/RemoteControl-NW.exe` | 588 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New World/GraalNW1.rar.extracted/GraalNW/RemoteControl-NW.exe` | 578 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New Main/graalnm24.rar.extracted/graalnm24/RemoteControl-NewMain.exe` | 578 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v1/Graal v1.30.rar.extracted/RemoteControl13.exe` | 522 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v1/Graal v1.28.rar.extracted/graal128/RemoteControl1.28.exe` | 502 KiB | PE | seihai-rc |
| RC | `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Custom RemoteControl.rar.extracted/Custom RemoteControl/Agret_RC.exe` | 436 KiB | PE | seihai-rc |
| NC | `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001 [V2].rar.extracted/nc7132001/NPCControl.exe` | 471 KiB | PE | seihai-rc / seihai-npcserver |
| NC | `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001.rar.extracted/nc7132001.exe` | 471 KiB | PE | seihai-rc / seihai-npcserver |

### Ghidra already noted (this tree)

Prior batch: `BytecodeViewer/`, `DClient_setup/`, `GCD/`, `GChar/`, `Graal_Auto_Key/`, `Graal_Digger/`, `ListServer/`, `TextConvert/`, `gserver-linux/`, `gserver-win/`, `gserver13/`, `gserver_pe_GServersource/`, `gserverstat/`, `gserverstat2/`, `serverlist3/`.

Full-corpus follow-up: `RemoteControl3/`, `remotecontrol_custom/`, `RemoteControl-NW/`, `RemoteControl_classic/`, `NPCControl/`, `TilesEditor/`, `Assimilator/`, `PNGView/`, `scripteditor/`, `GraalLevelGenNW/`, `Matriark_gserver/`; scrubbed prose also for `ListServer/`, `TextConvert/`, gserver family, `serverlist3/`.

Still thin: additional GServer MOD/GUI builds, more LAT converters, GAT NW2PNG PE deep-dives.

## Corpus by category

Counts are catalog files (downloaded unless noted). Sizes are sum of downloaded catalog bytes (not unpacked trees).

| Category | Files | Bytes | Placement |
| --- | ---: | ---: | --- |
| GServer | 8 | 40.9 MiB | `analysis-only` → seihai-gserver / GPL fork |
| RC [Remote Control] | 15 | 16.8 MiB | `seihai-rc` (analysis → clean-room) |
| NC [NPC Control] | 5 | 520 KiB | `seihai-rc` / seihai-npcserver |
| Server Lister | 1 | 1.3 MiB | `analysis-only` → seihai-listserver; lab status → setup-core |
| Main Client Application Archive | 152 | 548.9 MiB | `out-of-scope` / seihai client archive |
| LAT [Level Administration] | 41 | 77.6 MiB | `seihai` offline · `lat-core` live later |
| NAT [NPC Administration] | 8 | 19.0 MiB | analysis-only (script tooling) |
| GAT [Graphics Administration] | 7 | 683.3 MiB | seihai assets / analysis-only |
| Gani Tools | 24 | 31.5 MiB | seihai (gani) / analysis-only |
| Tools/ | 39 | 13.4 MiB | mixed — see class table |
| Website Developer Tools | 3 | 755 KiB | out-of-scope / ops notes |
| Discord Developer Tools | 1 | 10 KiB | out-of-scope |
| Account Creator | 2 | 25 KiB | out-of-scope / ops |
| Client misc | 2 | 298 KiB | analysis-only / TBD |

### Key packages (representative)

#### GServer

- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.39.rar` — 23.0 MiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/!Gserver Locations Unsure.rar` — 11.9 MiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.31.rar` — 2.3 MiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/Graal New World GServer.rar` — 1.5 MiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/GServersource.rar` — 1.3 MiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer Source/gserver source.rar` — 494 KiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.34.rar` — 231 KiB
- `Developer Tools/# Graal Client Tools And Sources/GServer/GServer 1.19.rar` — 92 KiB

#### RC [Remote Control]

- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/windows_rc3_beta.rar` — 6.1 MiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/RC 2.04.rar` — 3.7 MiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Custom RemoteControl.rar` — 3.6 MiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc-2010-linux.tgz` — 573 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc.rar` — 343 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/RC_20100329.rar` — 342 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc2.rar` — 296 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc2 (2).rar` — 296 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc (1).tgz` — 263 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc8212001.rar` — 226 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/Graal v1.39 - RC.rar` — 225 KiB
- `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/rc62101.rar` — 225 KiB
- … +3 more

#### NC [NPC Control]

- `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001 [V2].rar` — 300 KiB
- `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/nc7132001.rar` — 188 KiB
- `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/NPCControl13.rar` — 15 KiB
- `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/NPCControl1.3.rar` — 12 KiB
- `Developer Tools/# Graal Client Tools And Sources/NC [NPC Control]/NPC Server Document [Graal2001].rar` — 5 KiB

#### Server Lister / account misc

- `Developer Tools/# Graal Client Tools And Sources/Server Lister.rar` — 1.3 MiB
- `Developer Tools/# Graal Client Tools And Sources/## MISC ##/GraalRebornSecureLogin-master.rar` — 297 KiB
- `Developer Tools/# Graal Client Tools And Sources/Graal Account Creator [Website]/accountcreatorphp.rar` — 13 KiB
- `Developer Tools/# Graal Client Tools And Sources/Graal Account Creator [Website]/accountcreator.rar` — 12 KiB
- `Developer Tools/# Graal Client Tools And Sources/Graal Client Release Dates.rar` — 1 KiB

#### Main Client Archive (largest)

- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/vJA - Java Graal/JavaGraal-master.rar` — 91.1 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/vGraalWorlds/WorldsSetup64-518.rar` — 86.7 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v5/Graalv5.rar` — 30.5 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v4/Graal v4.04r2[Mac OS X].dmg` — 22.0 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v5/Graal v5.00r7[Windows][Unpacked][CRC Patch].rar` — 21.5 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v3/Graal v3.04[Mac OS X].dmg.gz` — 13.5 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v3/Graal v3.15r2[Mac OS X].dmg.gz` — 13.5 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New World/GraalNW1.rar` — 10.8 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v3/Graal v3.15r1[Linux][Broken].sh` — 8.8 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v4/Graal v4.20r6[Mac OS X].dmg.bz2` — 8.2 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v4/Graal4.dmg.bz2` — 8.1 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New World/GraalNWleak.rar` — 7.4 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v2 - New World/GraalNW2.rar` — 7.1 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v3/Graal v3.04[Linux][Fixed].tgz` — 6.3 MiB
- `Developer Tools/# Graal Client Tools And Sources/Main Client Application Archive/v3/Graal v3.04[Linux][Obsolete].sh` — 6.3 MiB
- … +137 more

#### LAT

- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/TilesEditor-1.2.rar` — 14.4 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/TilesEditor-lukegrahamSydney.rar` — 13.9 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/Tiled-1.2.4-win64.rar` — 13.9 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/TilesEditorrelease.rar` — 9.5 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/TilesEditor [Luke Graham - 39ster]/Graal Editor.rar` — 9.5 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/Graal Suits [Dusty].rar` — 6.8 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Images - Tutorials - Paint Shop Pro/Graal_Cliff_Tutorial_Fix Firsr_WEBPAGESAVED.rar` — 2.0 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/MAP Tools/NW2PNG2_Source_Files.rar` — 1.4 MiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Images - Tutorials - Paint Shop Pro/Level Linking - Tutorial.rar` — 559 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Images - Tutorials - Paint Shop Pro/pics1 [PSD FILE].rar` — 549 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Converters/NW2TMX-master.rar` — 518 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Generators/GraalOnline Dungeon Generator - NW -  [2003-05-23].rar` — 450 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/OpenGraal.LevelEditor-cs-master.rar` — 446 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/Gonstruct-master.rar` — 369 KiB
- `Developer Tools/LAT Tool - [Level Administration Team]/Level Editors/GraalEditorW10.rar` — 350 KiB
- … +26 more

#### NAT / GS2

- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/NPCs.rar` — 12.4 MiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Editors/scripteditor253 - (Has player added files).rar` — 2.7 MiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Editors/scripteditor253.rar` — 1.8 MiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/GS2 Compiler.rar` — 1.8 MiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Editors/BinaryCrash Script Eingine.rar` — 172 KiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Editors/gs2-beautify-master.rar` — 86 KiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Syntax.rar` — 73 KiB
- `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Functions/scriptfunctions_client_6_0_0_7.rar` — 22 KiB

#### GAT / Gani

- `Developer Tools/GAT Tool - [Graphics Administration Team]/PaintShopPro Files.rar` — 682.0 MiB
- `Developer Tools/Gani Tools [Graal Animation]/Gani Tutorial/Ganis Tutorial Video.rar` — 24.7 MiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GViewer [Web].rar` — 1004 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Gani Tutorial/Ganis Tutorial by Flame .[Document].rar` — 798 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Gani Tutorial/Gani Tutorial [HTML].rar` — 644 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GraalViewer2.1.rar` — 639 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GraalViewerRev5 [EXE].rar` — 592 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GraalViewerRev2 [EXE].rar` — 591 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GraalViewer2 [EXE].rar` — 591 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Graal Animation Tools/GraalAnimViewer2 [EXE].rar` — 589 KiB
- `Developer Tools/GAT Tool - [Graphics Administration Team]/NW2PNG.rar` — 531 KiB
- `Developer Tools/Gani Tools [Graal Animation]/Gani Tutorial/Ganis Tutorial - Fulg0reSama[Document].rar` — 423 KiB
- … +19 more

#### Tools/

- `Tools/scripteditor253.zip` — 2.3 MiB
- `Tools/scripteditor253.7z` — 1.7 MiB
- `Tools/GraalViewer/GraalViewer2_src.zip` — 1.4 MiB
- `Tools/GraalViewer/GraalViewerRev5.zip` — 671 KiB
- `Tools/GraalViewer/GraalViewerRev2.zip` — 670 KiB
- `Tools/GraalViewer/GraalViewer2.zip` — 670 KiB
- `Tools/GraalAnimViewer2.zip` — 664 KiB
- `Tools/TerrainGen.zip` — 499 KiB
- `Tools/levelfiller.7z` — 477 KiB
- `Tools/PNGView.rar` — 322 KiB
- `Tools/Graal Messenger.7z` — 320 KiB
- `Tools/graalshop.zip` — 315 KiB
- `Tools/Greaditor.7z` — 291 KiB
- `Tools/NW2PNG/NW2PNG2.0.zip` — 242 KiB
- `Tools/DClientSetup1198.zip` — 237 KiB
- `Tools/guildcode.zip` — 230 KiB
- `Tools/Graal Character Generator 1.0.zip` — 229 KiB
- `Tools/GraalLevelGen.zip` — 221 KiB
- `Tools/levelgenerator.zip` — 221 KiB
- `Tools/NW2PNG/NW2PNG 1.3r1.zip` — 219 KiB
- … +19 more

## Prior Ghidra follow-up archives (still valid)

| ID | Path | Status | Placement | Ghidra |
| --- | --- | --- | --- | --- |
| DClientSetup1198 | `Tools/DClientSetup1198.zip` | incomplete EOCD | `stay-staged (bad archive)` | `ghidra-analysis/DClient_setup/` |
| Graal_Auto_Key | `_inbox/Graal_Auto_Key.zip` | complete | `out-of-scope` | `Graal_Auto_Key/` |
| Graal_Digger | `_inbox/Graal_Digger.zip` | PE truncated/corrupt UPX | `out-of-scope` | `Graal_Digger/` |
| GCD 1.2.7 | `Tools/Graal Character Designer 1.2.7.zip` | complete | `out-of-scope / seihai` | `GCD/` |
| charactergenerator | `Tools/charactergenerator.7z` | complete | `out-of-scope / seihai` | `GChar/` |
| Server_Lister | `.../Server Lister.rar` | complete | `analysis-only` | `ListServer/`, `serverlist3/` |
| GS2_Compiler | `.../GS2 Compiler.rar` | complete | `analysis-only` | `BytecodeViewer/` |
| gserver_source | `.../gserver source.rar` | complete | `analysis-only` | `gserver13/`, `gserverstat/`, `gserverstat2/` |
| GServersource | `.../GServersource.rar` | complete | `analysis-only` | `gserver_pe_GServersource/`, `gserver-linux/`, `gserver-win/` |

## Clean-room rewrite

See [`SALVAGE_CANDIDATES.md`](./SALVAGE_CANDIDATES.md). Promote only when behaviour is understood, target crate is clear, and no raw/GPL text would land in an MIT tree.
