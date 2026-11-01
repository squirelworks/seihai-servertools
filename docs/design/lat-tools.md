# LAT toolkit: design

LAT (Level Administration Team) staff build and fix a running server's world.
They need to open a level *on the server*, change it, and put it back,
without shell access to the machine. These tools do that through the RC
file link, with the staff member's own account and rights.

Not in scope: the Seihai client's offline level editor
(`tools/level-editor` in `squirelworks/seihai`). It edits files on disk in
offline mode, as the original client's editor did, and stays there.

## Building blocks

| Block | Where | State |
|---|---|---|
| File link: list, download, upload | `lat_core::remote::RemoteFiles`; `LocalFolder` and `RcFiles` | working |
| RC login and file link (list, get, put) | `seihai-rc`, pinned commit | in use |
| NC login (database NPCs, classes, weapons) | `seihai-rc` | not used here yet |
| Level, gmap, gani, GS1 parsers | `seihai-core` (pinned commit) | in use |
| Level document (line-preserving edit, objects, NPC pane) | `tools/level-editor` in `seihai` | still only the offline editor; LAT writes `GLEVNW01` text |
| Tile grid and one gani frame | `seihai-draw` | in use |

## Level editor (LAT)

1. **Connect**: RC login (direct `127.0.0.1:<port>` in the lab, or through
   the listserver's RC list), account and password from a prompt or an
   environment variable. Show the account's rights; warn when it cannot
   upload to the levels folder.
2. **Browse**: the server's folders through `RemoteFiles::list`; open
   `.nw`, `.graal` and `.gmap`.
3. **Edit**: the shared level document and drawing, so it looks and saves
   exactly like the Seihai editor (unchanged level = same bytes).
4. **Upload**: show a diff against the downloaded bytes; before writing,
   download the server's copy again and refuse if it changed (someone else
   edited it). Keep the downloaded original in a local backup folder.
5. **Gmaps**: open a cell's level from the gmap grid, as the Seihai editor
   does.

## NPC editor

- Level NPCs: the whole `NPC ... NPCEND` script through the level document,
  GS1 syntax check with `seihai-core`'s parser before upload.
- NPC-server objects (database NPCs, classes, weapons): through the NC login,
  once `seihai-rc` has it. List, edit, add, delete.

## Gani editor

A line-preserving gani document, frame list, sprite table, sound and
attribute lines, and a preview drawn with the shared drawing code (the
client's own gani player for timing). Load and save through `RemoteFiles`.

## World and config checker

`seihai-servertools check WORLD` and the World check page. Today: parse
errors, links to missing levels, gmap cells naming missing levels, ganis
that do not parse, and images or sounds that level NPCs and ganis name but
the world folder lacks (`seihai-core`'s asset resolver). Next: GS1 syntax
errors, including images and sounds named inside NPC scripts, the
`serveroptions.txt` safety checks from `setup-core`, and a `--json` report.
Later the same checks run on a live server through `RcFiles`.

## Testing

Each editor is tested on `LocalFolder` first, then on a fake RC file browser
(as `seihai-rc`'s `tests/fake_servers.rs` does), then in the lab against the
fork on `127.0.0.1` with a LAT account. Never on a public server.
