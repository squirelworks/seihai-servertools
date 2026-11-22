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
| Graal3D lab (after Seihai M3) | Start/stop a February 2000 Graal3D host on loopback; optional `.x`+BMP world check | Seihai `documentation/plannedmilestones.md` (Graal3D M3–M5) | Seihai G3D-9 Rust host; Graal_analysis `graal3d/formats.md` |

February 2000 Graal3D does **not** use the listserver, RC, NC or 2D level
editors. Its worlds are DirectX `.x` meshes plus local BMPs, not `.nw` /
`.graal`. Do not extend LAT or Setup for it until Seihai's G3D-9 host
exists; then add only a lab kind and a texture-presence check (see
[NEXT.md](NEXT.md)).

The order of the work is in [NEXT.md](NEXT.md).

## Current follow-through (2026-11-22)

Roadmap issue #50's original Next list is done; its historical record stays
in [NEXT.md](NEXT.md). The first-release roadmap
[#99](https://github.com/squirelworks/seihai-servertools/issues/99) and
the wider 1.0 roadmap
[#100](https://github.com/squirelworks/seihai-servertools/issues/100)
remain open.

Dependencies were aligned by
[#106](https://github.com/squirelworks/seihai-servertools/pull/106):
`seihai-core` and `seihai-draw` are paired at `v0.7.2`, and
`seihai-rc` is pinned to rev
`fcfd2cc264fece3838a1acca56b1d2d3cc217a17` on that same core. The lock
file holds one core. The RC pin still needs a published tag when its
v0.2.0 release ships
([seihai-rc#66](https://github.com/squirelworks/seihai-rc/issues/66));
release builds, the release PR and owner publication are not completed by
the dependency alignment.

[#110](https://github.com/squirelworks/seihai-servertools/pull/110)
adds explicit shared-build selection to the world checker:
`check --client LABEL`, `check --list-clients` and the window's profile picker.
Selected-profile warnings cover successfully parsed level formats, gmaps,
ganis and classified GS1/GS2 level-NPC and weapon scripts. Unknown support
stays unknown; the default inventory and JSON format 1 are unchanged.
This is bounded progress toward #100, not completion: generic PNG/GIF
aliases remain shared, script classification is heuristic, syntax checks
still use V217, and the pinned profiles have no package-readability field.
It does not certify runtime or native compatibility.

Remaining work is listed in [NEXT.md](NEXT.md): the first-release gates,
full #100 checker coverage, suite-stress stage reruns and RC bots, remaining
NPC-server object work and the LAT-account lab, offline gaps in
`docs/design/classic-offline-gaps.md`, Graal3D lab/check when G3D-9 exists,
and defaulting to the Rust gserver at parity.
