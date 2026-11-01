# Defense RE — `Graal Digger.exe`

**Static analysis only — binary was never executed.**  
**Analyzed (UTC):** 2026-10-31  
**Ghidra:** 12.2 headless (limited — corrupt UPX)  
**Input:** `/workspace/recovered-hacks/Trainers/extracted/Graal_Digger/Graal Digger.exe`  
**Howto:** `/workspace/recovered-hacks/Trainers/extracted/Graal_Digger/Graal_Digger-Howto.txt` (author **Selzar**)

## Identity

| Field | Value |
|--|--|
| SHA-256 (carved PE) | `876723732378b36e8fd31d09120ec5a1b584cf46fb672ab9d5cf7a0a4f8b56d4` |
| Size on disk | 36111 (truncated; zip declares uncompressed **41893**) |
| PE TimeDateStamp | `1012939264` (2002-02-05T20:01:04Z) |
| Format | PE32 GUI, **UPX compressed**, 3 sections (`UPX0`/`UPX1`/`.rsrc`) |
| Runtime cue (packed strings) | `AutoIt` + mangled `_SCRIPTFULLPATH…` / splash fragment mentioning **v2.63** |

## Zip / packer blockers

1. **`Graal_Digger.zip` dual local entries:** first local header is `Graal_Digger-Howto.txt` (219 B); second is `Graal Digger.exe`. Central directory naming historically mismatched howto vs exe (taxonomy “zip quirk”).
2. **Deflate stream truncated:** recovered PE is 36111 B vs declared 41893 B (~5782 B missing) → CRC incomplete.
3. **`upx -d` / `upx -t`:** `OverlayException: invalid overlay size; file is possibly corrupt`.
4. **Ghidra:** imports **0**, interesting strings **0**, only a UPX stub entry recovered — expected for unbroken packed+truncated image.

## Static signals still usable for defense

| Signal | Detail |
|--|--|
| Packer | `UPX!` at file offset 992; section names `UPX0`/`UPX1` |
| Family | AutoIt (v2.63 splash fragment in compressed stream) |
| Howto behavior | Equip shovel → grass → start tool → focus Graal → OK to start dig loop |
| Filename IOC | `Graal Digger.exe`, `Graal_Digger-Howto.txt` |
| Expectation if intact | Same class as other RpgWarrior macros: `keybd_event` / dig key loop after focus — **not confirmed** on this truncated image |

`objdump` cannot map the UPX import directory (`.rsrc` claimed larger than available bytes). No Winsock strings observed in the truncated blob.

## Defense guidance

- Treat sample as **corrupt/incomplete AutoIt+UPX trainer**, not a confirmed second-stage malware drop — but do **not** execute; incompleteness can also indicate intentional anti-analysis packaging.
- Host detections: hash above + UPX+AutoIt combination + howto filename co-occurrence.
- Server-side: repetitive dig / shovel-use timing from focused classic client (same as Water Get / AutoD family).

## Artifacts

- Ghidra stub export under this directory (`SUMMARY` previously salvage-oriented; this file is the defense summary).
- `prelim/strings_all.txt`, `_analyze.log`
- Related: howto text; zip under `Trainers/Graal_Digger.zip`
