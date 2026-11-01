# Ghidra salvage analyses

Produced by `/workspace/tools/ghidra/analyze_tool.sh`.  
Pipeline docs: `/workspace/tools/ghidra/README.md`.  
Placement: `seihai-servertools/docs/design/SALVAGE_PLACEMENT.md` · inventory: [`../INVENTORY.md`](../INVENTORY.md).

Focus: purpose, CLI, protocols, and config roles for rebuild notes. Do **not** paste decompiled Delphi/GPL or GServer-v2 sources into MIT crates. Committed `SUMMARY.md` files are hand-written prose only: no string dumps, import lists, decompiler names, or binary offsets. Raw exporter output belongs in gitignored `tools/ghidra/local-out/`.

## Completed

| Dir | Binary | Placement | Highlights |
| --- | --- | --- | --- |
| `RemoteControl3/` | `windows_rc3_beta` / v5 `RemoteControl3.exe` | `seihai-rc` | GTK Win32 RC; listserver + NPC-server connect; player/NPC/account/file UI. |
| `remotecontrol_custom/` | Custom `remotecontrol.exe` (+ UPX `Agret_RC.exe`) | `seihai-rc` | Delphi Agret “Blue RC”; `controlconfig.txt` / `Software\Graal`. |
| `RemoteControl_classic/` | `rc.rar` `RemoteControl.exe` | `seihai-rc` | Stock Delphi RC baseline vs Agret/NW. |
| `RemoteControl-NW/` | NW2 / NC-pack `RemoteControl-NW.exe` | `seihai-rc` | NW-era Delphi RC; admin/account/ban/update-level. |
| `NPCControl/` | `nc7132001` `NPCControl.exe` | `seihai-rc` | NC client; `controlconfig.txt`; NPC/weapon/class scripts. |
| `scripteditor/` | Senolen `scripteditor.exe` 2.53 | `analysis-only` | NAT GS1 script editor UX; prefer GS2 sources for modern notes. |
| `PNGView/` | GAT `PNGView.exe` | `analysis-only` | Offline PNG viewer beside Assimilator. |
| `serverlist3/` | `slist/serverlist3` | `analysis-only` | ELF32 classic listserver; `clientsport`/`serversport`/`playerlimit`. Prefer C# rewrite tree. |
| `ListServer/` | UPX-unpacked `ListServer.exe` | `analysis-only` | Win32 Delphi plus Winsock; an ini server list. Do not run. |
| `gserver-linux/` | `GServersource/gserver` | `analysis-only` | ELF32 — prefer paired C++ source. |
| `gserver-win/` | `GServersource/gserver.exe` | `analysis-only` | Win32 PE same era/hash as pe alias. |
| `gserver13/` | `gserver_source/gserver13` | `analysis-only` | Classic Linux gserver ELF. |
| `gserver_pe_GServersource/` | same PE as `gserver-win` | `analysis-only` | Alias; watch `passwords.h` in sources. |
| `Matriark_gserver/` | Matriark 1.39 static ELF `gserver` | `analysis-only` | Stripped static; prefer GServersource notes + pack bugs.txt. |
| `BytecodeViewer/` | GS2 `BytecodeViewer.exe` | `analysis-only` | Console dump of Relay bytecode; **source authoritative**. |
| `TextConvert/` | `TextConvert.exe` | `analysis-only` / client-adjacent | Graal→NW text helper. |
| `Assimilator/` | GAT GIF2PNG `Assimilator.exe` | `seihai` / `analysis-only` | Offline Graal asset batch checker/converter. |
| `TilesEditor/` | 39ster `TilesEditor.exe` | `seihai` · later `lat-core` | Qt64 offline level/NPC/chest editor. |
| `GraalLevelGenNW/` | `GraalLevelGenNW.exe` | `lat-core` / `analysis-only` | Offline NW world generator from map image. |
| `gserverstat/` | `gserverstat` | `analysis-only` | HTML leaderboards; `statmaker` loop. |
| `gserverstat2/` | `gserverstat2` | `analysis-only` | Admin power/weapon HTML stats. |
| `GCD/` | `GCD.exe` | `out-of-scope / seihai (client)` | Character Designer 1.2.7 (Delphi). |
| `GChar/` | `GChar.exe` | `out-of-scope / seihai (client)` | Character generator sibling. |
| `Graal_Auto_Key/` | UPX-unpacked Auto Key | `out-of-scope` | AutoIt v2.64 key sender trainer. |
| `Graal_Digger/` | truncated UPX digger | `out-of-scope` | Trainer overlap; UPX corrupt. |
| `DClient_setup/` | salvaged `setup.exe` | `stay-staged` / client `out-of-scope` | VB6 bootstrapper; CAB truncated on Drive. |

## Prefer source over RE when present

- `_inbox/extracted/GServersource/` — `gserver.cpp`, `npcs.cpp`, `levels.cpp`, makefiles, MySQL bits.
- `_inbox/extracted/gserver_source/` — `gserver13`, `gserverstat*`, `statmaker`.
- `_extracted/listserver-nested/graal-listserver/` — C# listserver rewrite.
- `_extracted/gs2-compiler/GS2 Compiler/BytecodeViewer/Source/` — BytecodeViewer C++.
- `_extracted/gs2-compiler/GS2 Compiler/BytecodeCompiler/` — C++ compiler sources.

## Blockers

| Item | Reason |
| --- | --- |
| `DClientSetup1198.zip` | Truncated on Drive (no EOCD). |
| `GServer Stats.zip` (Drive) | Google `cannotDownloadAbusiveFile` — skipped. |
| Other malware-blocked Drive archives | See `INVENTORY.md` (`Account Manager.rar`, cracked client rars, `apcalc.exe`, …) — skipped. |
| `Graal Digger.exe` | Corrupt UPX overlay; trainer overlap. |
| Matriark / some Delphi PEs | Headless may exit nonzero after successful export (`rc=1`); artifacts still present — treat SUMMARY prose as authoritative. |

## Re-run

```bash
/workspace/tools/ghidra/analyze_tool.sh \
  /path/to/bin \
  /workspace/recovered-drive-tools/ghidra-analysis/<name> [max_decomp]
```

Raw files land in gitignored `tools/ghidra/local-out/<tool>/`. Edit `SUMMARY.md` by hand; the script will not overwrite it and will not paste strings or imports into it.
