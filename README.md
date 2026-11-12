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

**Status: scaffold.** Working today: the one-step lab start, the setup
wizard, a world checker, and the first LAT level and gani editors (open,
change, preview, upload through the RC file link or a local folder), and
a stress rig for the whole server suite (`seihai-suite-stress`,
[docs/lab/suite-stress.md](docs/lab/suite-stress.md)). The
plan is in [docs/PLAN.md](docs/PLAN.md), the code layout in
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
| Gserver player limit (fork `maxplayers`, Rust `max_players`) | `seihai-servertools setup capacity` | command line working |
| LAT level editor: open, change a tile, preview, upload | `seihai-servertools gui --page levels` | working ([design](docs/design/lat-tools.md)) |
| Gani editor: open, change a sprite frame, preview, upload | `seihai-servertools gui --page ganis` | working ([design](docs/design/lat-tools.md)) |
| NPC editor: level NPC scripts with the GS1 check, upload | `seihai-servertools gui --page npcs` | level NPCs working; NPC-server objects planned ([design](docs/design/lat-tools.md)) |
| Resource packages: build a `.gpak` / encrypted `.gpake` for a gserver folder ([notes](docs/design/gpak.md)) | `seihai-servertools gpak pack\|unpack`, `gui --page packages` | working |
| Client `.code` caches (`GWEBL001`, `GRESCR01`) | `seihai-servertools code show\|crc\|dump\|pack` | working |
| World and config checker | `seihai-servertools check WORLD` | parse, links, images and sounds, NPC scripts, packages |
| The window | `seihai-servertools gui` | pages laid out |

The level editor here is for staff editing a live server. The Seihai
client keeps its own offline level editor.

`seihai-servertools check --client-reach [--json] WORLD` adds warnings for
level NPC images and fixed GS1 script image names that have PNG art but no
GIF counterpart for pre-2.1 clients. It inventories file names and never
converts or writes art. A matching GIF name does not certify its contents or
original-client rendering.

The same option reports parsed gani files and lists the pinned shared client
profiles whose `ui.ganis` field is false. Those profiles need server animation
fallback review if the file is used. This is an inventory, not a scan of
animation call sites. The pinned data marks 1.41 unsupported and 1.42 supported;
these are shared-profile values, not a new original-client observation. Unused
and standard ganis are included too. A GIF copy
of a sprite sheet does not give a client gani support, and no nearest built-in
animation is inferred. Symbolic sheets such as `BODY` and `HEAD` are not
treated as literal image filenames; ordinary checks still handle parse errors
and missing images/sounds.

It also lists, for each parsed gmap, the shared profiles whose
`levels.gmap` is false (they need linked single levels if players reach
it), and for each level the profiles that cannot open its format. The
format comes from the level's magic (`GLEVNW01`, `GR-V1`, `Z3-V1`), not its
extension.

Scripts: each level NPC and weapon (`weapons/*.txt` files starting
`GRAWP001`) whose script is GS2 only, or mixes GS1 and GS2, is listed with
the shared profiles before 4.x, which run only GS1. The era is read from the
text: GS2 event functions (`function onCreated()`), `temp.`, `player.`,
`params[` and `new Name(`; GS1 event flags (`if (playerenters)`) and `#`
codes outside strings. GS1-only scripts run on every client and are not
listed. seihai-core's profiles have no GS2 flag, so the cut is 4.04r2, the
first 4.x row.

Tiles: each level with tiles past column 116 of the default `pics1.png` is
listed with the profiles whose shipped sheet is 1856×512 (1.28 beta 2 to
1.41, and 2.00 to 2.05); they draw nothing in those cells. That sheet's 116
columns match the left 116 of the 2048-wide 1.42 sheet cell for cell, so
tile ids map the same way and the old sheet just ends early. 1.21 (a
1280-wide sheet whose layout is not established) and 2.1x (no sheet in the
archives) are not listed, and `tiledefs` sheets are not matched. Dynamic
names and weapon images remain outside these slices of issue #38.

## Start a lab world

A lab world is an unpacked `seihai-gserver-v2` release: `gs2emu` and a
`servers/` folder with `default` (port 14802), `classic` (port 14803) and, in
releases from 2026-11-11, `modern` (port 14805, 4.20r8 to 6.037 and `.gpak`
packages), all on `127.0.0.1`. Put `seihai-listserver` and `seihai-servertools` in the same
folder (or on `PATH`), then:

```
seihai-servertools lab start --world path/to/world     # listserver, then default, classic and modern
seihai-servertools lab start --world path/to/world classic
seihai-servertools lab plan  --world path/to/world     # print what start would run
seihai-servertools lab start --world path/to/world --rust rust   # also seihai-gserver on <world>/rust
```

