# Salvage analysis: Assimilator (GIF2PNG / batch converter)

- **Binary:** `Developer Tools/GAT Tool - [Graphics Administration Team]/Assimilator GIF2PNG.rar.extracted/Assimilator.exe`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Borland C++/Delphi-era UI, ~427 KB

## Purpose (plain behaviour)

GAT-era **batch file converter / checker** branded around Graal assets (archive name “GIF2PNG”).

Observed behaviour:

- Batch-checks and converts graphics-admin assets: classic scripts, levels, plain text, and animations.
- Keeps a checklist and reports an error when the list is empty or nothing is selected.
- Shows a client-version status that stays blank when no client is configured.
- Offline graphics utility — not a network service.

## Recommended placement

| Tag | `seihai` / `analysis-only` |
| --- | --- |
| Why | Offline asset conversion helpers for graphics/admin workflows. |
| Not | `setup-core`, `seihai-rc`, live LAT |
| Licence | Legacy Borland tool — notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~2656 (internal ~2317) |
| Imports | ~347 |
| Exports | ~18 (unit init/finalize stubs) |
| Interesting strings | ~94 |
| Decompiled (local only) | 25 — **not committed** |

## Artifacts (local)

`prelim/`, `functions.*`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
