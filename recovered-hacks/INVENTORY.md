# Recovered Hacks Inventory

**Source:** Owner private Google Drive folder named `Hacks` (URL and folder id omitted from this repo).
**Listed at:** 2026-10-31T20:49:06.899885+00:00
**Handling:** Contents treated as untrusted. Download agent did not execute binaries/scripts; static inspection only. Sibling agents may have added _static_extract/ artifacts.

## Priority analysis targets

Deep-analysis queue for sibling agents (static only; do not execute unknowns):

1. **Trainers/Graal_Miner.zip (+ AutoPrograms duplicate)** _blocked_download_
   - Google Drive hard-blocks with cannotDownloadAbusiveFile. Obtain via acknowledgeAbuse=true or owner “Download anyway” for malware triage.

2. **Trainers/extracted/capslock/capslock.exe** `2576cbeacd5ed30f…`
   - Largest unique PE (10 sections, non-UPX). Custom loader stub (VirtualAlloc/GetProcAddress). Check packing and any input-capture beyond CapsLock macros.

3. **Trainers/extracted/Graal_Prayer/data/data.exe** `c331eee0e985c903…`
   - UPX-packed sidecar; AutoIt compiled-script strings. Secondary helper — see _static_extract/upx_unpacked and autoit_scripts.

4. **Trainers/extracted/GraalLoad/Graal Load.exe + bomb.txt** `812a0d32da379e75…`
   - VB6 window-message automation (FindWindow/SendMessage). bomb.txt is a Graal appearance/chat theatrical script, not an explosive payload.

5. **Trainers/extracted/GTS/GTS.exe** `36e264ff1b9bd441…`
   - VB6 “Graal Text Sender” chat automation (scrollers/shouts).

6. **Trainers/extracted/autoprogs/{AutoIdle,AutoD,AutoS}.exe**
   - UPX 1.07 family with AutoIt indicators. Sibling unpacks already under _static_extract/upx_unpacked/.

7. **RpgWarrior UPX trainers (Auto Key, Digger, Prayer, Random Walker, Water Get)**
   - Classic Graal automation. Digger PE truncated from corrupt zip. AutoPrograms copies are byte-identical archives.

8. **AFKBOT / AFKSCROLLBOT (Zorg)**
   - Unpacked PE32 chat-AFK bots with author readmes. Confirm SendKeys-style vs injection.

## Summary

- Drive files listed: **20**
- Downloaded OK: **18**
- Blocked by Google: **2**
- Archives extracted: **18** (originals preserved)
- Extracted member files: **30**
- Unique primary PE binaries: **15** (22 extracted instances incl. duplicates)
- Sibling `_static_extract/` artifacts: **53**

## On-disk vs git

Binaries and archives remain on the agent workspace under `recovered-hacks/Trainers/` for sibling static analysis. Repo `.gitignore` tracks analysis docs/metadata only (`*.md`, `*.json`, `*.BLOCKED.txt`), not PE/archives.

## Blockers

- `Trainers/Graal_Miner.zip` — **403 cannotDownloadAbusiveFile — Google identified as malware or spam** (Drive file id omitted; expected 39156 bytes)
- `Trainers/AutoPrograms by RpgWarrior/Graal_Miner.zip` — **403 cannotDownloadAbusiveFile — Google identified as malware or spam** (Drive file id omitted; expected 39156 bytes)
  - Same reported size as Trainers/Graal_Miner.zip; likely identical duplicate

Owner workaround: Drive API download with `acknowledgeAbuse=true`, or Drive UI “Download anyway”.

## Key paths

```
recovered-hacks/
├── INVENTORY.md / inventory.json
├── DEFENSE_PROPOSALS.md / defense-checklist.md   # sibling notes
├── _static_extract/                              # sibling UPX/strings/AutoIt extracts
└── Trainers/
    ├── *.zip *.7z                                # Drive originals (18)
    ├── Graal_Miner.zip.BLOCKED.txt
    ├── extracted/                                # unpacked siblings
    └── AutoPrograms by RpgWarrior/
        ├── *.zip + Graal_Miner.zip.BLOCKED.txt
        └── extracted/
```

## Observations

