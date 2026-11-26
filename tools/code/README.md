# seihai-code

Client `.code` caches (`GWEBL001` level caches, `GRESCR01` script caches): show, CRC, dump and pack. Local files only.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools code ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-code    # target/release/seihai-code
cargo test -p seihai-code
target/release/seihai-code crc synthetic.code
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## Depends on

- lat-core (code_cache, gpak keys)
- seihai-core: seihai-core and seihai-draw come in through lat-core, at the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag `v0.7.2` of `squirelworks/seihai-core`, never a branch). Moving the pin moves it for every tool at once, so the workspace keeps one copy of the parsers. lat-core also brings seihai-rc (pinned commit) for the live file link.

It does not depend on any other tool in `tools/`.

## Usage

```text
code show FILE [--key HEX16] [--level NAME]
    GWEBL001 (a level cache): host, signature, mod time, level name, body size.
    The level name defaults to the file name (start.nw-14802.code is start.nw).
    GRESCR01: decrypted and stored-body CRC32. Needs --key to decrypt.
    HEX16 for a script is the two key words from packet 140 or 197, each as a
    little-endian 32-bit value, first word first.
code crc FILE [--domain plain|stored] [--key HEX16]
    Prints only the CRC32. Default stored: the bytes after the magic, before
    decryption, which is the CRC a 6.113 server sends in packet 197. Needs no
    key. Plain is the decrypted body and needs --key. Neither is a
    cache-acceptance check.
code dump FILE --out PLAIN [--key HEX16] [--level NAME]
    Writes the plain level text, or the plain script body.
code pack PLAIN --out FILE [--key HEX16]
    A file that starts with GWEBL001 is packed as a level cache.
    Any other plain file is a script body and needs --key.
Hosts in files are whatever the cache stored (a lab file uses 127.0.0.1
or <host>). This command does not open a connection.
```
