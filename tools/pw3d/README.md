# seihai-pw3d

Turns a 2D Graal playerworld (a folder of `.nw`, `.graal` or `.zelda` levels
and their gmaps) into 3D worlds that seihai-gserver serves to the 3D clients
(4.20r6, 4.20r8, 5.x).

```
seihai-pw3d INPUT OUT [--start LEVEL] [--at X,Y] [--tileset PICS1.PNG]
            [--tiledefs FILE] [--objects DIR] [--images DIR]... [--meaning-maps]
            [--wall-height F] [--colour-bits N] [--separate-interiors] [--force]
seihai-servertools pw3d ...        # the same, from the front door
seihai-pw3d --help | --version
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
| NPCs with an image | a cut-out of the picture (or its `setimgpart`), standing up as wide and tall as in 2D, found by name in `INPUT` and `--images` folders; a figure where the NPC shows a gani or the picture is a sheet or missing | approximated; scripts dropped |
| baddies, chests, signs | simple shapes (a red figure, a wood and gold chest, a post with a board) | approximated; baddy behaviour and chest items dropped |
| links | tinted tiles; the walker takes them, switching world with `g3dworld` when the link leaves the world | lossless (links to levels not in the folder are listed and dropped) |
| signs | the words on a board above each sign in 3D capital letters (our own 3×5 font with wider N, M and W; up to 8 lines of about 26 letters, narrower when signs stand close); the walker also says them in chat, which the 5.12 does not draw in 3D | text kept; lower case, colours and rare symbols lost |
| gmap | one world, its levels side by side (Torque `x`, `-y` from tile `x`, `y`, one unit a tile) | lossless layout |
| interiors (levels a link from the start level or a gmap leads to, and the levels linked on from them: houses, cellars, caves) | placed in the world they are entered from, in rows 1536 tiles south of it and 1536 apart, with `portal` lines for each door; the walker takes the door as a move, with no world switch | layout lossless; sky and light are the outdoor ones |
| any other level | its own world, `config/graal3dworlds/<level>.txt`, so a TCP warp to that level also takes a 3D client there | |

The `PW3DWalk` weapon (`weapons/pw3dwalk.gs2`) carries each world's walk
grid as run-length rows and its links and signs. Each tick it reads the
arrow keys (`keydown2`), moves a quarter tile unless the tile ahead is a
wall, sets `player.position` and keeps a free camera behind and above the
player. This is the walker shape a real 5.12 walked with in an earlier lab.

## Houses and other interiors

A 3D client given a world switch at every door stalls: each `g3dworld`
resends the whole terrain, needs a second phase 2 to light it, and a world
with no terrain, water or Sun froze a real 5.12 (Graal_analysis
`torque/3d-worldswitch-terrain-lighting-512-2026-11-26.md`, T3D-162). So the
converter follows the links: the start world and each gmap are hubs, and
every other level reached from a hub by links (through other interiors,
for a house's cellar) goes into that hub's world, laid out in rows starting
1536 tiles south of it. A level reached from two hubs goes to the first
(the start world, then gmaps by name). Entering is then a walk inside one
world:

- **Not drawn from outside.** A 5.12 draws nothing whose centre is over
  about 1000 to 1050 units from the camera (T3D-075), and 1536 keeps every
  interior well past that from the land and from the other interiors.
- **Not sent from outside, with `graal3d_scope = on`** in the server's
  options: seihai-gserver then ghosts an object only within 1100 of the
  player. Each world file has a `portal` line a door, so while the player
  stands at a door the room behind it is in scope already and appears the
  moment the player is through; the land outside comes back the same way.
  Without scoping every interior is sent at join, but still not drawn.
- **The walker** keeps each level's walk grid in its own coordinates and
  the place it stands in the world; a link into an interior of the same
  world is a move, and `g3dworld` is sent only when the server world
  changes (a door from one gmap's interior into another gmap).

Every interior also keeps its own `config/graal3dworlds/<level>.txt`, so a
TCP warp into it still takes a 3D client there; the walker does not know of
such a warp. `--separate-interiors` keeps the old layout, every interior a
world of its own. More than 48 x 58 interiors under one hub do not fit
inside seihai-gserver's 100 000-unit position limit; the rest stay worlds of
their own (listed in the warnings).

## What the tiles show, and the 3D built from it

A tile number says nothing about what it is a picture of, but the
playerworld's object sets do: every stamp in a `GOBJSET01` file or a 1.x
`objects.dat` has a name ("big tree", "house roof", "cave entrance",
"rock platform", "fence", "stone way") and the tiles it is made of. With
`--objects DIR` each tile gets the kind most of its stamps' names vote for
(tree, shrub, rock, house, door, castle wall, fence, cliff, path, water,
bridge, stairs, furniture), checked against its tiledefs type: a walkable
tile under a tree's name is the crown's ground, a non-water tile in a pond
stamp is the shore. Tiles no stamp uses keep their tiledefs type.
`--meaning-maps` writes `PW3D-MAPS/<level>.png`, one pixel square a tile
coloured by its kind, to check the reading against the 2D level.

From the kinds the 3D is extrapolated:

- **Height.** A 3/4-view level draws a drop as a cliff face below the
  higher ground. The ground is split into regions at cliffs, each vertical
  run of cliff tiles votes that the region above stands `run x 0.75` higher
  than the one below, and the heights are solved from the largest region
  out. Stairs sit half way; water and lava are sunk; a pond's rim is a low
  edge.
- **Trees.** Each patch of tree tiles is split into trees of about 45 tiles
  (a 2D tree is about 6 x 7); each gets a trunk at the bottom of the 2D
  sprite and a rounded crown in the tiles' colour, as tall as the sprite.
- **Houses.** House and door tiles, with the unnamed walls touching them,
  become a house: walls as tall as the 2D front (the rows below the door's
  top), a gabled roof in the roof tiles' colour along the longer side, and
  a door on the south face where the door tiles are (or a doorstep stands
  out of the front).
- **Shrubs, rocks, furniture, fences, castle walls.** Low rounded lumps,
  boxes, a 1-tile and a two-wall-heights rail.

Without `--objects` only the tiledefs types are used, which gives the flat
layout with raised walls described above.

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

The conversion lives in `crates/lat-core` (`lat_core::pw3d`: `meaning`,
`scene`, `shape`, `convert`, `walker`), with its unit tests (cliff heights,
houses, trees, outward faces, 16-bit splits); this crate reads the command
line and writes the files. It depends on no other tool, only on `lat-core`,
and on `seihai-core` (pinned once in the root `Cargo.toml` by tag, v0.7.2)
for the tests, which build levels with its tile codes and compile the
walker with its GS2 compiler. The tests
run the binary on a made-up playerworld and check the worlds, shapes, links
and that seihai-core compiles the walker, as the gserver does at link time.
