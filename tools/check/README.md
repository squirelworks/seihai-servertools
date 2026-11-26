# seihai-check

The world and config checker: every level, gmap and gani parsed with Seihai's own parsers, missing levels, images and sounds, NPC scripts, packages, and the setup checks on each server folder. Exit code 1 on errors, so it can run in CI.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools check ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-check    # target/release/seihai-check
cargo test -p seihai-check
target/release/seihai-check --json path/to/world
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (world checks), setup-core (setting checks)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
check [--json] [--client LABEL] [--client-reach] [--make-gif] WORLD
check --list-clients

check: reads every level, gmap and gani under WORLD and reports parse errors,
links or gmap cells that point at missing levels, and images or sounds a
level NPC or a gani names that are not in the folder. It also runs the setup
checks on every servers/<name>/config/serveroptions.txt (and the accounts/ of
public folders) and on seihai-listserver.toml: a setting `setup` would refuse
to save is an error. Exit code 1 on errors (a missing image or sound is a
warning). --json prints one JSON object instead, for CI: format (1), the
counts (levels, gmaps, ganis, errors, warnings) and findings, each with level
(error, warning, note), source (world or settings), file, key (or null) and
text. --client LABEL adds shared-profile level/gmap/gani/script warnings for
one build; check --list-clients lists the accepted labels. Unknown script
support remains unknown; package readability and exact per-build vocabulary
are not certified. Without --client the general inventory stays unchanged.
--client-reach also inventories level NPC image references (including
fixed GS1 script names) with PNG art but no GIF counterpart for pre-2.1 clients.
It flags parsed gani files for fallback review using shared profiles' gani
support, without claiming use or choosing a built-in animation, and lists
the profiles without gmap support or the level's format. It lists level NPC
and weapon (weapons/*.txt) scripts that are GS2 only or mix GS1 and GS2, with
the shared profiles that do not run GS2 (before 3.153), and levels with tiles
past column 116 of pics1.png, which the 1856-wide sheet of 1.28b2 to 2.05
lacks. It does not certify script, animation, tile or gmap compatibility.
--make-gif (opt-in) first writes a GIF copy next to each PNG that
--client-reach would report, keeping every pixel; it never replaces a file
and skips art with over 256 colours.
```
