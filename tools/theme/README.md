# seihai-theme

LFThemeMaker: patch themes for `seihai-level fill` from a recipe.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools theme ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-theme    # target/release/seihai-theme
cargo test -p seihai-theme
target/release/seihai-theme make recipe.tm patches.nwt
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (levelfill)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
theme make RECIPE.tm OUT.nwt [--seed N] [--force]
    LFThemeMaker: #ofObjects random patches of Tile01..Tile15 on Tile00,
    sized and as dense as the recipe says, each with the rule
    QtyPerObject|Tile00.
```
