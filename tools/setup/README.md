# seihai-setup

Server setup on the command line: a new server folder from scratch (`setup new`), and the go-public wizard: take a gserver folder from the lab to the public and back, host a listserver, set the player limit and the HQ password. Every rule and write goes through setup-core.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. `seihai-servertools setup ...` runs the same code with the
same words, for those who want every tool in one program.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-setup    # target/release/seihai-setup
cargo test -p seihai-setup
target/release/seihai-setup status --world path/to/world default
```

Only this tool and the crates it uses are built. The binary needs nothing
else from this repository at run time.

## A server from scratch: `setup new`

```text
setup new [DIR] [--plan FILE] [--listservers N] [--gservers N]
          [--npcserver auto|external|off] [--public HOST]
          [--remote-listserver HOST:PORT] [--staff NAME]... [--rc-from RANGE]...
          [--bin-dir DIR]... [--no-port-check] [--print-plan] [--yes]

Writes a new server folder DIR (default seihai-server; it must not exist or
be empty) for the Seihai Rust stack: listservers, gservers each pointed at
a listserver, an NPC-server for each gserver that wants one, staff accounts
for RC with random passwords (printed once, saved nowhere), and start and
stop scripts (start.sh / stop.sh, start.bat / stop.bat).

Without --yes it asks for each choice, with the options as the defaults.
With --yes it asks nothing: the layout comes from --plan FILE (a
layout.toml, as every folder made gets one) or from the options:

  --listservers N      listservers run here (default 1; 0: none, or --remote-listserver)
  --gservers N         gservers (default 1), all on the first listserver
  --npcserver MODE     auto: each gserver starts its NPC-server (default);
                       external: start.sh starts it, with a token file; off
  --public HOST        listen on every address and list the gservers at HOST
                       (default: a lab on 127.0.0.1, this machine only)
  --remote-listserver HOST:PORT   register with a listserver run elsewhere
  --staff NAME         a staff account for RC on every gserver (default admin;
                       repeat for more; --staff none for no account)
  --rc-from RANGE      where staff RCs may log in from (default 127.0.0.1;
                       repeatable; an address, ADDR/BITS or *)
  --bin-dir DIR        where seihai-gserver, -listserver, -npcserver are
                       (repeatable; then next to this program, then PATH).
                       Without seihai-gserver the accounts are not made and
                       the commands to make them are printed instead
  --no-port-check      do not check whether the ports are free on this machine
  --print-plan         print the layout.toml and write nothing

`seihai-servertools guide setup` explains every choice.
```

The tests in `tests/new.rs` write a full layout (two listservers, three
gservers with each NPC-server mode, two staff accounts) into a temporary
folder. With `SEIHAI_SUITE_BIN_DIR` naming a folder with seihai-gserver,
seihai-listserver, seihai-npcserver and seihai-rc, `boots_on_loopback` also
runs its `start.sh` on 127.0.0.1 and checks that every gserver registers with
its listserver, the NPC-servers connect, the staff accounts log in with
seihai-rc (and only where they are staff) and `stop.sh` stops it all:

```sh
SEIHAI_SUITE_BIN_DIR=path/to/server/programs cargo test -p seihai-setup --test new
```

## Depends on

- setup-core (its `layout` module for `setup new`), ctrlc
- seihai-core: None. setup-core does not use seihai-core, so this tool builds without it.

It does not depend on any other tool in `tools/`.

## Usage

```text
seihai-servertools setup: set up a server from scratch, or take a gserver folder
from the lab to the public and back

  setup new    [DIR] [options]   a new server folder from scratch: listservers,
               gservers pointed at them, NPC-servers, staff accounts for RC,
               start and stop scripts; asks step by step (`setup new --help`)

  setup status [--world DIR] [SERVER]
  setup public [--world DIR] [SERVER] [--listserver HOST[:PORT]] [--account NAME]
               [--name TEXT] [--description TEXT] [--url TEXT]
               [--interface AUTO|ADDR] [--server-ip AUTO|ADDR] [--upnp] [--yes]
  setup lab    [--world DIR] [SERVER] [--yes]      Back to lab
  setup undo   [--world DIR] [SERVER]              restore the last backup
  setup capacity --kind fork|rust [--world DIR] [SERVER] --max-players N [--yes]
  setup capacity --kind fork|rust [--world DIR] [SERVER] --undo [--yes]
               preview and set the gserver's player limit, or undo it
  setup hq-password [--world DIR] [SERVER] --hq-password-env VAR
               write hq_password in config/adminconfig.txt (dated backup);
               `setup public` takes --hq-password-env too
  setup start  [--world DIR] [SERVER] [--gserver PATH] [--for SECS]
               start the server and show the three lights; Ctrl+C stops it
  setup test-connection HOST[:PORT]               one TCP connect to that address

  setup host-listserver [--world DIR] [--bind ADDR] [--gserver-port N]
               [--client-ports A,B,C] [--accounts FILE] [--gserver-ip peer|sent]
               [--site-url URL] [--upgrade-url URL]
               [--max-connections N] [--silent-list-ms MS]
               [--allow-gserver ADDR]... [--hq-password-env VAR] [--any-gserver]
               [--first-account NAME --password-env VAR] [--listserver-bin PATH] [--yes]
               host a listserver for other machines (edits seihai-listserver.toml)
  setup listserver-lab  [--world DIR] [--yes]     listserver back to 127.0.0.1
  setup listserver-undo [--world DIR]             restore its last backup

SERVER defaults to `default` (servers/default). --world defaults to the
current folder. Values not given are asked for. Every save keeps a dated
backup of serveroptions.txt next to it.

Who may register (host-listserver): --allow-gserver ADDR, once per gserver
address, lets only those addresses register; each must send the HQ password
from the environment variable named by --hq-password-env (the same text as
hq_password in that gserver's config/adminconfig.txt). An address already in
the file keeps its password when no variable is given. --any-gserver empties
the list.

--silent-list-ms (default 500) lets real 1.41 rev 1, which waits without
sending anything, get the server list; 0 turns it off. --max-connections
(default 256) caps the listserver's open links, players and gservers together. With --yes and neither option, the list is left as it is.

capacity: fork edits servers/SERVER/config/serveroptions.txt (maxplayers);
rust edits config/server.txt (max_players) under --world, without a SERVER name.
The file must exist. N must be a positive whole number. Only that player's
limit setting changes; no server is started or restarted. The effective limit
and reload behavior depend on the server. This does not raise listserver
max_connections or guarantee that the machine can sustain that many players.
```
