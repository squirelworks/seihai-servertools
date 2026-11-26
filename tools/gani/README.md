# seihai-gani

GraalShop movie mode (`GANI0002`): list a movie's actors and keyframes, paint a timeline frame to PNG.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gani ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-gani    # target/release/seihai-gani
cargo test -p seihai-gani
target/release/seihai-gani movie show intro.gani
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (gani, movie, render, files)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gani movie show FILE [--frame N]
    Lists a GraalShop movie (GANI0002): its timeline length and each actor
    (CHAR, SPRITE, SOUND, TEXT) with its keyframes. --frame N prints what
    every actor is at timeline frame N instead (a key keeps its last value;
    dx and dy move in a straight line between keyframes).
gani movie render FILE OUT.png [--frame N] [--size WxH] [--images DIR]
    Paints timeline frame N (default 0) to a PNG, WxH pixels (default
    640x480). SPRITE actors are drawn from the gani's sprite table and CHAR
    actors as a player through their ani gani (default idle, facing dir,
    with the head, body, sword, shield and attr pictures their keys name),
    at dx, dy in pixels, lower layer first. Pictures and ganis are found by
    name in --images DIR (default: the gani's folder and below), any letter
    case. TEXT actors are drawn in the font file their font key names
    (Arial finds arial.ttf or .otf there), else Ubuntu Light, at fontsize
    pixels. A CHAR's colours and chat are not drawn yet.
```
