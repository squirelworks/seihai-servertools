# seihai-setup

The go-public wizard on the command line: take a gserver folder from the lab to the public and back, host a listserver, set the player limit and the HQ password. Every rule and write goes through setup-core.

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

## Depends on

- setup-core, ctrlc
- seihai-core: None. setup-core does not use seihai-core, so this tool builds without it.

It does not depend on any other tool in `tools/`.

## Usage

```text
seihai-servertools setup: take a gserver folder from the lab to the public and back

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
