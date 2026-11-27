# seihai-gen

World generators: height-field terrain (2D, 3D with `--3d`, streamed client terrain with `--stream`), the 3D world from an existing gmap (`gen 3d`, `--playerworld` for a gserver folder), levels from a picture, and dungeons.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gen ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-gen    # target/release/seihai-gen
cargo test -p seihai-gen
target/release/seihai-gen terrain out --world isle --size 2x2
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (generate, terrain3d, terrain_chunks)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gen terrain DIR --world NAME [--size WxH] [--seed N] [--base N] [--height N]
            [--chaos F] [--islands] [--force]
            [--3d [--vscale F] [--trees N] [--stream] [--playerworld [--v2]]]
    A height-field world (TerrainGenerator): WxH levels (default 2x2) named
    <NAME><column><row>.nw with a 9x9 HEIGHTS block each, edge links, and
    NAME.gmap with the generator keys. --height is how far the first split
    may move a point (default 64), --chaos the factor after each split
    (default 0.5), --islands keeps the rim at --base.
    --3d also writes the same ground for a Graal 3D server: NAME.3ds (one
    mesh per level, coloured water, sand, grass and rock by height),
    NAME_tree.3ds, NAME_rock.3ds and NAME_graal3dworld.txt (sun, ground,
    trees on grass, rocks up high, spawn in the middle). --vscale is 3D
    units of height per height unit (default 0.25), --trees the tries per
    level (default 4). --stream also writes NAME_terrain/: the ground as
    streamed terrain for 4.20r6 and 5.x clients (256 heights<x>-<y>.dat
    chunks, heights.info and its textures), and the world file uses it.
    --playerworld lays DIR out as a seihai-gserver world that serves as
    it is (seihai-gserver --world DIR): config/graal3dworld.txt, the
    levels and gmap in levels/, the 3D files in files/3d/ under real,
    lower-cased names, and config/server.txt with 3D on, the terrain
    pushed at login and the start level under the spawn. With --v2 it is
    a GServer-v2 folder instead: the levels and gmap in world/, the 3D
    files in world/3d/, and config/foldersconfig-3d.txt: the lines the
    server's foldersconfig needs.
gen 3d GMAP DIR [--world NAME] [--vscale F] [--trees N] [--stream]
       [--playerworld [--v2]] [--force]
    The --3d files (and with --stream, the streamed terrain) from a gmap that
    already has a HEIGHTMAP block, such as one written by gen terrain or by
    the original TerrainGenerator. NAME defaults to the gmap's file name.
    GENBASE, GENHEIGHT and GENSEED in the gmap set the water level, the
    height bands and where trees go.
gen image PICTURE DIR --world NAME
    Levels from a .png or .gif drawn in GraalLevelGen's twelve colours
    (64 pixels a level, sampled every 6): NAME_a-01.nw ... and NAME.gmap.
    Levels already in DIR are painted over; their links and NPCs stay.
gen dungeon DIR --start NAME (--layout FILE | --maze WxH) [--seed N]
            [--path TILE] [--wall TILE] [--force]
    A dungeon from an X (wall) / . (path) layout file or a maze of WxH
    cells, each cell 8 tiles wide and 11 tall, cut into levels
    NAME<column><row>.nw with edge links and NAME.gmap. Tiles default to
    path 0 and wall 1.

Without --force, terrain and dungeon refuse to replace files in DIR.
```
