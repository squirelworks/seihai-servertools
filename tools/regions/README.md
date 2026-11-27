# seihai-regions

A big 3D overworld written as a grid of smaller **regions**, for
seihai-gserver's `regions` world line (its `docs/GRAAL3D.md`, "A world built
from regions"). Each region is a folder `<i>-<j>` (i east, j north, from the
south-west) holding its piece of the streamed terrain and a `region.txt` of
object lines in the region's own coordinates. The server stitches them into
one seamless world: the terrain slides round the player and objects are
created and deleted by distance, so a player walks across region edges with
no warp, and a client's 4096-entry ghost table never fills while no one spot
holds more than 3584 objects within the scope reach.

- `split` cuts an existing world (a `terrain ... repeat WxH` large terrain,
  or a plain 16 x 16 one) into regions, and writes the world file that uses
  them. Terrain edges match by construction: the regions are the same
  chunks, renamed. No overlap between regions is needed, because nothing
  switches at an edge.
- `stats` prints the objects in each region as a grid, and the densest spot
  within the scope radius plus hysteresis (1300 units by default), against
  what one client holds.
- `demo` builds a whole test overworld as a gserver folder: hills, open
  fields and dense towns, for lab runs.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools regions ...` runs the same code with the
same words.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-regions    # target/release/seihai-regions
cargo test -p seihai-regions
target/release/seihai-regions demo lab-world
target/release/seihai-regions stats lab-world/config/graal3dworld.txt --root lab-world
```

## Depends on

- lat-core (terrain_chunks for the demo's terrain, pw3d's shape writer for
  its `.3ds` files, terrain3d's texture writer)
- seihai-core comes in through lat-core, at the one pin in the root
  `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of
  `squirelworks/seihai-core`).

It does not depend on any other tool in `tools/`.

## How big a region should be

A region is 2, 4, 8 or 16 terrain chunks a side (256 to 2048 units at the
default 8 metres a square); the whole world is 16 to 256 chunks a side, so
at most 32768 units. The region size changes nothing on the wire: the
client gets the same sliding window and the same scoped objects either way.
Choose it for editing: 8 chunks (1024 units) is about a 2D level's worth of
ground at 3D scale and keeps `region.txt` files small.

What does matter is density. A client holds at most 3584 world objects, and
an object stays ghosted while it is within the scope radius plus the
hysteresis (1100 + 200 units by default). So the budget is per 1300-unit
circle, not per region: `stats` reports the worst circle. A town that needs
more than that in one place is better as its own world behind a warp, the
way a house or cave is.

## Usage

```text
seihai-regions: a big 3D overworld as a grid of regions (seihai-gserver's
`regions` world line).

regions split WORLD.txt --root DIR --into FOLDER [--chunks N] [--out FILE] [--force]
    Cuts WORLD.txt's terrain and objects into regions of N x N terrain
    chunks (2, 4, 8 or 16; default 8): DIR/FOLDER/<i>-<j>/ with its
    heights<x>-<y>.dat and a region.txt of its object and objectfor lines in
    the region's coordinates. The terrain line (`terrain F S repeat WxH`, or
    a plain 16 x 16 one) becomes `regions FOLDER CxR N S`; every other line
    (sun, sky, spawn, water, replicators, ...) stays in the world file, which
    goes to FILE (default WORLD-regions.txt beside WORLD.txt). DIR is the
    server's world root, which the folders in the world file are under.
regions stats WORLD.txt --root DIR [--chunks N] [--reach UNITS] [--budget N]
    The objects in each region and the most within REACH units (default
    1300, the server's scope radius plus hysteresis) of any spot, against
    the BUDGET (default 3584) a client can hold. Works on a world with a
    regions line or a terrain line (cut into N-chunk regions to count).
regions demo DIR [--regions CxR] [--chunks N] [--towns N] [--town-objects N]
            [--town-radius U] [--trees N] [--seed N] [--force]
    A test overworld as a gserver folder: DIR/config/graal3dworld.txt with
    a regions line, DIR/world/3d/regions/ with hilly terrain, a grass,
    rock, sand and path heights.info and textures, houses, trees and rocks
    (.3ds) in DIR/world/3d/. Default 8x8 regions of 8 chunks (8192 units a
    side), 6 towns of 1500 houses within 200 units of their centres, and
    150 trees a region. The spawn is in the first town.

Without --force, split and demo refuse to write over files.
```
