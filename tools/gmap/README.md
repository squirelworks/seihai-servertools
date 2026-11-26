# seihai-gmap

Gmaps offline: build a `.gmap` from a level list, generator-style names or a linked world's edge links, and paint a whole gmap to PNG.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gmap ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-gmap    # target/release/seihai-gmap
cargo test -p seihai-gmap
target/release/seihai-gmap make world.gmap --links levels start.nw
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (worldmap, render, files)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gmap make OUT.gmap --width N LEVEL...
    Puts the levels row by row in a grid N wide (`-` is a hole).
gmap make OUT.gmap --names DIR
    Places every level in DIR by its name, <world><column><row>.nw
    (world-a01.nw, worldaa12.nw), as the generators name them.
gmap make OUT.gmap --links DIR START
    Walks the edge links from level START in DIR (a link along the left,
    right, top or bottom edge leads to the neighbour there) and puts the
    linked world on one grid.
    Each prints the grid size and what it could not place.
gmap render IN.gmap OUT.png [--dir DIR] [--sheet PNG] [--scale N]
           [--npcs] [--images DIR]
    Paints every level of the gmap into one PNG. Levels are looked up in
    --dir (default: the gmap's folder and below), any letter case. Chests
    and signs as for level render. --npcs and --images as for level render
    (files default to --dir). --scale defaults to 4 (256 pixels a level).
```