- Collection is vintage Graal Online trainers/bots (timestamps ~2002–2017), not general exploit kits.
- Many PEs are UPX-compressed; capslock.exe is a multi-section custom loader.
- GraalLoad (VB6) loads client script .txt files; bomb.txt is theatrical chat/appearance scripting.
- GTS = Graal Text Sender (VB6 chat macros).
- Prayer data/data.exe shows AutoIt compiled-script strings; sibling unpacks present.
- Graal_Digger.zip is damaged; recovered PE is truncated (CRC fail).
- AutoPrograms by RpgWarrior mirrors Trainers archives (byte-identical where both present).
- Sibling analysis artifacts live under _static_extract/ (UPX unpacks, strings, AutoIt script extracts) plus DEFENSE_PROPOSALS.md / defense-checklist.md.

## Duplicate archives (analyze once)

- `Trainers/Graal_Auto_Key.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Auto_Key.zip`
- `Trainers/Graal_Digger.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Digger.zip`
- `Trainers/Graal_Prayer.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Prayer.zip`
- `Trainers/Graal_Random_Walker.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Random_Walker.zip`
- `Trainers/Graal_Water_Get.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Water_Get_v1.0.zip`
- `Trainers/Graal_WaterGet.zip` ≡ `Trainers/AutoPrograms by RpgWarrior/Graal_Water_Get_v1.1.zip`

## Unique primary PE binaries

