# Salvage → seihai-servertools placement bridge

For Drive-download, Ghidra, and salvage agents. Read this before copying anything into the clone. Full crate map: [`SEIHAI_SERVERTOOLS_MAP.md`](./SEIHAI_SERVERTOOLS_MAP.md).

## Target repo

| | |
|---|---|
| **Local path** | `/workspace/seihai-servertools` |
| **Remote** | `https://github.com/squirelworks/seihai-servertools` |
| **Licence** | `MIT OR Apache-2.0` — never paste GServer-v2 / gs2lib / decompiled Delphi |
| **Staging (this folder)** | `/workspace/recovered-drive-tools/` |

The clone is a **nested sibling repo** (gitignored from the Seihai client tree). Do **not** commit it into `squirelworks/seihai`. Promote only clean-room MIT rewrites into the clone; leave raw dumps here.

---

## Crate mapping decision tree

Ask in order. Stop at the first yes.

```
Recovered artifact / behaviour note
│
├─ Raw binary, installer, zip, Ghidra DB, decompile dump, offsets?
│     → KEEP in /workspace/recovered-drive-tools/  (never commit into seihai-servertools)
│
├─ Client trainer / bot / auto-key / digger / AFK helper?
│     → OUT OF SCOPE for servertools (client cheat archive only; do not place)
│
├─ Offline level editor, Document model, tile/sprite drawing, format parsers?
│     → squirelworks/seihai (seihai-core / tools/level-editor / shared edit crate)
│       then pin by git tag in seihai-servertools Cargo.toml — do not duplicate
│
├─ RC/NC session, staff file browser packets, live RemoteFiles transport?
│     → squirelworks/seihai-rc (pin by tag); lat-core only holds the trait + LocalFolder
│
├─ Gserver / listserver / NPC-server runtime itself?
│     → seihai-gserver / seihai-listserver / seihai-npcserver (or GPL fork)
│
├─ Lab start, process group, world folder layout, start scripts?
│     → crates/setup-core/{lab,run}.rs + scripts/lab/
│
├─ serveroptions.txt / listserver TOML / go-public / lab↔public / backups / lights / test-connect?
│     → crates/setup-core/ (config/, profile, checks, lights, net, folder)
│
├─ Live LAT level / NPC / gani edit helpers (once shared drawing + RC exist)?
│     → crates/lat-core/{level,npc,gani,remote}.rs + later app pages
│
├─ World / config checker (missing links, gmap holes, options safety)?
│     → crates/lat-core/check.rs (+ CLI check, Check page)
│
└─ Unsure?
      → leave staged here; add a row to INVENTORY.md with Placement = TBD;
        do not invent a crate home
```

### Quick targets

| Kind | Path under clone |
|---|---|
| Lab / go-public / config / process | `crates/setup-core/` |
| LAT live editors & world checker | `crates/lat-core/` |
| CLI dispatch / env / exit codes only | `crates/seihai-servertools/` |
| egui pages (call cores; no rules) | `crates/app/src/pages/` |
| Owner-droppable start scripts | `scripts/lab/` |
| Salvage lab notes | `docs/lab/` or `docs/design/` |

### Hard rules when writing into the clone

1. **Core-crate logic only** — rules/formats/checks in `setup-core` or `lat-core` with tests; CLI prints; UI holds state.
2. **Line-preserving config** — edit `serveroptions.txt` / listserver TOML keeping comments, order, unknown keys; temp + rename; dated backup.
3. **Loopback-only** — `127.0.0.1` and lab ports; no real public hosts/IPs in code, docs, or tests.
4. **egui / eframe 0.29** — match Seihai client / level editor.
5. **Secrets from env or prompt** — never argv; no telemetry; no built-in server lists.

---

## Do not commit raw binaries

**Never** add these to `seihai-servertools` (or to the Seihai client repo):

- `.exe` / `.dll` / `.so` / installers / `.zip` / `.7z` / `.rar` of recovered tools
- Ghidra project DBs, `.rep`, memory dumps, decompiled C, IDA/Rizin DBs
- Binary offsets (`0x…`, `FUN_…`) in committed source or docs inside the MIT repos
- GPL GServer-v2 / gs2lib source, even as “reference copy”

