# seihai-servertools: plan

## Goal

Make a Graal server easy to run and easy to look after:

1. **One download, one command** to start a lab world: the listserver and the
   gservers on `127.0.0.1`, stopped together.
2. **A setup wizard** (window and command line) that takes a server from the
   lab defaults to the public, refuses unsafe combinations, and puts
   everything back with one "Back to lab" button. It can also set up this
   machine to host a listserver for others.
3. **The LAT toolkit** for Level Administration Team staff: a level editor
   that works on a live server through the RC file link, NPC and gani
   editors, and a world and config checker.

## Decisions (do not reopen without asking Squirrel)

- `MIT OR Apache-2.0`. No code from GServer-v2, gs2lib, gs2-parser or
  graal-listserver (`AGENTS.md`).
- Window toolkit: egui/eframe, the same version as the Seihai client and its
  level editor (Squirrel, 2026-10-31).
- The setup wizard also exists on the command line (`seihai-servertools setup`)
  with the same steps as prompts (Squirrel, 2026-10-31).
- "Host a listserver" is in the first version, on `seihai-listserver`'s
  existing settings (Squirrel, 2026-10-31).
- The fork (`seihai-gserver-v2`) stays gserver only; its start scripts moved
  here (`scripts/lab/`).
- The Seihai client's offline level editor stays in Seihai. The LAT level
  editor here works on a live server.
- Drawing and protocol code is shared with Seihai through `seihai-core` (and,
  for drawing and the level document, a new shared crate in the `seihai`
  repo), pinned by tag. Never copied.
- No real hostnames or IPs anywhere; no outbound calls the owner did not ask
  for.

## Tracks

| Track | What | Design | Depends on |
|---|---|---|---|
| Lab | One-step start of the lab world | this file, `ARCHITECTURE.md` | the fork's release layout, `seihai-listserver` |
| Setup | Join a listserver, Host a listserver, Back to lab | `design/go-public.md` | the fork's `serveroptions.txt`, `seihai-listserver.toml` |
| LAT | Level, NPC and gani editors on a live server | `design/lat-tools.md` | `seihai-rc` file browser (stage 4) and NC login (stage 5); shared drawing crate in `seihai` |
| Check | World and config checker | `design/lat-tools.md` | `seihai-core` parsers |

The order of the work is in [NEXT.md](NEXT.md).
