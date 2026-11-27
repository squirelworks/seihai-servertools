# seihai-servertools: the 3D tools explained

How to turn a 2D Graal playerworld (or a generated one) into a 3D world that
seihai-gserver serves to the Torque-era 3D clients (4.20r6, 4.20r8, 5.x), how
to add your own models and pictures to it, and how to edit it offline in the
3D level editor. Written against **seihai-servertools 0.1.0** (released
2026-11-27).

Related reading:

- each tool's own README in the repo: `tools/pw3d`, `tools/regions`,
  `tools/asset3d`, `tools/edit3d`, `tools/gen`, and
  `scripts/community-worlds/README.md`;
- Graal_analysis `graal3d/community-guide.md`: the full guide to the 3D
  mode itself (world file, terrain, scoping, scripting, writing a server);
- seihai-gserver `docs/GRAAL3D.md`: the server side, including "Known limits
  on a real 5.12".

---

## Contents

1. What servertools is
2. Getting it
3. The 3D tools at a glance
4. How a 3D world is laid out
5. A generated world in one command (`seihai-gen`)
6. Converting a 2D playerworld (`seihai-pw3d`)
7. Interiors, doors and `graal3d_scope`
8. Big overworlds as regions (`seihai-regions`)
9. Your own models and pictures (`seihai-asset3d`)
10. The offline 3D level editor (`seihai-edit3d`)
11. Serving the result with seihai-gserver
12. The community test worlds
13. Troubleshooting
14. Known limits

---

## 1. What servertools is

**seihai-servertools** is the Seihai project's toolbox for running and
building Graal servers on the Seihai server stack (seihai-gserver,
seihai-listserver, seihai-npcserver, seihai-rc). Besides the 3D tools it has
the one-step lab start, the server-from-scratch and go-public setup wizards,
the world checker, the LAT editors for a live server, package and level
tools and a stress rig. Licence: MIT or Apache-2.0.

**The 3D tools are standalone programs.** You do not need the rest of
servertools, or the `seihai-servertools` front door, to use them. Each is
one self-contained executable with no run-time dependencies:

| Program | Job |
|---|---|
| `seihai-gen` | generate a terrain world, or a 3D world from a gmap |
| `seihai-pw3d` | convert a 2D playerworld to 3D |
| `seihai-regions` | big overworlds as regions |
| `seihai-asset3d` | convert models and pictures |
| `seihai-edit3d` | the offline 3D level editor (window and command line) |

Copy the ones you want anywhere (or onto `PATH`) and run them on their own.
Every command in this guide is written for the standalone programs.

(For completeness: the all-in-one `seihai-servertools` program runs the same
code with the same words, so `seihai-servertools pw3d ...` equals
`seihai-pw3d ...`. You never need it for the 3D work.)

## 2. Getting it

### Download a release

The 0.1.0 release has archives for Linux x86_64, Windows x86_64, macOS Intel
and macOS Apple Silicon, plus `SHA256SUMS`:

<https://github.com/squirelworks/seihai-servertools/releases/tag/v0.1.0>

```
sha256sum -c SHA256SUMS --ignore-missing
tar xzf seihai-servertools-linux-x86_64.tar.gz      # or unzip the Windows .zip
```

Each archive holds every `seihai-<tool>` program side by side (plus the
all-in-one `seihai-servertools`). For the 3D work, take out just the
programs you need, for example `seihai-pw3d` and `seihai-edit3d`
(`seihai-pw3d.exe` and so on on Windows); each runs on its own. The builds
are not signed: on macOS the first start may need right-click, then Open.

### Build from source

```
git clone https://github.com/squirelworks/seihai-servertools
cd seihai-servertools
# just the 3D tools, each its own program in target/release/
cargo build --release -p seihai-gen -p seihai-pw3d -p seihai-regions -p seihai-asset3d -p seihai-edit3d
cargo build --release -p seihai-edit3d --no-default-features   # editor without the window
```

On Linux the editor window needs the usual X11 or Wayland libraries at run
time (on a minimal system that includes `libxkbcommon-x11` and an OpenGL
driver). The command-line tools need nothing.

### What else you need

- **seihai-gserver 0.2.0 or later** to serve the worlds. Interiors (`portal`
  lines) and regions (`regions` line) need 0.2.0. The generated terrain
  worlds and asset worlds also run on seihai-gserver-v2 4.0.0-seihai.6 with
  its Graal 3D add-on.
- **A 3D client**: 4.20r6, 4.20r8 or 5.x. The tools were checked against a
  real 5.12.

## 3. The 3D tools at a glance

