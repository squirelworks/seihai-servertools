# Salvage analysis: TextConvert

- **Binary:** `_extracted/graal-to-nw/TextConvert.exe` (also under LAT Level Converters / Tools `GRAALtoNWconverter`)
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland Delphi, ~303 KB

## Purpose (plain behaviour)

Offline **Graal → New World text/level conversion helper**. Delphi GUI; no network service role.

Useful as a behaviour reference for level/text migration tooling that may land in **`seihai`** format crates — not as code to paste.

## Recommended placement

| Tag | `analysis-only` / client-adjacent `seihai` |
| --- | --- |
| Why | Offline converter; format facts only. |
| Not | `setup-core`, `seihai-rc` |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~1839 (internal ~1500) |
| Imports | ~342 |
| Interesting strings | ~111 |
| Decompiled (local only) | 20 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`
