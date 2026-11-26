# seihai-level

Levels offline: convert between `.nw`, `.graal`, `.zelda` and Tiled `.tmx`, paint a level to PNG (NW2PNG's job), and LevelFiller theme fills.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools level ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-level    # target/release/seihai-level
cargo test -p seihai-level
target/release/seihai-level convert old.graal new.nw
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (level, tmx, render, levelfill, files)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
level convert IN OUT.nw|OUT.graal|OUT.zelda|OUT.tmx [--sheet PNG]
    Reads a .nw (GLEVNW01), .graal (GR-V1) or .zelda (Z3-V1) level, chosen
    by the file's first bytes, or a Tiled .tmx map, and writes the format
    OUT's extension names: GLEVNW01 text, GR-V1.03, Z3-V1.04 or a Tiled
    map (one 16-pixel tile layer per board layer on the tile sheet --sheet
    names, default pics1.png beside the map; links, signs, NPCs, baddies
    and chests as objects carrying their level lines in an nw property).
    A .tmx is read back from its CSV layers and those nw properties. Board, links, signs, baddies, NPCs
    and chests are kept; comments and a GR-V1.04 HEIGHTS block are not.
    A binary level has one layer, and a .zelda has no NPCs or chests and
    numbered link arrivals; what is dropped is printed as a warning.
level render IN OUT.png [--sheet PNG] [--scale N] [--npcs] [--images DIR]
    Paints the level's board (every layer) to a PNG, 1024 pixels a side at
    --scale 1 (default), or 1024/N with N = 2, 4, 8 or 16. --sheet is the
    tile sheet (default: Seihai's built-in pics1.png). A chest is the 2x2
    closed or open tiles (magenta left out); one whose x or y is outside
    0 to 62 is skipped. A sign is a 2 by 1 mark with its first line, except
    a sign at 0,0, which is not drawn. --npcs also draws each visible level
    NPC: an image (.png, .gif or .mng, first frame), or a gani when the
    image ends in .gani or the script has a literal showcharacter,
    setcharani or setani (idle when showcharacter names nothing). That
    read is not a GS1 run: a command inside a quote or a comment is not
    drawn, and a name built at runtime is not drawn. Files are found by
    name in --images DIR (default: the level's
    folder and below), any letter case; body.png and head0.png fall back
    to Seihai's built-in pictures, and a stock gani such as idle.gani too.
level fill THEME.nwt LEVEL... (--out DIR | --in-place) [--seed N] [--edges]
           [--log FILE] [--across GMAP]
    LevelFiller: scatters the theme's objects over each GLEVNW01 level, rule
    by rule in theme order, only where the whole object covers the rule's
    replace tile (|tile, else repbase, else base). Every line but the
    layer-0 BOARD rows is kept as it was. --seed makes a fill repeatable
    (default 0; each level continues the same sequence). --edges also tries
    positions over the level's edge (they never fit; LevelFiller's
    "keep inside" off). --log writes what each rule placed.
    Complex objects run their script (setsize, add, return, if, while,
    for, r, c, round); their debug and msg lines are printed and logged.
    Border objects (.top=, .left=, .bttm=, .rght= as percent,object) are
    placed along the outer edges of each rule's patches, where the whole
    border fits on the replace tile. --across GMAP lets objects (with
    --edges) and borders run into the neighbouring levels that the gmap
    places next to a level, when those levels are in the same batch; all
    levels are written after the whole batch is filled.
```
