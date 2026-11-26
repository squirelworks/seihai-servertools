# seihai-gif2mng

gif2mng: each GIF as a PNG (one frame) or an MNG animation, beside it or in a folder.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gif2mng ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-gif2mng    # target/release/seihai-gif2mng
cargo test -p seihai-gif2mng
target/release/seihai-gif2mng walk.gif --out out
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (anim)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gif2mng [-b] [--out DIR] [--force] GIF...
    Each GIF as a .png (one frame) or .mng animation beside it or in DIR;
    -b makes palette index 2 transparent. An existing file is kept unless
    --force is given.
```
