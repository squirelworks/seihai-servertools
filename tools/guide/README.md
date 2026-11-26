# seihai-guide

The guide: how a Seihai server fits together (listserver, gservers,
NPC-servers, RC), every tool of seihai-servertools with when to use it, how
to set up a server from scratch with `setup new`, each server's settings, RC
usage and rights, and troubleshooting. `seihai-servertools` with no words
prints its overview.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools guide ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-guide    # target/release/seihai-guide
cargo test -p seihai-guide
target/release/seihai-guide rc
```

Only this tool is built. The binary needs nothing else at run time.

## Depends on

- Nothing: the guide is text. Its test checks that every folder in `tools/`
  is named in `guide tools`, so a new tool gets a line there.
- seihai-core: None.

It does not depend on any other tool in `tools/`.

## Usage

```text
seihai-servertools guide [TOPIC]

Topics:
  overview         how the servers fit together (the default)
  tools            every tool, what it is for and when to use it
  setup            a server from scratch with `setup new`, step by step
  listserver       the server list: ports, accounts, hosting for others
  gserver          the game server: settings, accounts, staff, the listserver link
  npcserver        server-side scripts: auto, external, the token
  rc               remote control with seihai-rc: logging in, rights, common tasks
  troubleshooting  what to check when something does not connect
  all              every topic, one after the other
```
