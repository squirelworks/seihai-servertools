# Salvage candidates → seihai-servertools (clean-room only)

After the **2026-10-31 full Tools Drive download** (~308 files / ~2.8 GiB staged). **Do not** commit recovered binaries, decompiled C, or GPL/Delphi text into the MIT clone. Placement tree: [`SALVAGE_PLACEMENT.md`](./SALVAGE_PLACEMENT.md).

## Clear candidates (this pass)

**None** ready for a new `setup-core` / `lat-core` feature PR yet. Corpus is inventoried; most high-value PE/ELF work stays `analysis-only` or belongs in sibling repos.

| Class | Examples | Target | Why not promoting now |
| --- | --- | --- | --- |
| GServer | `GServersource`, `gserver source`, 1.39 MOD builds | `seihai-gserver` / GPL fork | Runtime — not servertools MIT crates |
| Server Lister | `serverlist3`, ListServer | `seihai-listserver` | Prefer existing listserver notes over RE paste |
| RC | `RemoteControl3.exe`, custom RC, linux RC tarballs | **`seihai-rc`** | Needs dedicated RC protocol work; thin on SUMMARY notes |
| NC | `NPCControl.exe`, `nc7132001` | **`seihai-rc`** / `seihai-npcserver` | Same as RC; do not execute PEs |
| LAT editors | TilesEditor, Greaditor, Gonstruct, converters | **`seihai`** offline · **`lat-core`** live later | Blocked on shared drawing + RC RemoteFiles |
| NAT / GS2 | GS2 Compiler, scripteditor253, BytecodeViewer | analysis-only | Licence / Delphi risk; behaviour notes only |
| GAT / Gani / viewers | NW2PNG, GraalViewer, AnimViewer | **`seihai`** assets | Client-side tooling |
| Client archive / DClient / designers | Main Client Archive, GCD, GChar, DClient | out-of-scope / `seihai` | Not servertools; DClient archive incomplete |
| Trainers | Auto Key, Digger | out-of-scope | See `recovered-hacks/` |

## Optional (weak) behaviour notes only

**Lab status files** (from `gserverstats.pl` / gserverstat companions — not from reimplementing HTML ELFs):

- Classic ops wrote `playercount.txt`, `accountcount.txt`, `serveruptime.txt` under the gserver path and checked whether a `gserver` process was alive.
- `setup-core` **already** implements wizard lights from **console lines** (`Listening on`, listserver connect). A future optional reader for those three text files could enrich a lab status page — only if a lab profile actually emits them.
- If pursued: invent loopback fixtures; do **not** copy the Perl or embed real host paths from the dump.

**RC / NC staff file browser (future):**

- Behaviour belongs in **`seihai-rc`**; `lat-core` keeps the `RemoteFiles` trait + `LocalFolder` until a tagged RC client exists.
- Invent loopback fixtures; never point salvage tests at public hosts.

**LAT offline formats:**

- Prefer extending `seihai-core` / level-editor in **`seihai`**, then pin by tag into servertools — do not duplicate parsers here.

## Blockers (do not chase)

- **7** Google `cannotDownloadAbusiveFile` items (including `GServer Stats.zip`, cracked/client packs, `apcalc.exe`) — leave blocked; do not mirror.
- `WorldsSetup64-551.rar` — Drive 404 (`-518` is present).
- `PaintShopPro Files.rar` — downloaded, skipped unpack (huge).

## Process reminder

Promote only when behaviour is understood from notes/tests, target crate is clear, and no raw/GPL text would land in the clone (`SALVAGE_PLACEMENT.md` §C).