Raw material stays under `/workspace/recovered-drive-tools/<archive>/` (and analysis under `ghidra-analysis/`). Only a **clean-room MIT rewrite** (behaviour notes → new Rust) enters the clone, on its own branch/PR.

Client `recovered-hacks/Trainers/` bots are unrelated to servertools — do not map them into this clone.

---

## Checklist: when `INVENTORY.md` + `ghidra-analysis/` appear

Use this when Drive download or Ghidra agents drop candidates. Do **not** invent rows beforehand.

### A. Inventory landed (`/workspace/recovered-drive-tools/INVENTORY.md`)

- [ ] Confirm each row is a real file under this folder (no speculative tools).
- [ ] Add / fill a **Placement** column using the decision tree above (`setup-core` / `lat-core` / sibling repo / stay-staged / out-of-scope).
- [ ] Mark licence risk: Delphi/GPL dump → behaviour notes only.
- [ ] Incomplete or corrupt archives → Placement = `stay-staged (bad archive)`; do not open a rewrite PR.

### B. Ghidra analysis landed (`/workspace/recovered-drive-tools/ghidra-analysis/`)

- [ ] Prefer markdown behaviour write-ups over importing DBs into any Seihai repo.
- [ ] Strip offsets/addresses before anything is summarized into `docs/` of seihai-servertools.
- [ ] Map each analysed tool → one crate target (or “not servertools”).
- [ ] If LAT editor behaviour: note blockers (shared drawing crate + `seihai-rc` RemoteFiles) — implement `LocalFolder` path first.
- [ ] If setup/lab/config behaviour: prefer extending existing `setup-core` modules over new crates.

### C. Ready to open a clean-room stub in the clone? (rare)

Only if **all** are true:

- [ ] Behaviour is understood from notes/tests (not pasted decompile).
- [ ] Target crate is clearly `setup-core` or `lat-core` (or a thin CLI/UI wire-up).
- [ ] No raw binary or GPL/Delphi text would be committed.
- [ ] Loopback-safe fixtures exist or can be invented without real hosts.

Otherwise: update Placement in `INVENTORY.md` and stop. **Do not** open a seihai-servertools PR just to park dumps.

---

## Agent hand-off

| Agent | Drop here | Then |
|---|---|---|
| Drive download | archives under `recovered-drive-tools/` (+ update `INVENTORY.md`) | leave Placement blank or `TBD` if unsure |
| Ghidra | `ghidra-analysis/<tool>/` markdown + optional private DB | fill Placement using this doc |
| Salvage / rewrite | clean-room Rust only | branch in `/workspace/seihai-servertools`, cores first |

**Concrete placements (2026-10-31 full corpus):** see [`INVENTORY.md`](./INVENTORY.md) (~308 downloaded / ~2.8 GiB staged).

| Class | Placement |
|---|---|
| GServer / Server Lister / GS2 Compiler | `analysis-only` (never commit binaries/GPL into MIT) |
| **RC** / **NC** | **`seihai-rc`** (and npcserver for NC DB) — analysis until clean-room; pin by tag for `lat-core` RemoteFiles |
| **LAT** editors / converters / generators | offline → **`seihai`**; live → **`lat-core`** after shared drawing + RC |
| Lab / status / start / config helpers | **`setup-core`** |
| GAT / Gani / NW2PNG / viewers | **`seihai`** client/assets (mostly analysis-only binaries) |
| Auto Key / Digger / cracked / packet loggers | `out-of-scope` |
| Character Designer / charactergenerator / Main Client Archive | `out-of-scope / seihai (client)` |
| DClientSetup1198.zip | `stay-staged (bad archive)` |

Ghidra markdown under `ghidra-analysis/` (gserver family, ListServer/serverlist3, BytecodeViewer, GCD/GChar, DClient setup, trainers). Next thin notes: RC / NC PEs — static only, never execute.