| Tool | What it does | Typical use |
|---|---|---|
| `seihai-gen terrain --3d` / `seihai-gen 3d` | a height-field world (new, or from a gmap with a `HEIGHTMAP` block) as a 3D ground with streamed terrain, trees and rocks | a quick test world; a TerrainGenerator world in 3D |
| `seihai-pw3d` | every level of a 2D playerworld as 3D, with a walker weapon, sign boards, and house interiors in the same world | bring an existing 2D world into 3D |
| `seihai-regions` | cut a big overworld into regions, count its density, or build a demo overworld | worlds bigger than one terrain, or with many objects |
| `seihai-asset3d` | glTF/GLB, OBJ, STL, 3DS and pictures as shapes a 5.12 draws, added to a world | your own houses, props, signs |
| `seihai-edit3d` | an offline 3D editor with a live preview: objects, spawn, sun, terrain brushes; also scriptable | place things, sculpt and paint terrain |
| `scripts/community-worlds/build.sh` (in the source repo) | builds the project's 3D test worlds as reproducible tarballs | try the 3D mode without making anything |

The usual order: **make a world** (gen or pw3d), **add assets** (asset3d),
**edit** (edit3d), **serve** (seihai-gserver).

## 4. How a 3D world is laid out

All the tools write, or work on, a **seihai-gserver world folder**:

```
myworld/
  config/
    server.txt                  server settings (graal3d = true, ...)
    graal3dworld.txt            the default 3D world
    graal3dworlds/<name>.txt    any other 3D worlds (one per gmap / level)
    assets3d.txt                seihai-asset3d's list of added assets
  files/                        shapes (.3ds), textures, sky faces, terrain;
                                served to clients by file name
  levels/                       the 2D levels and gmaps (2D players share them)
  weapons/                      GS2 weapons, such as pw3dwalk.gs2
  backups/                      seihai-edit3d's terrain backups (not served)
```

The world file (`config/graal3dworld.txt`) is plain text, one entry a line,
`#` for comments. The lines you will meet most:

| Line | Meaning |
|---|---|
| `object NAME SHAPE X Y Z [AX AY AZ ANGLE]` | a shape in the world (name unique in the world) |
| `objectfor ACCOUNTS NAME SHAPE X Y Z ...` | the same, sent only to those accounts |
| `spawn X Y Z` | where players start |
| `sun DX DY DZ R G B A AR AG AB AA` | light direction, light colour, ambient colour |
| `sky FACES [DISTANCE [FOG R G B]]` | six sky-box faces, comma-separated |
| `terrain FOLDER [SQUARE] [repeat] [WxH]` | streamed terrain |
| `regions FOLDER CxR [CHUNKS] [SQUARE]` | terrain and objects from region folders |
| `water X Y Z SX SY SZ [TEXTURE [TYPE]]` | a box of water; its surface is at Z + SZ |
| `portal X Y Z RADIUS TX TY TZ` | a door to a far part of the same world |
| `weapon3d NAME [FILE]` | a GS2 weapon sent to every 3D client |
| `playershape SHAPE`, `camera X Y Z ...`, `movetype ...`, `replicator ...` | see the community guide |

**Units and axes:** one unit is one 2D tile, z is up, +y is north. A 2D tile
(x, y) is 3D (x, −y). A positive turn about +z is clockwise seen from above
(+y swings toward +x), the clients' sense, not the right-hand rule. Turn
angles in the world file are radians; the editor's `--turn` takes degrees.

## 5. A generated world in one command (`seihai-gen`)

The fastest way to a working 3D server:

```
seihai-gen terrain myworld --world isle --size 4x4 --3d --stream --playerworld
seihai-gserver --world myworld
```

That writes:

- `levels/`: the 2D levels (`isleA00.nw` ...) with heights, edge links and
  `isle.gmap`;
- `files/3d/`: `isle.3ds` (a mesh ground), `isle_tree.3ds`, `isle_rock.3ds`
  and `isle_terrain/` (the streamed terrain: 256 `heights<x>-<y>.dat`
  chunks, `heights.info` and four 256 × 256 textures);
- `config/graal3dworld.txt`: sun, the `terrain` line, trees and rocks on the
  ground, the spawn on flat grass near the middle;
- `config/server.txt`: `name = 3 isle` (a 5.x links 3D only to a server whose
  name starts with `3 `), `graal3d = true`, `graal3d_pushterrain = true`
  and the start level under the 3D spawn.

Options worth knowing:

