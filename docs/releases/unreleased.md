Notes for the next seihai-servertools release. One line per user-visible change, added by the PR that makes it; the release PR moves them into `vX.Y.Z.md` (VERSIONING.md).

- `seihai-suite-stress probe` has two new steps. `want FILE` sends a file request. `expect SECONDS TEXT` fails the run unless a matching server message arrives in time, so a steps file can be a smoke test (seihai-gserver-v2#87).
- Built on seihai-core and seihai-draw v0.7.2, with seihai-rc at a `main` commit on the same core, so the lock file holds one core (it held v0.6.1 and, through seihai-rc, v0.4.0). The LAT NPC-server database page now tokenizes NPC and class scripts in the server's text profile, which seihai-rc reads from the server's `Server:` line.

- Movie previews no longer overflow the word-wrap width calculation when a TEXT actor has an extreme negative horizontal coordinate; ordinary wrapping and partially visible text retain their placement.

- World checks can select a shared client build with `check --client LABEL` or the GUI profile picker; `check --list-clients` lists labels. Selected level/gmap/gani/script capability warnings preserve unknown support and JSON format 1. Default inventory and dependency pins are unchanged; per-build package readability and exact script vocabulary are not certified.

- Saving levels and ganis preserves parsed finite NPC, baddy, link-arrival and sound coordinates outside the i32 range instead of silently narrowing them; ordinary formatting and existing keep-arrival semantics remain unchanged.

- Listserver setup reads valid TOML strings and integers consistently with its editor, preserving quoted account paths, escapes and numeric separators; malformed files retain the existing best-effort fallback.

- Repacking resource packages validates and finishes a temporary output before replacement, preserving an existing package on validation/write refusal and retaining Unix file-creation modes; package and key-file updates remain separate.

- `gen terrain --3d` also writes the world for a Graal 3D server: a `.3ds` ground built from the same height field (one coloured mesh per level), tree and rock shapes, and a `graal3dworld.txt` that places them on the ground with a sun and a spawn point. `--vscale` sets the height scale and `--trees` the placements per level. Not yet loaded by a real 3D client.
- `gen terrain --stream` (implies `--3d`) also writes the ground as streamed terrain for 4.20r6 and 5.x clients. `NAME_terrain/` holds the 256 `heights<x>-<y>.dat` chunk files, `heights.info` (one texture name a line) and four 256 x 256 24-bit textures, and the world file uses seihai-gserver-v2's `terrain` line. Each chunk is written exactly as the client re-encodes it, so the stamps the client sends back match. The format is the one the real 5.12 client was confirmed to read on 2026-11-25 (Graal_analysis `torque/terrain-512.md`). A full generated world has not yet been loaded on a real client.
