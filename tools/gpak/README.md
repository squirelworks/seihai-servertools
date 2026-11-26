# seihai-gpak

Resource packages for 4.20 and 6.x clients: build a `.gpak` or encrypted `.gpake` for a gserver folder, keeping its key file, or unpack one.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools gpak ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-gpak    # target/release/seihai-gpak
cargo test -p seihai-gpak
target/release/seihai-gpak pack art art.gpake --server path/to/servers/modern
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (gpak)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
gpak pack FOLDER OUT [--server DIR] [--key HEX16] [--keys FILE]
    Packs every file under FOLDER. OUT ends in .gpake (encrypted), .gpak
    (plain) or .zip (plain, for 4.20, which loads .zip but not .gpak).
    --server DIR: a gserver folder (servers/<name>); OUT is written to
    DIR/world/gpaks/ and a .gpake's key to DIR/config/gpakkeys.txt, where
    the gserver finds both. Name OUT in the gserver's gpaks option
    to send it at login. Without --server, a .gpake's key goes to --keys FILE if given and
    is printed. Without --key a .gpake gets a random key.
gpak unpack PACKAGE DIR [--key HEX16 | --keys FILE | --server DIR]
    Writes the files in PACKAGE to DIR, decrypting a .gpake.
```