| Option | Effect |
|---|---|
| `--size WxH` | levels across and down (default 2x2) |
| `--seed N`, `--base N`, `--height N`, `--chaos F`, `--islands` | the height field (TerrainGenerator's knobs) |
| `--vscale F` | 3D height per height unit (default 0.25) |
| `--trees N` | tree tries per level (default 4) |
| `--stream` | also write streamed terrain (wanted for 4.20r6 and 5.x) |
| `--playerworld` | lay it out as a seihai-gserver folder that serves as it is |
| `--playerworld --v2` | the GServer-v2 layout instead (`world/`, `world/3d/`, `config/foldersconfig-3d.txt`) |

`seihai-gen 3d GMAP DIR` does the same from an existing gmap that has a `HEIGHTMAP`
block, including those from the original TerrainGenerator; `GENBASE`,
`GENHEIGHT` and `GENSEED` in the gmap set the water level, height bands and
tree places.

Terrain chunks are written in the client's own encoding, so the stamp a
returning client sends back matches and only changed chunks are sent again.
Always make chunks with these tools rather than by hand.

## 6. Converting a 2D playerworld (`seihai-pw3d`)

```
seihai-pw3d INPUT OUT [--start LEVEL] [--at X,Y] [--tileset PICS1.PNG]
            [--tiledefs FILE] [--objects DIR] [--images DIR]... [--meaning-maps]
            [--wall-height F] [--colour-bits N] [--separate-interiors] [--force]
```

`INPUT` is a folder of `.nw`, `.graal` or `.zelda` levels and gmaps
(subfolders are searched). `OUT` becomes a seihai-gserver world folder.

### Why it builds 3D content at all

A 3D client draws none of a level's tiles, signs or NPCs once it is in 3D. It
does not move its own player, has no collision and runs no level links. What
it does draw is shapes with colour materials, a sky and a sun, and it runs
`weapon3d` scripts. So the converter builds shapes from the tiles, and a
walker weapon does the walking, the walls and the links.

### Step by step

1. **Gather the playerworld** into one folder: levels, gmaps, and if you have
   them the object sets (`GOBJSET01` files or `objects.dat`) and the pictures
   NPCs use.
2. **Run a first pass** with the defaults:

   ```
   seihai-pw3d myplayerworld out3d --start onlinestartlocal.nw
   ```

   Without `--start` it picks `onlinestartlocal`, then `start`, then the first
   level by name. `--at X,Y` sets the start tile (default the middle, or the
   nearest floor).
3. **Read `out3d/PW3D-REPORT.md`**: per world and per level, what came across,
   what was approximated, what was dropped, and links that lead to levels not
   in the folder.
4. **Turn on object reading** for real 3D shapes:

   ```
   seihai-pw3d myplayerworld out3d --objects myplayerworld/tileobjects \
               --images myplayerworld/images --meaning-maps --force
   ```

   `--objects` reads each tile's meaning from the object sets' stamp names
   (tree, house, door, cliff, path, water, fence ...) and builds hills from
   cliffs, round-crowned trees, roofed houses with doors, shrubs and rocks.
   `--meaning-maps` writes `PW3D-MAPS/<level>.png`, one coloured square per
   tile by what it was read as, so you can check the reading against the 2D
   level.
5. **Use your own tileset** if you have one: `--tileset pics1.png` (colours)
   and `--tiledefs FILE` (walls and water; text or `arrays.dat`). Otherwise
   Seihai's built-in tileset and tiledefs are used.
6. **Tune**: `--wall-height` (tiles, default 1.5), `--colour-bits` (1 to 8 a
   channel, default 3; fewer bits means fewer materials).
7. **Add a `config/server.txt`** and accounts (pw3d does not write them), then
   serve (section 11). Turn `graal3d_scope = on` if the world has interiors.

### What comes across

| 2D | 3D | |
|---|---|---|
| tile grid | one coloured `.3ds` ground a level | layout lossless |
| tile picture | the tile's average colour | approximated |
| walls | raised blocks (`--wall-height`); low walls half as tall | approximated |
| water, lava | sunk 0.35 and 0.15 | approximated |
| NPCs with an image | an upright cut-out of the picture, as big as in 2D; a figure for ganis, sheets or missing pictures | scripts dropped |
| baddies, chests, signs | simple shapes | behaviour and items dropped |
| signs' text | the words on a board above the sign in 3D capital letters (up to 8 lines) | lower case, colours, rare symbols lost |
| links | tinted tiles; the walker takes them | lossless (links out of the folder listed and dropped) |
| gmap | one world with its levels side by side | lossless layout |
| interiors (houses, cellars, caves) | placed in the world they are entered from, joined by `portal` doors | sky and light are the outdoor ones |
| any other level | its own world, `config/graal3dworlds/<level>.txt` | |

### The resulting folder

For a small sample (a start level and one house) the output looks like this:

```
out3d/
  PW3D-REPORT.md
  config/graal3dworld.txt               start level's world, house inside it
  config/graal3dworlds/sample_house.txt the house as its own world too (TCP warps)
  files/pw3d_sample_start.3ds           the start level's ground
  files/pw3d_sample_house.3ds           the interior's ground
  files/pw3d_player.3ds
  files/pw3d_sky0.png ... pw3d_sky5.png
  levels/sample_start.nw, sample_house.nw   the 2D levels, unchanged
  weapons/pw3dwalk.gs2                  the walker
```

and the world file:

```
sun 0.57735 0.57735 -0.57735 0.85 0.85 0.8 1 0.4 0.4 0.45 1
sky pw3d_sky0,pw3d_sky1,pw3d_sky2,pw3d_sky3,pw3d_sky4,pw3d_sky5 1000
skyoptions 1 0 0 0.45 0.65 0.95
playershape pw3d_player.3ds
movetype interpolate
object PW3D_L0 pw3d_sample_start.3ds 0 0 0
# Interiors, entered by their doors (links).
object PW3D_I0 pw3d_sample_house.3ds 0 -1600 0
# Doors between its parts, for graal3d_scope.
portal 32.0 -1663.5 0 5.0 31.5 -26.5 0
portal 32.0 -25.5 0 5.0 31.5 -1660.0 0
spawn 32.00 -32.00 1.00
weapon3d PW3DWalk weapons/pw3dwalk.gs2
```

### The walker (`PW3DWalk`)

`weapons/pw3dwalk.gs2` carries each world's walk grid (run-length rows), its
links and signs. Each tick it reads the arrow keys, moves a quarter tile
unless the tile ahead is a wall, sets the player's position, and keeps a
camera behind and above the player (in front of walls). Walking runs at
about 5 tiles a second by the clock. A link inside the same world is a move;
`g3dworld` is sent only when the server world changes. Signs are also said
in chat, which a 5.12 does not draw in 3D (hence the boards).

**Keep `OUT` local.** It holds copies of the playerworld's own levels; do not
publish it.

## 7. Interiors, doors and `graal3d_scope`

A world switch at every door is slow on a 5.12: it resends the whole terrain,
needs a second lighting phase, and a switch into a world with no terrain,
water or Sun froze the client. So pw3d keeps interiors in the same world:

- The **start level and each gmap are hubs**. Every level a hub's links
  reach (and levels linked on from those, like a cellar) goes into that hub's
  world, in rows starting **1536 tiles south** of it and 1536 apart. A level
  reached from two hubs goes to the first (the start world, then gmaps by
  name).
- **Not drawn from outside:** a 5.12 draws nothing whose centre is more than
  about 1000 to 1050 units from the camera, and 1536 is well past that.
- **Not sent from outside, with scoping on.** With `graal3d_scope = on` in
  `config/server.txt`, seihai-gserver ghosts an object only within the scope
  radius (1100) plus hysteresis (200) of the player. Each door gets a
  `portal X Y Z RADIUS TX TY TZ` line: while a player stands within RADIUS of
  the door, the objects round the far side are in scope too, so the room is
  already there when the walker moves the player through, and the land comes
  back the same way on the way out.
- **On a real 5.12** (lab): doors with portals and scoping gave no world
  switch, no terrain resend, no relight and no empty frame; positions landed
  exactly on the link targets. Scoping without portals showed one sky-only
  frame (under 0.23 s) per crossing. Without scoping every interior is sent
  at join but still not drawn.
- **Limits:** inside, the sky and light are the outdoor ones. A TCP warp
  straight into an interior's level goes to that level's own world
  (`config/graal3dworlds/<level>.txt`), which the walker does not know about.
  More than 48 × 58 interiors under one hub do not fit inside seihai-gserver's
  100 000-unit position limit; the rest stay worlds of their own (listed in
  the warnings).
- `--separate-interiors` gives the old layout: every interior a world of its
  own, entered by a world switch.

The scope settings in `config/server.txt`:

```
graal3d_scope = on             # off, on, or auto (auto: worlds over graal3d_scopeauto objects)
graal3d_scopeauto = 2000
graal3d_scoperadius = 1100
graal3d_scopehysteresis = 200
```

## 8. Big overworlds as regions (`seihai-regions`)

One streamed terrain is 256 × 256 squares, and a client holds 4096 ghosts.
For a bigger world, seihai-gserver's `regions` line builds one seamless world
from a grid of region folders `<i>-<j>` (i east, j north, from the south-west),
each with its own terrain chunks and a `region.txt` of object lines in the
region's own coordinates. The terrain window slides round the player and
scoping brings objects in and out, so nothing switches at a region edge.

```
seihai-regions split WORLD.txt --root DIR --into FOLDER [--chunks N] [--out FILE] [--force]
seihai-regions stats WORLD.txt --root DIR [--chunks N] [--reach UNITS] [--budget N]
seihai-regions demo DIR [--regions CxR] [--chunks N] [--towns N] [--town-objects N]
                        [--town-radius U] [--trees N] [--seed N] [--force]
```

- **`split`** cuts an existing world (a `terrain ... repeat WxH` large
  terrain or a plain 16 × 16 one) into regions of N × N chunks (2, 4, 8 or
  16; default 8 = 1024 units), and writes a world file whose `terrain` line
  became `regions FOLDER CxR N S`. Other lines (sun, sky, spawn, water ...)
  stay. Default output: `WORLD-regions.txt` beside the input.
- **`stats`** prints objects per region as a grid and the densest spot
  within 1300 units (scope radius plus hysteresis) against the 3584 world
  objects one client can hold. The budget is per 1300-unit circle, not per
  region.
- **`demo`** builds a whole test overworld: by default 8 × 8 regions of 8
  chunks (8192 units a side), 6 towns of 1500 houses, 150 trees a region, the
  spawn just outside the first town. It writes the world file and
  `files/3d/` but no `server.txt`; add one with `graal3d = true` and
  `graal3d_scope = on`.

Choosing a region size changes nothing on the wire; pick it for editing
(8 chunks is about a 2D level's worth of ground at 3D scale). What matters
is density: a town too dense for one spot is better as its own world behind
a warp. On a real 5.12 the 18,055-object demo walked with no seams or stalls;
the limit was frame rate in dense towns, not memory or the ghost table.

## 9. Your own models and pictures (`seihai-asset3d`)

```
seihai-asset3d add FOLDER INPUT... [--name NAME] [options] [--force]
seihai-asset3d convert INPUT OUT_DIR [--name NAME] [options] [--force]
seihai-asset3d list FOLDER
seihai-asset3d remove FOLDER NAME
seihai-asset3d check FILE...
seihai-asset3d formats
```

### Supported

| Input | What comes across | What is dropped |
|---|---|---|
| `.glb`, `.gltf` (glTF 2.0) | every mesh of the default scene with its node transforms; base colour and base colour texture (PNG/JPEG, embedded, beside it or `data:`) | skins, morphs, animations, normals, PBR maps, second UV set; Draco/meshopt refused |
| `.obj` + `.mtl` | polygons (triangulated), `Kd` colour, `map_Kd` texture | normals, groups, smoothing, other maps |
| `.stl` | triangles, one grey material | (STL has no colours) |
| `.3ds` | meshes, material colours, texture maps | rewritten with safe names and sizes |
| `.png .gif .mng .jpg .jpeg .bmp .tga` | an upright card facing south, 16 pixels a tile, cropped to its opaque part; `--solid` builds it from coloured strips instead | |
| `.dts`, `.dif` | copied after the 5.12's version checks (`.dts` version 24, `.dif` version 44) | their textures are not converted |

### Not supported, and what to do

- **FBX** (closed format), **`.blend`**, **Collada `.dae`**, **PLY**: open in
  Blender and export glTF (`.glb`) or OBJ.
- **MD2, MD3**: the 5.12 has no loader, and shapes do not animate.
- **`.gani`**: shapes do not animate on the 5.12; save the frame you want as a
  PNG and convert that.
- **DDS textures** draw white on the 5.12.

### What it writes, and the limits it keeps

- One `<name>.3ds` and textures `<name>_0.png`, `<name>_1.png` ... in lower
  case (the client lower-cases texture names, drops folder and extension, and
  asks once; any other name never arrives).
- Textures as **8-bit PNG**, at most **1024 px** a side (`--max-texture`),
  stretched to **power-of-two** sides (a 5.12 pads other sizes and the
  picture then fills only part of its face). Alpha cut to 0 or 255 (the 5.12
  has no blending).
- Mesh chunks of at most **65 535** points and faces; at most **250 000
  points a shape** (bigger is refused unless `--force`; simplify first, for
  example with Blender's Decimate).
- Every face has a material (a mesh with none draws nothing).
- **Axes:** glTF and OBJ are +y up (`--up` changes it); the model's front
  (+z) is turned to face south. `--turn` turns further. `--scale` is tiles
  per source unit (default 1, so a 1.8 m glTF person is 1.8 tiles; a 2D
  player is about 3); `--height H` scales to a height instead. The origin
  goes to the middle of the footprint on z = 0 (`--origin centre|keep`).
- **`--colour-only`** bakes textures into face colours (`--colour-bits`) for
  the 4.20r8, which draws no `.3ds` textures, or for worlds that want no
  texture downloads.
- The 5.12 draws both sides of every face; a picture card's back shows it
  mirrored.

### Example

```
seihai-asset3d add myworld chair.glb crate.obj banner.png --height 2
seihai-asset3d list myworld
seihai-edit3d add myworld chair1 chair.3ds 40 -22 0 --ground
```

`add` writes the shapes and textures into `myworld/files/` and a line per
asset into `config/assets3d.txt`:

```
# asset NAME SHAPE WIDTH DEPTH HEIGHT TEXTURES SOURCE
asset crate crate.3ds 2 2 2 - crate.obj
asset banner banner.3ds 2 0 2 banner_0 banner.png
```

That list is for builders; the server does not read it. Re-adding an asset
of the same name replaces it; a file belonging to something else is not
overwritten without `--force`. `remove` deletes an asset's files and line
(world files still placing it will log it as missing). `convert` writes to a
folder of its own, for other server layouts. `check` reports what a 5.12
would have trouble with in any `.3ds`, `.dts`, `.dif` or picture.

Lab result (real 5.12): a glTF column, OBJ crates (JPEG and an oversized PNG
texture), a transparent picture card, a colour-only crate, a `--solid` card
and a 21-letter asset name all drew correctly, textured and facing the
right way.

## 10. The offline 3D level editor (`seihai-edit3d`)

```
seihai-edit3d FOLDER [--world NAME] [--renderer CMD]      # the window
seihai-edit3d worlds FOLDER
seihai-edit3d show FOLDER [--world NAME]
seihai-edit3d add FOLDER NAME SHAPE X Y Z [--turn DEG] [--ground] [--for ACCOUNTS]
seihai-edit3d move FOLDER NAME X Y Z [--turn DEG] [--ground]
seihai-edit3d remove FOLDER NAME
seihai-edit3d spawn FOLDER X Y Z [--ground]
seihai-edit3d terrain FOLDER raise|lower|flatten|smooth|paint X Y RADIUS VALUE
seihai-edit3d render FOLDER OUT.png [--size WxH] [--eye X,Y,Z --yaw DEG --pitch DEG]
                     [--fov DEG] [--renderer CMD]
```

`FOLDER` is the world folder (the one with `config/`). `--world NAME` opens
`config/graal3dworlds/NAME.txt` instead of the default world; a world that
does not exist yet opens empty and is created on save. Shapes and textures
are found by name anywhere in the folder, as the server finds them.

### Why a separate editor

The 5.12 carries Torque's WorldEditor and TerrainEditor, but against a remote
server nothing they do is saved, and the TerrainEditor cannot attach to the
server's terrain. The world lives in the server's files, so the editor edits
those files directly, offline.

### The window

```
┌ world ▾ │ Save │ Undo │ Redo │ Reload files │ Select │ Place │ Terrain │ path ┐
│ Objects      │                                     │ Selected object        │
│ (filter)     │          live preview               │  name, shape, X Y Z,   │
│ Tree1        │                                     │  turn; Copy, Delete,   │
│ rock1        │                                     │  Stand on the ground   │
│ ...          │                                     │ World: spawn, "Spawn   │
│              │                                     │  at the view's centre",│
│              │                                     │  sun dir, light, ambient│
│              │                                     │ Problems               │
└──────────────┴─────────────────────────────────────┴────────────────────────┘
```

- **View:** right-drag turns, middle-drag (or shift + right-drag) moves,
  the wheel zooms. Picking an object in the list centres the view on it.
- **Select** mode: left-click picks the object under the pointer; edit its
  place and turn on the right, or Copy, Delete, Stand on the ground.
- **Place** mode: left-click adds the chosen shape where the pointer meets the
  ground, named after the shape (`tree`, `tree2`, ...).
- **Terrain** mode (only when the world has a `terrain` line): left-click or
  drag applies the brush: raise, lower, flatten to a height, smooth, or paint
  a `heights.info` material. One drag is one undo step.
- **Top bar:** switch world (it asks before dropping unsaved changes), Save,
  Undo and Redo (40 steps, terrain included), **Reload files** (rescans the
  folder for shapes and textures you added or changed, for example after an
  `asset3d add`; it does not reload the world file).
- **Problems** lists what the server would log for this world: unknown lines,
  lines that do not parse, two objects with one name, missing shapes.

### The preview

Built in: `.3ds` shapes in their material colours (no textures yet),
streamed terrain (each material the average colour of its texture, when the
texture is in the folder), water surfaces, lit flat by the world's sun, the
spawn as a yellow box. `.dts`/`.dif` shapes, missing shapes and shapes over
200 000 triangles are magenta boxes and listed under Problems. It uses the
clients' conventions (unit = tile, turn sense, water surface at z + size). It
is for placing things, not a pixel match of the client.

`--renderer CMD` hands each frame to another renderer, called as
`CMD WORLD_FILE ROOT OUT.png EX EY EZ YAW PITCH FOV W H`; if it fails the
window falls back to the built-in preview. Picking needs the built-in one.

### Command-line edits

The same edits without the window, for scripts:

```
seihai-edit3d show myworld
seihai-edit3d add myworld rock1 isle_rock.3ds 30 -100 0 --ground --turn 45
seihai-edit3d move myworld rock1 32 -100 0 --ground
seihai-edit3d remove myworld rock1
seihai-edit3d spawn myworld 22 -106 1 --ground
seihai-edit3d add myworld vipchest chest.3ds 10 -10 0 --for alice,bob     # an objectfor line
```

- `--ground` makes Z a height above the terrain under X Y. When there is no
  terrain there (a pw3d world's floor is a shape, not terrain) it says so
  and keeps Z as given.
- `--turn` is degrees about +z, clockwise seen from above.

### Terrain editing

```
seihai-edit3d terrain myworld raise   20 -108 6 3     # up to 3 units at the centre
seihai-edit3d terrain myworld lower   20 -108 6 3
seihai-edit3d terrain myworld flatten 20 -108 6 4     # toward height 4
seihai-edit3d terrain myworld smooth  20 -108 6 0.5   # strength 0 to 1
seihai-edit3d terrain myworld paint   20 -108 4 2     # material 2 = 3rd line of heights.info
```

The brush works within RADIUS units of world X Y with a smooth fall-off (full
at the centre, none at the edge). It prints how many squares changed; a
brush that changes nothing saves nothing. Terrain holes are not kept yet.

### Saving and dated backups

- The **world file** keeps everything as written: comments, blank lines,
  order and unknown lines byte for byte; a changed line keeps its indent and
  trailing comment; a new object goes after the last object line. The old
  file is kept as `config/graal3dworld.txt.bak-YYYYMMDD-HHMMSS` (`-2`, `-3`
  ... for more saves in the same second), and the new one is written through
  a temporary file and a rename.
- When the **terrain** changed, its folder is first copied to
  `backups/<terrain folder>.bak-YYYYMMDD-HHMMSS` under the world folder (not
  beside the terrain, where the server would serve the copy), then all 256
  chunks and `heights.info` are written in the client's own encoding. A
  returning client then downloads only the chunks that changed.
- To roll back, copy a backup over the file or folder. Each command-line
  edit is one save and so one backup; the window saves when you press Save.

### Reload and live servers

The server reads the world file at start. After editing offline, restart
the server (or edit while it is stopped). A 5.12 fetches each file once a
session, so a changed shape or texture reaches a player after they link
again; terrain chunks are re-sent by stamp.

For live edits on a running seihai-gserver there is `g3dedit` (in the server,
`graal3d_edit = staff`), which changes object lines in game; terrain still
needs this editor.

### Render to a picture

```
seihai-edit3d render myworld view.png --size 800x500 --eye 20,-130,25 --yaw 0 --pitch 30
```

Without `--eye` it looks down at the spawn. Handy for checking a world in a
script or CI.

## 11. Serving the result with seihai-gserver

1. Make sure `config/server.txt` has at least:

   ```
   name = 3 My World          # 5.x link 3D only to a server whose name starts "3 "
   graal3d = true
   graal3d_pushterrain = true # with streamed terrain: a 5.12's first visit is textured
   graal3d_scope = on         # pw3d interiors and regions worlds
   start_level = mystart.nw
   ```

   `seihai-gen ... --playerworld` writes one for you; pw3d and `regions demo` do not.
   Add accounts as for any world.

2. Start it:

   ```
   seihai-gserver --world myworld
   ```

   (If you also use servertools' lab starter, `seihai-lab start --world LAB
   --rust myworld` starts it next to a lab; give it its own port.)

3. **Read the start-up log.** seihai-gserver looks up every shape and texture
   the world file names and checks each for the faults that crashed or
   blanked a real client (`.3ds` materials and bounds, `.dts` version, PNG
   sizes and depths, terrain texture sizes, `heights.info` names, a second
   `terrain` line). `graal3d_shapecheck = skip` leaves faulty shapes out.

4. Point a 3D client at it. A 5.12 logs in as a guest over TCP, is warped to
   the `3 ` server, opens UDP on the TCP port + 1, then downloads ghosts,
   lights the mission and shows the 3D view.

For seihai-gserver-v2 (C++ fork) the options go in `config/serveroptions.txt`
without underscores (`graal3d`, `graal3dscope`, `graal3dpushterrain`), with
`hq_level = 4` in `config/adminconfig.txt`; use `seihai-gen ... --playerworld --v2`
for its layout. It lacks regions, `portal` and several worlds for now.

## 12. The community test worlds

```
scripts/community-worlds/build.sh OUT --graal-analysis ../Graal_analysis --date 2026-11-27
```

builds five tarballs, each with the world, a README (what it shows, settings,
expected client, limits), a licence note, the build commits and
`SHA256SUMS`:

| Tarball | World |
|---|---|
| `seihai3d-isle-terrain-<date>` | a 4 × 4-level island with streamed terrain, seihai-gserver and gserver-v2 layouts |
| `seihai3d-regions-overworld-<date>` | the 8192-unit, 8 × 8-region overworld, 6 towns, 18,055 objects |
| `seihai3d-scoping-grid-<date>` | 20,164 cubes on flat terrain (distance scoping) |
| `seihai3d-interiors-sample-<date>` | a small original playerworld with a house and its interior joined by `portal` doors |
| `seihai3d-conformance-worlds-<date>` | the conformance suite's 40 redistributable cases |

They are reproducible (same commits and date, same checksums) and generated
from nothing by the project's own tools. The last two need a Graal_analysis
checkout; without it they are skipped.

## 13. Troubleshooting

| Symptom | Likely cause and fix |
|---|---|
| The client never enters 3D | the server name does not start with `3 `, or `graal3d` is off |
| Terrain white on the first visit | turn on `graal3d_pushterrain`, or relink |
| Every chunk downloads every visit | the chunks were not made by the tools; regenerate with `--stream` or save through `seihai-edit3d` |
| A shape does not show | a `.3ds` with no material, a `.dts` not version 24, or a missing file: read the start-up log, run `seihai-asset3d check` |
| Textures draw white | 16-bit PNG, BMP, TGA, DDS, over 1024 px, or a name with capitals or a folder; re-add with `seihai-asset3d` |
| A picture fills only part of its card | a non-power-of-two texture made elsewhere; convert with `seihai-asset3d` |
| The player does not move | the 3D client does not walk by itself; the walker weapon must be sent (`weapon3d PW3DWalk ...`) |
| Players walk through walls | no client collision; only the walker stops them |
| Houses visible from outside, or slow joins | `graal3d_scope` is off |
| One sky-only frame at each door | scoping on but no `portal` lines; rerun with seihai-pw3d 0.1.0 or later |
| Client freezes after a warp | the target world has no terrain, water or Sun; give every world a Sun |
| Black view in a big world | ghost table filled before the camera: turn on scoping |
| A changed file does not show | the client fetches a file once a session; relink. To roll back, give the old file a new modification time |
| `--ground` leaves Z unchanged | no terrain under that point (pw3d floors are shapes); give Z yourself |
| The editor's Terrain button is greyed | the world has no `terrain` line |
| The editor window will not open on Linux | missing X11/Wayland libraries (for example `libxkbcommon-x11`) or no OpenGL; the command-line edits and `render` still work |
| Asset refused as too big | over 250 000 points: decimate it, or `--force` at your own risk |

## 14. Known limits

**3D clients (real 5.12):** 4096 ghosts a client (the server sends at most
3584 world objects); one terrain a world, 256 × 256 squares; draw distance
about 1000 units; no client collision; shapes do not animate; no alpha
blending; 2D chat and sign text are not drawn in 3D; a world switch into a
world with no terrain, water or Sun freezes the client; full screen stays at
"Connecting...".

**seihai-pw3d:** NPC scripts, baddy behaviour, chest items and music are
dropped; each tile is one flat colour; interiors use the outdoor sky and
light; a TCP warp into an interior goes to its separate world, which the
walker does not know; up to 48 × 58 interiors a hub. Under Wine with no real
keyboard, the arrow keys did not reach the walker in the lab, so lab runs
used a fixed route.

**seihai-asset3d:** no FBX, Collada, PLY, MD2/MD3, ganis or DDS; no
animation, normals or PBR maps; `.dts`/`.dif` textures are not converted;
colour-only output not yet tried on a real 4.20r8.

**seihai-edit3d (0.1.0):** a scaffold that works, but has not yet been used
on a real client's world beyond the lab terrain checks:

- no textures in the preview, no sky box, `.dts`/`.dif` shown as boxes;
- `replicator`, `objectfor` accounts, `weapon3d` and `camera` lines are kept
  and shown but not edited in the window;
- no dragging objects in the preview, no grid;
- terrain holes are not kept;
- regions worlds: objects in `region.txt` files are not opened (edit the
  world before `split`, or the `region.txt` by hand);
- in 0.1.0 it lists `portal` and `regions` lines under Problems as unknown,
  though they are valid seihai-gserver 0.2.0 lines (they are kept unchanged
  on save); fixed after 0.1.0;
- in 0.1.0 two terrain saves within the same second share one backup folder
  name, so the second overwrites the first backup; wait a second between
  scripted terrain edits on 0.1.0. Fixed after 0.1.0 (`-2`, `-3` ... as for
  the world file);
- edits are offline: restart the server to pick them up.

**seihai-regions:** a world is at most 256 chunks a side (32 768 units);
`demo` writes no `server.txt`; the region size is for editing only.
