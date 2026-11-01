# Salvage analysis: scripteditor (Senolen 2.53)

- **Binary:** `Developer Tools/NAT Tools And Scripts [NPC Administration Team]/Script Editors/scripteditor253.rar.extracted/Script Editor/scripteditor.exe`
- **Mirrors:** Tools `scripteditor253.zip` / `.7z`; “player added files” rar variant
- **Analyzed (UTC):** 2026-10-31 · static strings + file(1); Ghidra optional follow-up in same dir
- **Format:** PE32 GUI (x86), ~1.0 MB

## Purpose (plain behaviour)

**Script Editor by Senolen** (v2.53 pack; upgrade UI mentions 2.54). GS1-era NPC/weapon scripting helper, not the later GS2 compiler.

Observed menu/behaviour surface:

- Open, close, play, and test documents, and set the client folder.
- Insert templates: NPC script, weapon, baddy scripting, chat command, function, conditional, take-item, horse and leap placement, and a note about a 3D light NPC.
- A find-NPC helper backed by a rich-text guide, paste script, save and save-as.
- Built-in “upgrade/download” path (do **not** use — external fetch; ignore for lab).

Prefer GS2 compiler **source** under `GS2 Compiler.rar` for modern script tooling notes; this editor is historical UX reference only.

## Recommended placement

| Tag | `analysis-only` |
| --- | --- |
| Why | Legacy GS1 script UX; not a clean-room target for MIT crates as-is. |
| Related | `BytecodeViewer/` + GS2 compiler sources |
| Not | `setup-core`, live `seihai-rc` session code |

## Notes

Re-run Ghidra into this folder if deeper import graphs are needed; keep SUMMARY free of offsets and anonymous decompiler names.
