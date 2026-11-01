# seihai-servertools

Tools for running and building a Graal server with the Seihai server stack:
start a lab world with one command, take a server from the lab to the public
with a setup wizard, and the LAT (Level Administration Team) toolkit for
editing a live server's world.

Part of the Seihai server stack:
[seihai-gserver](https://github.com/squirelworks/seihai-gserver) ·
[seihai-npcserver](https://github.com/squirelworks/seihai-npcserver) ·
[seihai-listserver](https://github.com/squirelworks/seihai-listserver) ·
[seihai-rc](https://github.com/squirelworks/seihai-rc) ·
[seihai-servertools](https://github.com/squirelworks/seihai-servertools) ·
[seihai-gserver-v2](https://github.com/squirelworks/seihai-gserver-v2) (GPL fork of GServer-v2, ships first) ·
client: [seihai](https://github.com/squirelworks/seihai).

**Status: scaffold.** Working today: the one-step lab start and a first
world checker. The setup wizard and the LAT editors are planned and laid
out. The plan is in [docs/PLAN.md](docs/PLAN.md), the code layout in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), the ordered to-do list in
[docs/NEXT.md](docs/NEXT.md), operator security notes in
[docs/security/README.md](docs/security/README.md), and the rules every
contributor and coding agent follows in [AGENTS.md](AGENTS.md).

## What is in it

| Tool | Command | State |
|---|---|---|
| One-step lab start: listserver + gservers, stopped together | `seihai-servertools lab start` | working |
| Go-public setup: Join a listserver, Back to lab, undo | `seihai-servertools setup public\|lab\|undo\|status` | command line working ([design](docs/design/go-public.md)) |
| Go-public setup in the window (same steps, Start and check lights) | `seihai-servertools gui --page setup` | working |
| Host a listserver (edits `seihai-listserver.toml`, adds the first account) | `setup host-listserver\|listserver-lab\|listserver-undo`, window | working |
| LAT level editor: open, edit and upload levels on a live server | window page | planned ([design](docs/design/lat-tools.md)) |
| NPC editor and gani editor | window pages | planned ([design](docs/design/lat-tools.md)) |
| World and config checker | `seihai-servertools check WORLD` | parse, links, images and sounds |
| The window | `seihai-servertools gui` | pages laid out |

The level editor here is for staff editing a live server. The Seihai
client keeps its own offline level editor.

## Start a lab world

A lab world is an unpacked `seihai-gserver-v2` release: `gs2emu` and a
`servers/` folder with `default` (port 14802) and `classic` (port 14803), both
on `127.0.0.1`. Put `seihai-listserver` and `seihai-servertools` in the same
folder (or on `PATH`), then:

```
seihai-servertools lab start --world path/to/world     # listserver, then default and classic
seihai-servertools lab start --world path/to/world classic
seihai-servertools lab plan  --world path/to/world     # print what start would run
```

Or copy `scripts/lab/start.sh` (Linux, macOS), `start.command` (macOS
Finder) or `start.bat` (Windows) into the world folder and run it.

The listserver starts first, on `127.0.0.1` (14900 for gservers, 14921 to
14923 for clients; `seihai-listserver.toml` in the world folder is used if
present). Each console goes to `servers/<name>/logs/console.txt` and
`logs/listserver-console.txt`. Ctrl+C stops everything. A server that stops
right away is reported with the end of its console. Without a listserver,
the gservers start alone (`--no-listserver` does that on purpose).

## Take a server public (and back)

```
seihai-servertools setup status --world path/to/world default
seihai-servertools setup public --world path/to/world default     # asks the wizard's questions
seihai-servertools setup lab    --world path/to/world default     # Back to lab
seihai-servertools setup undo   --world path/to/world default     # restore the last backup
```

`setup public` walks through the safety check, server details, network,
listserver (an address you type; there is no built-in list) and router
steps, shows a before/after of the changed lines, refuses unsafe
combinations, and saves only when you say yes. Every answer can also be
given as an option for scripts (`setup --help`), with `--yes` to skip the
questions. `serveroptions.txt` is edited line by line (comments and other
keys stay) and a dated backup is kept next to it.

### Host a listserver

```
seihai-servertools setup host-listserver --world path/to/world      # asks the wizard's questions
GRAAL_PW=... seihai-servertools setup host-listserver --world path/to/world \
    --yes --bind 0.0.0.0 --first-account owner --password-env GRAAL_PW
seihai-servertools setup listserver-lab --world path/to/world       # back to 127.0.0.1
```

It edits `seihai-listserver.toml` in the world folder (comments and other
keys stay, dated backup), refuses open mode on outside interfaces, and
lists the ports to forward. `seihai-listserver` has no gserver allow-list
yet, so keep its gserver port closed to the outside.

## Check a world

```
seihai-servertools check path/to/world
```

Reads every level, gmap and gani with the Seihai client's own parsers and
reports files that do not parse, links to levels that are not in the world,
gmap cells naming missing levels, and images or sounds that a level NPC or
a gani names but the world folder does not have. The exit code is 1 when
there are errors (a missing image or sound is a warning), so it can run in CI.

## Build

```
cargo build --release
cargo test
cargo build --release --no-default-features   # command line only, no window
```

On Linux the window needs the usual X11 or Wayland libraries at run time;
the command line needs nothing.

Releases for Linux, Windows and macOS are built by GitHub Actions on each `v*`
tag (none yet).

## Licence

`MIT OR Apache-2.0`, at your option (see `LICENSE-MIT` and `LICENSE-APACHE`).
This repo contains no code from GServer-v2, gs2lib or gs2-parser (GPL) and must not.
