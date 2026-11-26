# seihai-servertools

Tools for running and building a Graal server with the Seihai server stack:
start a lab world with one command, take a server from the lab to the public
with a setup wizard, and the LAT (Level Administration Team) toolkit for
editing a live server's world.

Part of the Seihai project: the client [seihai](https://github.com/squirelworks/seihai)
and its library [seihai-core](https://github.com/squirelworks/seihai-core); the Rust
server stack [seihai-gserver](https://github.com/squirelworks/seihai-gserver),
[seihai-listserver](https://github.com/squirelworks/seihai-listserver),
[seihai-npcserver](https://github.com/squirelworks/seihai-npcserver),
[seihai-rc](https://github.com/squirelworks/seihai-rc) and
[seihai-servertools](https://github.com/squirelworks/seihai-servertools); the GPL forks
[seihai-gserver-v2](https://github.com/squirelworks/seihai-gserver-v2) and
[seihai-gs2lib](https://github.com/squirelworks/seihai-gs2lib); and the research notes in
[Graal_analysis](https://github.com/squirelworks/Graal_analysis).

## Status

Nothing has been released yet, but most of the tools below work today: the
one-step lab start, the go-public setup wizard, the world checker, the LAT
level, gani and NPC editors (open, change, preview, and upload through the RC
file link or to a local folder), the package and level tools, and a stress rig
for the whole server suite (`seihai-suite-stress`,
[docs/lab/suite-stress.md](docs/lab/suite-stress.md)).

The plan is in [docs/PLAN.md](docs/PLAN.md), the code layout in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), the to-do list in
[docs/NEXT.md](docs/NEXT.md) and notes for server operators on security in
[docs/security/README.md](docs/security/README.md). Contributors should read
[AGENTS.md](AGENTS.md) for the project rules.

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
| Level convert: `.graal` / `.zelda` / `.nw`, either way, and Tiled `.tmx` (NW2TMX's job) | `seihai-servertools level convert` | working |
| gif2mng: GIF to PNG or MNG animation | `seihai-servertools gif2mng` | working (`-b` partial) |
| GraalShop movie mode (`GANI0002`): list actors, paint a timeline frame; movies survive a gani save | `seihai-servertools gani movie show\|render` | working (SPRITE, CHAR and TEXT actors drawn) |
| Level and gmap pictures (NW2PNG's job) | `seihai-servertools level render`, `gmap render` | working (board, chests, signs; NPC images and ganis with `--npcs`) |
| LevelFiller and LFThemeMaker: theme fills, patch themes | `seihai-servertools level fill`, `theme make` | working (borders and `--across GMAP`) |
| Gmap from levels: a list, generator names, or a linked world's edge links | `seihai-servertools gmap make` | working |
| World generators: height field (2D, 3D with `--3d`, streamed client terrain with `--stream`, or 3D from an existing gmap with `gen 3d`; `--playerworld` lays it out as a gserver folder), from a picture, dungeon | `seihai-servertools gen terrain\|3d\|image\|dungeon` | working (3D output not yet client-tested) |
| Leaderboard pages from seihai-gserver accounts (gserverstat's job) | `seihai-servertools stats` | working |
| The window | `seihai-servertools gui` | pages laid out |

Every command is also its own program, built, tested and run on its own:
`seihai-lab`, `seihai-setup`, `seihai-check`, `seihai-code`, `seihai-gpak`,
`seihai-level`, `seihai-theme`, `seihai-gmap`, `seihai-gani`, `seihai-gen`,
`seihai-stats`, `seihai-gif2mng` and `seihai-servertools-gui` take the same
words after their name (`seihai-gpak pack ...` is `seihai-servertools gpak
pack ...`). Each lives in `tools/<tool>/` with its own README; the list and
the rules for a new tool are in [tools/README.md](tools/README.md).

The level editor here is for staff editing a live server. The Seihai
client keeps its own offline level editor.

## Getting started

There are no release downloads yet, so build it first (see [Build](#build)).
Then, the first time:

1. Download a [seihai-gserver-v2 release](https://github.com/squirelworks/seihai-gserver-v2/releases)
   and unpack it. That folder is your lab world.
2. Put `seihai-servertools` and, if you want a server list,
   [seihai-listserver](https://github.com/squirelworks/seihai-listserver)
   in the same folder.
3. From that folder, run `seihai-servertools lab start` and point a client at
   `127.0.0.1:14802`. Ctrl+C stops everything.
4. `seihai-servertools gui --world .` opens the window with the editors and the
   setup pages; `seihai-servertools check .` reports anything broken in the world.

After that, normal use is `lab start` from the world folder (or the start
script, below) and `gui` when you want to edit.

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

`check --list-clients` lists shared build labels. `check --client "1.39" WORLD`
(or the World check page's profile picker) adds selected-build level, gmap, gani
and script capability warnings. The default general inventory and JSON format
are unchanged. Unknown support remains unknown; package readability and exact
per-build script vocabulary are not certified.

`check --client-reach WORLD` also warns about content that some client
versions cannot use: PNG art with no GIF copy for pre-2.1 clients, gmaps and
ganis on versions without them, GS2 scripts on versions that only run GS1,
tiles past the edge of older tile sheets, and image names built at run time.
`check --make-gif WORLD` writes the missing GIF copies first. What each check
reads and where its limits are: [docs/check-client-reach.md](docs/check-client-reach.md).

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

## Command line

`seihai-servertools --help` lists every command, and most take `--help`
for their own options (`setup --help`, `gpak --help`, `level fill --help` and so
on; `lab` is covered by the main `--help`). The options you will use most:

| Command | Options |
| --- | --- |
| `lab start [SERVER...]` / `lab plan` | `--world DIR` (default: the current folder), `--listserver PATH` or `--no-listserver`, `--gserver PATH` (the `gs2emu` to run), `--rust DIR` (repeatable) and `--rust-gserver PATH`. With no SERVER it starts `default`, `classic` and `modern`, whichever exist. |
| `setup status\|public\|lab\|undo [SERVER]` | `--world DIR`; see [Take a server public](#take-a-server-public-and-back). |
| `check WORLD` | `--json` for CI; `--client LABEL` selects a shared build (`check --list-clients` lists them); `--client-reach` warns about content older clients can't use, and `--make-gif` adds GIF copies of the PNG art it flags. Exit code 1 on errors. |
| `gui` | `--world DIR`, `--page lab\|setup\|levels\|npcs\|database\|ganis\|packages\|check` to open on a page. |
| `gpak pack FOLDER OUT` / `gpak unpack PACKAGE DIR` | `--server DIR`, `--key HEX16` or `--keys FILE` for encrypted `.gpake`. |
| `level convert IN OUT.nw`, `level render IN OUT.png`, `gmap render` | `--sheet PNG` (tile sheet), `--scale N`; `gmap render` also takes `--dir DIR`. |
| `level fill THEME.nwt LEVEL...`, `theme make RECIPE.tm OUT.nwt` | `--out DIR` or `--in-place`, `--seed N`. |
| `gmap make OUT.gmap` | `--width N LEVEL...`, `--names DIR` or `--links DIR START`. |
| `stats ACCOUNTS OUTDIR [MODE...]` | Modes: kills, deaths, rupees, highap, lowap, maxpower, power, arrows, bombs, glovepower, swordpower, shieldpower (default all). |
| `gif2mng GIF...` | `--out DIR`, `--force`, `-b` (palette index 2 transparent). |

The tool reads no environment variables of its own; programs it starts are
found next to it, in the world folder or on `PATH`.

### seihai-suite-stress

A standalone tool ([tools/suite-stress](tools/suite-stress/README.md)), not
part of `seihai-servertools`. It starts the whole server suite on `127.0.0.1` and drives bot
clients at it. `seihai-suite-stress run` runs bots for `--stay` seconds and
writes a report; `compat` tries one bot per version profile; `versions` lists
the profiles; `probe ADDR VERSION ACCOUNT STEPS` runs one scripted client
against a gserver you started (`STEP_MS` sets the wait after each step).
The main options, all with defaults, are `--bin-dir DIR` (where the server
programs are, default next to it), `--out DIR`, `--base-port N` (24800),
`--bots N` (24), `--versions LIST|all`, `--stay SECS` (60), `--ramp N` (20)
and `--no-npcserver` / `--no-listserver`; `seihai-suite-stress --help` has the
full list, including the walk, chat, warp and external-client options.
[docs/lab/suite-stress.md](docs/lab/suite-stress.md) explains the report.

## Build

```
git clone https://github.com/squirelworks/seihai-servertools
cd seihai-servertools
cargo build --release      # target/release/seihai-servertools, every seihai-<tool> and seihai-suite-stress
cargo test
cargo build --release -p seihai-servertools --no-default-features   # command line only, no window
cargo build --release -p seihai-gpak   # one tool on its own (any tool in tools/)
cargo test -p seihai-gpak
```

On Linux the window needs the usual X11 or Wayland libraries at run time;
the command line needs nothing.

Releases for Linux, Windows and macOS will be built by GitHub Actions on each
`v*` tag; there has been none yet.
When a release happens, what the version numbers mean and what 1.0.0 needs: [VERSIONING.md](VERSIONING.md).

## Licence

`MIT OR Apache-2.0`, at your option (see `LICENSE-MIT` and `LICENSE-APACHE`).
This repo contains no code from GServer-v2, gs2lib or gs2-parser (GPL) and must not.