| SHA256 (16) | Size | file(1) | Paths |
|---|---:|---|---|
| `463e8f757e82bc5a…` | 42793 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Auto_Key/Graal Auto Key.exe`<br>`Trainers/extracted/Graal_Auto_Key/Graal Auto Key.exe` |
| `876723732378b36e…` | 36111 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Digger/Graal Digger.exe`<br>`Trainers/extracted/Graal_Digger/Graal Digger.exe` |
| `6959bcb8066efdee…` | 43420 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Prayer/Graal Prayer.exe`<br>`Trainers/extracted/Graal_Prayer/Graal Prayer.exe` |
| `c331eee0e985c903…` | 42676 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Prayer/data/data.exe`<br>`Trainers/extracted/Graal_Prayer/data/data.exe` |
| `b41973d19fc0089c…` | 43185 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Random_Walker/Graal Random Walker.exe`<br>`Trainers/extracted/Graal_Random_Walker/Graal Random Walker.exe` |
| `4f95fa2af7d7db1c…` | 43483 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Water_Get_v1.0/Graal Water Get.exe`<br>`Trainers/extracted/Graal_Water_Get/Graal Water Get.exe` |
| `7482de4824d370f1…` | 41820 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Water_Get_v1.1/Graal WaterGet.exe`<br>`Trainers/extracted/Graal_WaterGet/Graal WaterGet.exe` |
| `35dbd87ca7560712…` | 24576 | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections | `Trainers/extracted/AFKBOT/AFKBOT/AFK Bot.exe` |
| `fdab4020b62e1ef4…` | 24576 | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections | `Trainers/extracted/AFKSCROLLBOT/AFKSCROLLBOT/AFK Scroll Bot.exe` |
| `36e264ff1b9bd441…` | 40960 | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections | `Trainers/extracted/GTS/GTS.exe` |
| `812a0d32da379e75…` | 36864 | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections | `Trainers/extracted/GraalLoad/Graal Load.exe` |
| `5a4493d79d0619a9…` | 41317 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/extracted/autoprogs/AutoD.EXE` |
| `ebf4a5913132fa8f…` | 42041 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/extracted/autoprogs/AutoIdle.exe` |
| `7e427085717c47e2…` | 41072 | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections | `Trainers/extracted/autoprogs/AutoS.exe` |
| `2576cbeacd5ed30f…` | 130048 | PE32 executable (GUI) Intel 80386, for MS Windows, 10 sections | `Trainers/extracted/capslock/capslock.exe` |

## Drive originals

Drive file ids omitted (private owner folder). Local paths + sizes + download status retained.

| Path | Size | Status | SHA256 |
|---|---:|---|---|
| `Trainers/AFKBOT.7z` | 5001 | OK | `f7ac393a1d6415c1…` |
| `Trainers/AFKSCROLLBOT.7z` | 5668 | OK | `bd46ee159f3ecaef…` |
| `Trainers/autoprogs.zip` | 111200 | OK | `0cfbfffde3b7ed08…` |
| `Trainers/capslock.zip` | 126297 | OK | `4c65b83d528c8d37…` |
| `Trainers/Graal_Auto_Key.zip` | 38024 | OK | `827d9b5549da0308…` |
| `Trainers/Graal_Digger.zip` | 39297 | OK | `27e051d29205b797…` |
| `Trainers/Graal_Miner.zip` | 39156 | BLOCKED | — |
| `Trainers/Graal_Prayer.zip` | 76631 | OK | `9c86172af8f52755…` |
| `Trainers/Graal_Random_Walker.zip` | 38442 | OK | `456991c2269be852…` |
| `Trainers/Graal_Water_Get.zip` | 38746 | OK | `8209e0ff526a55bd…` |
| `Trainers/Graal_WaterGet.zip` | 37327 | OK | `642e9849f5736ea9…` |
| `Trainers/GraalLoad.zip` | 18438 | OK | `ef05589392c3f77f…` |
| `Trainers/GTS.zip` | 8763 | OK | `2b0d01bade67865f…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Auto_Key.zip` | 38024 | OK | `827d9b5549da0308…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Digger.zip` | 39297 | OK | `27e051d29205b797…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Miner.zip` | 39156 | BLOCKED | — |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Prayer.zip` | 76631 | OK | `9c86172af8f52755…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Random_Walker.zip` | 38442 | OK | `456991c2269be852…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Water_Get_v1.0.zip` | 38746 | OK | `8209e0ff526a55bd…` |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Water_Get_v1.1.zip` | 37327 | OK | `642e9849f5736ea9…` |

## Extracted + notes inventory

| Path | Size | MIME | Category | SHA256 | file(1) |
|---|---:|---|---|---|---|
| `DEFENSE_PROPOSALS.md` | 16820 | `text/markdown` | defense_notes | `22cc5d5c07f47bef…` | Unicode text, UTF-8 text |
| `THREAT_TAXONOMY.md` | 16396 | `text/markdown` | writeup | `5a0b21e4e908d2e0…` | Unicode text, UTF-8 text, with very long lines (320) |
| `Trainers/AutoPrograms by RpgWarrior/Graal_Miner.zip.BLOCKED.txt` | 371 | `text/plain` | blocked_placeholder | `b5ffa6b1d187bd35…` | Unicode text, UTF-8 text |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Auto_Key/Graal Auto Key.exe` | 42793 | `application/vnd.microsoft.portable-executable` | trainer_binary | `463e8f757e82bc5a…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Digger/Graal Digger.exe` | 36111 | `application/vnd.microsoft.portable-executable` | trainer_binary | `876723732378b36e…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections ⚠️ Carved from corrupt zip; CRC fail; decompressed 36111 vs declared 41893 (truncated) |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Digger/Graal_Digger-Howto.txt` | 219 | `text/plain` | writeup | `2166abaa7cc1f196…` | ASCII text, with CRLF line terminators |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Prayer/Graal Prayer.exe` | 43420 | `application/vnd.microsoft.portable-executable` | trainer_binary | `6959bcb8066efdee…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Prayer/data/data.exe` | 42676 | `application/vnd.microsoft.portable-executable` | trainer_binary | `c331eee0e985c903…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Random_Walker/Graal Random Walker.exe` | 43185 | `application/vnd.microsoft.portable-executable` | trainer_binary | `b41973d19fc0089c…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Water_Get_v1.0/Graal Water Get.exe` | 43483 | `application/vnd.microsoft.portable-executable` | trainer_binary | `4f95fa2af7d7db1c…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Water_Get_v1.1/Graal WaterGet.exe` | 41820 | `application/vnd.microsoft.portable-executable` | trainer_binary | `7482de4824d370f1…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/AutoPrograms by RpgWarrior/extracted/Graal_Water_Get_v1.1/Graal_WaterGet-Howto.txt` | 224 | `text/plain` | writeup | `7c11e4e42123e834…` | ASCII text, with CRLF line terminators |
| `Trainers/Graal_Miner.zip.BLOCKED.txt` | 274 | `text/plain` | blocked_placeholder | `f0372ca8e6db422b…` | Unicode text, UTF-8 text |
| `Trainers/extracted/AFKBOT/AFKBOT/AFK Bot Read Me.txt` | 1007 | `text/plain` | writeup | `2fbd034251d6fc81…` | ASCII text, with CRLF line terminators |
| `Trainers/extracted/AFKBOT/AFKBOT/AFK Bot.exe` | 24576 | `application/vnd.microsoft.portable-executable` | trainer_binary | `35dbd87ca7560712…` | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections |
| `Trainers/extracted/AFKSCROLLBOT/AFKSCROLLBOT/AFK Scroll Bot Read Me.txt` | 1320 | `text/plain` | writeup | `d2cbb74e05ab07b7…` | ASCII text, with CRLF line terminators |
| `Trainers/extracted/AFKSCROLLBOT/AFKSCROLLBOT/AFK Scroll Bot.exe` | 24576 | `application/vnd.microsoft.portable-executable` | trainer_binary | `fdab4020b62e1ef4…` | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections |
| `Trainers/extracted/GTS/GTS.exe` | 40960 | `application/vnd.microsoft.portable-executable` | trainer_binary | `36e264ff1b9bd441…` | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections |
| `Trainers/extracted/GraalLoad/Graal Load.exe` | 36864 | `application/vnd.microsoft.portable-executable` | trainer_binary | `812a0d32da379e75…` | PE32 executable (GUI) Intel 80386, for MS Windows, 3 sections |
| `Trainers/extracted/GraalLoad/bomb.txt` | 1372 | `text/plain` | graal_client_script | `332cb8718c0f6098…` | ASCII text, with CRLF line terminators |
| `Trainers/extracted/Graal_Auto_Key/Graal Auto Key.exe` | 42793 | `application/vnd.microsoft.portable-executable` | trainer_binary | `463e8f757e82bc5a…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/Graal_Digger/Graal Digger.exe` | 36111 | `application/vnd.microsoft.portable-executable` | trainer_binary | `876723732378b36e…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections ⚠️ Carved from corrupt zip; CRC fail; decompressed 36111 vs declared 41893 (truncated) |
| `Trainers/extracted/Graal_Digger/Graal_Digger-Howto.txt` | 219 | `text/plain` | writeup | `2166abaa7cc1f196…` | ASCII text, with CRLF line terminators |
| `Trainers/extracted/Graal_Prayer/Graal Prayer.exe` | 43420 | `application/vnd.microsoft.portable-executable` | trainer_binary | `6959bcb8066efdee…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/Graal_Prayer/data/data.exe` | 42676 | `application/vnd.microsoft.portable-executable` | trainer_binary | `c331eee0e985c903…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/Graal_Random_Walker/Graal Random Walker.exe` | 43185 | `application/vnd.microsoft.portable-executable` | trainer_binary | `b41973d19fc0089c…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/Graal_WaterGet/Graal WaterGet.exe` | 41820 | `application/vnd.microsoft.portable-executable` | trainer_binary | `7482de4824d370f1…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/Graal_WaterGet/Graal_WaterGet-Howto.txt` | 224 | `text/plain` | writeup | `7c11e4e42123e834…` | ASCII text, with CRLF line terminators |
| `Trainers/extracted/Graal_Water_Get/Graal Water Get.exe` | 43483 | `application/vnd.microsoft.portable-executable` | trainer_binary | `4f95fa2af7d7db1c…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/autoprogs/AutoD.EXE` | 41317 | `application/vnd.microsoft.portable-executable` | trainer_binary | `5a4493d79d0619a9…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/autoprogs/AutoIdle.exe` | 42041 | `application/vnd.microsoft.portable-executable` | trainer_binary | `ebf4a5913132fa8f…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/autoprogs/AutoS.exe` | 41072 | `application/vnd.microsoft.portable-executable` | trainer_binary | `7e427085717c47e2…` | PE32 executable (GUI) Intel 80386, for MS Windows, UPX compressed, 3 sections |
| `Trainers/extracted/capslock/capslock.exe` | 130048 | `application/vnd.microsoft.portable-executable` | trainer_binary | `2576cbeacd5ed30f…` | PE32 executable (GUI) Intel 80386, for MS Windows, 10 sections |
| `Trainers/extracted/capslock/readme.htm` | 7990 | `text/html` | writeup | `f055ad6e0a8c9838…` | HTML document, ASCII text, with very long lines (815), with CRLF line terminators |
| `defense-checklist.md` | 5153 | `text/markdown` | defense_notes | `fe5c170968bbec96…` | Unicode text, UTF-8 text |

## Sibling static-extract artifacts

See `_static_extract/` (UPX unpacks, PE info, strings, AutoIt script dumps). Full hashes in `inventory.json`.

