# Salvage analysis: BytecodeViewer.exe

- **Binary:** `/workspace/recovered-drive-tools/_extracted/gs2-compiler/GS2 Compiler/BytecodeViewer/ViewerApp/BytecodeViewer.exe` (same tree under NAT Tools GS2 Compiler extract / `_inbox/extracted/GS2 Compiler/`)
- **Analyzed (UTC):** prior headless run · SUMMARY rewritten 20261031T211700Z from **source + binary identity** (PE RE thin; prefer source)
- **Ghidra:** 12.2
- **Purpose focus:** recover CLI, protocols, config paths, and rebuild/rehost notes for seihai-servertools (ops/server tooling — not malware inventing).

## Purpose guess

Small **console PE** that dumps a Graal **GS2 compiled script bytecode** capture for humans. Companion package ships full C++ source (`BytecodeViewer/Source/main.cpp`) plus opcode name tables (`BytecodeViewer_defs.txt`). Prefer reading that source over decompile — Ghidra mostly sees CRT/locale noise on this build.

## Recommended placement (`SALVAGE_PLACEMENT.md`)

| Placement | `analysis-only` |
| --- | --- |
| Why | NAT/GS2 diagnostics aid; behaviour understood from source. Useful reference for script-toolchain docs, not a seihai-servertools runtime dependency. |
| Not | `setup-core` / `lat-core` / `seihai` / `seihai-rc` |
| Related | GS2 Compiler sources in the same package; NAT script editors |
| Licence risk | Third-party / community tool — notes only |

## Identity

```
# file
PE32 executable (console) Intel 80386, for MS Windows, 5 sections

# size
88K (Sep 2017 stamp on recovered copy)

# sha256
8e6b1f22947a6df814fc2f4be8169998e1f2bd1effc2becf51029eff3107f6a9
```

## Ghidra program

| Field | Value |
| --- | --- |
| Format | Portable Executable (PE) |
| Language | x86:LE:32:default |
| Compiler | windows |
| Functions | 313 (internal 310) |
| Imports | 73 |
| Exports / entries | 40 |
| Interesting strings | 64 (mostly CRT) |
| Decompiled funcs | ~30–40 |

## Behaviour (from source)

CLI: `BytecodeViewer <script file>`. Opens the capture, **skips the first byte** (Graal Relay message id prefix), then parses:

1. **Information section** — length-prefixed; prints code target, target name, and whether to save script to file.
2. Segmented payload named in source as: integer/unknown purpose, function names, strings, bytecode.
3. Opcode pretty-print driven by `BytecodeViewer_defs.txt` (index=`name` lines) loaded from the working directory.

Big-endian short decoding matches other GS2 tooling. No network listen or connect — offline file viewer only. Ordinary Windows file I/O through the C runtime dominates imports.

## Prefer source over RE

- `…/BytecodeViewer/Source/main.cpp`, `Arrays.h`, project files (`.sln` / `.vcxproj` / `.cbp`)
- `BytecodeViewer_defs.txt` / `BytecodeViewer_defs2.txt` beside ViewerApp

## Rebuild / rehost checklist

- [x] Language: native C++ console, MSVC-era PE.
- [x] CLI and defs-file contract documented in source.
- [ ] Optional: reimplement dump against seihai GS2 fixtures if NAT docs need it — do not vendor binary.

## Artifacts

- Local Ghidra export under this dir (CRT-heavy); treat source tree as authoritative.
