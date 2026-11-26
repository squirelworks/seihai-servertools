# seihai-pw3d

Turns a 2D Graal playerworld (a folder of `.nw`, `.graal` or `.zelda` levels
and their gmaps) into 3D worlds that seihai-gserver serves to the 3D clients
(4.20r6, 4.20r8, 5.x).

```
seihai-pw3d INPUT OUT [--start LEVEL] [--at X,Y] [--tileset PICS1.PNG]
            [--tiledefs FILE] [--wall-height F] [--colour-bits N] [--force]
seihai-servertools pw3d ...        # the same, from the umbrella binary
```

## Why it has to build 3D content

A 3D client draws none of a level's tiles, signs or NPCs: entering 3D
replaces the 2D view (Graal_analysis `torque/3d-win10-sky-prefs-scripting-512-2026-11-26.md`,
T3D-138). It never moves its own player, has no collision and runs no level
links (`torque/3d-scoping-interiors-limits-512-2026-11-26.md`). What it does
draw is GameObjects with `.3ds` shapes that have colour materials, a Sky and
a Sun, and it runs `weapon3d` GS2 scripts. So:

| 2D | 3D | |
|---|---|---|
| tile grid (layer 0) | one `.3ds` ground a level: one quad per run of equal tiles, sides where a tile stands higher than its neighbour | layout lossless |
| tile picture | the tile's average colour on a `pics1.png`-layout sheet, quantised (`--colour-bits`) to one material per colour | approximated |
| walls (tiledefs type 22 and up) | raised blocks `--wall-height` tall; type 20 (bushes, low walls) half as tall | approximated |
| water, lava | sunk 0.35 and 0.15 | approximated |
| upper tile layers | the top-most tile gives the colour; layer 0 decides walking, as in 2D | approximated |
| NPCs with an image, baddies, chests, signs | coloured boxes (purple, red, wood and gold, a post with a board) | approximated; NPC images, scripts, baddy behaviour and chest items dropped |
| links | tinted tiles; the walker takes them, switching world with `g3dworld` when the link leaves the world | lossless (links to levels not in the folder are listed and dropped) |
| signs | the walker shows the text while the player stands below one | lossless text |
| gmap | one world, its levels side by side (Torque `x`, `-y` from tile `x`, `y`, one unit a tile) | lossless layout |
| any other level | its own world, `config/graal3dworlds/<level>.txt`, so a TCP warp to that level also takes a 3D client there | |

The `PW3DWalk` weapon (`weapons/pw3dwalk.gs2`) carries each world's walk
grid as run-length rows and its links and signs. Each tick it reads the
arrow keys (`keydown2`), moves a quarter tile unless the tile ahead is a
wall, sets `player.position` and keeps a free camera behind and above the
player. This is the walker shape a real 5.12 walked with in an earlier lab.

## Output

`OUT` is a seihai-gserver world folder:

- `config/graal3dworld.txt`: the start level's world (`--start`, default
  `onlinestartlocal`, `start`, else the first level by name);
- `config/graal3dworlds/<name>.txt`: every other world;
- `files/`: `pw3d_<level>.3ds` per level, `pw3d_player.3ds`, six sky faces;
- `weapons/pw3dwalk.gs2`;
- `levels/`: the 2D levels and gmaps, unchanged, so 2D players share the
  world;
- `PW3D-REPORT.md`: per level, what came across, what was approximated and
  what was dropped.

Add `config/server.txt` (with `graal3d = true`) and accounts as for any
world. `OUT` holds copies of the playerworld's own levels: keep it local and
do not publish it. Everything else in it is generated.

Tile colours come from Seihai's own built-in tileset and walls from its
built-in tiledefs (both from seihai-core) unless `--tileset` and
`--tiledefs` name others, such as a client's `pics1.png` you have locally.

## Build and test

```
cargo build -p seihai-pw3d
cargo test -p seihai-pw3d
```

It depends only on `seihai-core` (pinned once in the workspace by tag) for
the level, gmap and tiledefs parsers, the built-in tileset and the GS2
compiler the tests check the walker with, and `png` for the sky. The tests
run the binary on a made-up playerworld and check the worlds, shapes, links
and that seihai-core compiles the walker, as the gserver does at link time.