`--rust DIR` starts `seihai-gserver --world DIR` too (a folder with
`config/server.txt`, relative to the world), after the listserver and next
to the fork's folders; with no fork `gs2emu` it starts alone. Its console
goes to `logs/<folder>-console.txt`. The program is looked for in the world
folder, next to `seihai-servertools` and on `PATH`, or given with
`--rust-gserver PATH`. Give it its own port: seihai-gserver's default, 14802,
is `servers/default`'s too, and the plan says so. Its `config/server.txt` is
never edited; if its `listserver` line does not point at the lab listserver
(`127.0.0.1:14900`), the plan says which line to change.

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
lists the ports to forward. "Who may register" writes the listserver's
`gservers` allow-list: only the addresses you list may register, each after
sending its HQ password (`hq_password` in that gserver's
`config/adminconfig.txt`). Give the password in an environment variable
(`--allow-gserver 127.0.0.1 --hq-password-env HQ_PW`) or at the prompt; it is
never printed. Still open the gserver port only to those addresses. Each
gserver gets the same password with
`HQ_PW=... seihai-servertools setup hq-password --hq-password-env HQ_PW`
(or `setup public --hq-password-env HQ_PW`, or the HQ password field in the
window), which writes `hq_password` in its `config/adminconfig.txt`.
It also sets `silent_list_after_ms` (default 500, so the real 1.41 rev 1,
which waits without sending anything, gets the list; `--silent-list-ms`) and
can raise `max_connections` for a big world (`--max-connections`, listserver
default 256). `lab start` notes when the listserver would leave 1.41 rev 1
without a list.

### Set a gserver player limit

Use an explicit server kind; the two gservers use different files and keys:

```sh
# Fork: servers/default/config/serveroptions.txt, maxplayers
seihai-servertools setup capacity --kind fork --world path/to/lab --max-players 1000
# Rust: config/server.txt under this world, max_players (not TOML)
seihai-servertools setup capacity --kind rust --world path/to/rust-world --max-players 1000
# Restore the latest backup of that config, including other settings in it
seihai-servertools setup capacity --kind rust --world path/to/rust-world --undo
```

The config must already exist. The command previews its file and old/new
setting and asks before saving (`--yes` confirms for scripts). It preserves
comments, unknown settings and line endings, keeps a dated backup, and refuses
to overwrite a file changed since preview. Setting the same value does nothing.
`--undo` restores the whole previous file, not only the capacity setting.
For another fork folder, add its SERVER name; Rust uses only `--world`.

This is the game-player admission limit. The listserver's separate
`max_connections` limit counts its open player and gserver links together;
raise it through `setup host-listserver --max-connections N` when needed.
The stress rig already accepts `--max-players` and `--list-max-connections`.
Increasing either setting does not establish a tested player capacity. The
command does not restart servers; follow the selected server's config reload
procedure. No window control for gserver capacity is added by this command.

## Check a world

```
seihai-servertools check path/to/world
```

Reads every level, gmap and gani with the Seihai client's own parsers and
reports files that do not parse, links to levels that are not in the world,
gmap cells naming missing levels, and images or sounds that a level NPC or
a gani names but the world folder does not have. It also runs the setup
checks on each `servers/<name>/config/serveroptions.txt` (and the `accounts/`
of public folders) and on `seihai-listserver.toml`, so a setting `setup` would
refuse to save, such as `localhost = true` off loopback, is an error. In each
gserver folder it warns about resource packages: a `.gpake` with no key in
`config/gpakkeys.txt` (the gserver will not send it), a key file under the
served `world/` or `files/`, and a plain `.gpak` when `allowedversions.txt`
may let in 4.20r8, which never loads one. The exit code is 1 when
there are errors (a missing image or sound is a warning), so it can run in CI.
`check --json WORLD` prints the same findings as one JSON object (`format` 1,
the counts, and each finding's `level`, `source`, `file`, `key` and `text`).

## Inspect client cache CRCs

```sh
seihai-servertools code show synthetic.code --key C82CEA9ED93DFB8F
seihai-servertools code crc synthetic.code
seihai-servertools code crc synthetic.code --domain plain --key C82CEA9ED93DFB8F
```

`code crc` defaults to the `stored` domain: CRC32 of the on-disk bytes after
the `GRESCR01` magic, before decryption. It needs no key. In a real Windows
6.113 session this is the CRC the server sends in packet 197, for every
script checked (see the [audit](docs/design/code-caches.md)). `--domain plain`
gives the CRC of the decrypted body and needs `--key`. `--key` is the two key
words from packet 140 or 197, each written as a little-endian 32-bit value,
first word first; the tool applies the DES key-byte bit reversal itself.
`show` labels both CRCs. Neither CRC proves the client will accept the cache,
that the body is a valid GS2 container, or that the script runs. Commands
operate on local files only.

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
