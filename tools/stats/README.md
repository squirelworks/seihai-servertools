# seihai-stats

Leaderboard pages (one HTML page per mode) from a seihai-gserver world's accounts, gserverstat's job. Passwords never reach a page.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools stats ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-stats    # target/release/seihai-stats
cargo test -p seihai-stats
target/release/seihai-stats path/to/accounts out kills
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- setup-core (stats)
- seihai-core: None. setup-core does not use seihai-core, so this tool builds without it.

It does not depend on any other tool in `tools/`.

## Usage

```text
stats ACCOUNTS OUTDIR [MODE...]
    Leaderboard pages (one MODE.html each) from a seihai-gserver world's
    accounts folder. Modes: kills, deaths, rupees, highap, lowap, maxpower,
    power, arrows, bombs, glovepower, swordpower, shieldpower (default all).
    Passwords are never read into a page.
```
