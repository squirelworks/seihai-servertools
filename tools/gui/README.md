# seihai-servertools-gui

The window: the lab, setup and LAT editor pages (levels, NPCs, ganis, packages, database, world check). It needs X11 or Wayland libraries at run time on Linux.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gui ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-servertools-gui    # target/release/seihai-servertools-gui
cargo test -p seihai-servertools-gui
target/release/seihai-servertools-gui --world path/to/world --page levels
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- servertools-app (the egui window), which uses setup-core and lat-core
- seihai-core: seihai-core and seihai-draw come in through servertools-app -> lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gui [--world DIR] [--page lab|setup|levels|npcs|database|ganis|packages|check]
    Opens the window: the lab, setup and LAT editor pages.
```
