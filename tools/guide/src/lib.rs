//! `seihai-guide` (also `seihai-servertools guide`, and what
//! `seihai-servertools` prints with no words): how a Seihai server is put
//! together, every tool and when to use it, how to set one up from scratch,
//! the RC, and what to do when something does not work. Plain text only;
//! the rules and checks it talks about live in setup-core.

use std::process::ExitCode;

/// Help for `seihai-guide`.
pub const USAGE: &str = "\
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
  all              every topic, one after the other";

/// The topics in order, with their text.
pub const TOPICS: &[(&str, &str)] = &[
    ("overview", OVERVIEW),
    ("tools", TOOLS),
    ("setup", SETUP),
    ("listserver", LISTSERVER),
    ("gserver", GSERVER),
    ("npcserver", NPCSERVER),
    ("rc", RC),
    ("troubleshooting", TROUBLE),
];

/// The text of one topic, or `None` for no such topic.
pub fn topic(name: &str) -> Option<&'static str> {
    TOPICS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, t)| *t)
}

/// Runs one command line: the words after `seihai-guide`.
pub fn run(args: &[&str]) -> ExitCode {
    match args {
        [] => {
            println!("{OVERVIEW}");
            ExitCode::SUCCESS
        }
        ["--help" | "-h"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["all"] => {
            let all: Vec<&str> = TOPICS.iter().map(|(_, t)| *t).collect();
            println!("{}", all.join("\n\n"));
            ExitCode::SUCCESS
        }
        [name] => match topic(name) {
            Some(t) => {
                println!("{t}");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("guide: no topic `{name}`\n\n{USAGE}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

pub const OVERVIEW: &str = "\
HOW A SEIHAI SERVER FITS TOGETHER

A Graal-style server is a few programs that talk to each other:

  players' clients ──list──> LISTSERVER <──registers── GSERVER <──link── NPC-SERVER
         │                   (server list,              (the game     (server-side
         └─────────play─────────────────────────────>    world)        scripts)
  staff's seihai-rc ──RC login──> GSERVER  (and the listserver's RC list)

  listserver   seihai-listserver. Keeps the server list. Clients ask it which
               servers are up (client ports 14921-14923); gservers register on
               its gserver port (14900). It can also check account passwords.
  gserver      seihai-gserver. The game world: levels, players, accounts. Clients
               play on its port (14802). It registers with one listserver,
               named by `listserver = HOST:PORT` in its config/server.txt.
  npcserver    seihai-npcserver. Runs the server-side NPC scripts of one gserver,
               over a private link on 127.0.0.1 (14810). One per gserver that
               needs scripts; the gserver can start it for you.
  RC           seihai-rc, the remote control. Staff log in to a gserver's own
               port to see players, edit accounts and files, change options,
               and to NPC Control.

One listserver can list many gservers; each gserver lists on at most one.
A gserver works without a listserver too: players then connect to it directly.

There are two gservers in the Seihai suite:
  seihai-gserver     the Rust server (MIT/Apache). Its NPC-server is the separate
                     seihai-npcserver. `setup new` builds a folder for this one.
  seihai-gserver-v2  the GPL fork of GServer-v2 (gs2emu), with its NPC-server
                     built in. `lab start` and `setup public` work on its folders.

Where to start:
  seihai-servertools setup new           a new server from scratch, step by step
  seihai-servertools guide tools         every tool and when to use it
  seihai-servertools guide rc            remote control
  seihai-servertools guide troubleshooting
  seihai-servertools --help              every command and its options
  seihai-servertools guide all           all of the guide";

pub const TOOLS: &str = "\
EVERY TOOL, AND WHEN TO USE IT

Each is `seihai-servertools TOOL ...` and also its own program `seihai-TOOL`.

Running servers
  guide        this guide.
  setup        set up and change servers. `setup new` writes a new server folder
               from scratch (listservers, gservers, NPC-servers, staff, start
               scripts). `setup public|lab|undo|status` takes a seihai-gserver-v2
               folder public and back; `setup host-listserver` opens a listserver
               to other machines; `setup capacity`, `setup hq-password`.
               Use it first, and whenever a server changes who may reach it.
  lab          one-step lab start for a seihai-gserver-v2 world: the listserver
               and every gserver, stopped together with Ctrl+C. Use it to try
               a world on this machine.
  check        check a world folder before you start it: levels, links, images,
               sounds, NPC scripts, packages, and what a given client build can
               read (--client). Use it before going public and in CI (--json).
  stats        leaderboard web pages from a seihai-gserver world's accounts.
  gui          the window: lab start, setup, and the LAT editors for levels,
               ganis, NPCs and packages on a live server through the RC.
  suite-stress (its own program, seihai-suite-stress) starts the whole Rust suite
               on 127.0.0.1 and drives bot clients at it; for load tests.

Building a world
  level        convert levels (.graal, .zelda, .nw, Tiled .tmx), paint one to
               PNG, scatter theme objects with LevelFiller.
  gmap         make a gmap from levels, paint a whole gmap.
  gen          generate worlds: height-field terrain (2D or 3D), from a picture,
               dungeons.
  theme        LFThemeMaker: patch themes for `level fill`.
  gani         GraalShop movie ganis: list actors, paint a frame.
  gif2mng      GIF art as PNG or MNG for clients that want those.
  pw3d         a 2D playerworld as 3D worlds for seihai-gserver: reads what
               the tiles show (--objects) and builds hills, trees and houses.
  gpak         resource packages (.gpak, encrypted .gpake) for 4.20 and 6.x.
  code         the client's .code caches: show, CRC, dump, pack.

Other programs of the suite, from their own releases:
  seihai-gserver, seihai-listserver, seihai-npcserver   the servers
  seihai-rc                                             the remote control
  seihai (seihai-mae)                                   the Seihai client";

pub const SETUP: &str = "\
A SERVER FROM SCRATCH: setup new

1. Get the programs. Download seihai-gserver, seihai-listserver,
   seihai-npcserver and seihai-rc from their Releases pages (or build them),
   and put them in one folder, for example ~/seihai/bin.

2. Run the wizard:

     seihai-servertools setup new myserver --bin-dir ~/seihai/bin

   It asks:
   - lab or public. Lab listens on 127.0.0.1, so only this machine can join.
     Public listens on every address and asks for the address players use
     (your host name or public IP); the listserver then lists the gservers
     there (gserver_ip = \"sent\", listserver_ip on each gserver).
   - how many listservers to run here. Usually one. 0 uses a listserver
     someone else runs (give its gserver port, HOST:PORT) or none at all.
   - how many gservers. They take the listservers in turn.
   - the staff accounts for RC, and where their RC may log in from.
   - whether to keep the default names and ports. Answer no to name each
     server, set its ports, pick its listserver (a name from this layout,
     HOST:PORT for one elsewhere, or none) and its NPC-server mode:
       auto      the gserver starts seihai-npcserver itself (simplest)
       external  start.sh starts it, sharing a random token file
       off       no server-side scripts
   Then it shows the layout, checks names, links and every port (a port used
   twice is refused; one already busy on this machine is a warning), and
   writes the folder when you say yes.

3. What you get:

     myserver/
       README.txt            ports, addresses, start, RC commands
       layout.toml           the plan (no passwords); `--plan` makes another
       listservers/NAME/     seihai-listserver.toml, accounts.txt if any
       gservers/NAME/        a world: config/server.txt, accounts/, levels/,
                             files/, npcs/
       start.sh, stop.sh     Linux and macOS (logs/ and run/)
       start.bat, stop.bat   Windows

   Each staff account is made on its gservers with STAFF 1, its IP range and
   folder rights, and added to listservers that have an accounts file. Its
   password is random and printed once; it is not saved anywhere.

4. Start it: `cd myserver && ./start.sh` (or start.bat). Each gserver's log
   says `listserver link to ...: registered` and, with an NPC-server,
   `NPC-server connected`. Put your levels in gservers/NAME/levels/ (the
   start level is onlinestartlocal.nw; without it a plain board is made).

Without questions (scripts, CI):

  setup new DIR --yes [--listservers N] [--gservers N] [--npcserver MODE]
                [--public HOST] [--remote-listserver HOST:PORT]
                [--staff NAME]... [--rc-from RANGE]... [--bin-dir DIR]...
  setup new DIR --yes --plan layout.toml
  setup new --print-plan [options]       print the layout.toml, write nothing

layout.toml keys: bind, public_host; [[listserver]] name, title,
gserver_port, client_ports, accounts (\"open\" or \"file\"); [[gserver]] name,
title, description, port, listserver (a name, HOST:PORT or \"none\"),
listserver_accounts, npcserver (auto, external, off), npcserver_port,
create_accounts; [[staff]] account, gservers (empty: all), ip_ranges, folders.
Keys left out take the defaults.

For a seihai-gserver-v2 (fork) folder instead: unpack its release, then
`seihai-servertools lab start` to try it and `setup public` to go public.";

pub const LISTSERVER: &str = "\
THE LISTSERVER (seihai-listserver)

Settings: seihai-listserver.toml (`seihai-listserver sample-config` prints every
key with its default). It is read again when it changes; bind and ports need
a restart.

  bind             127.0.0.1 for a lab, 0.0.0.0 to serve other machines
  gserver_port     where gservers register (14900). Open it only to your own
                   gservers: anything that reaches it can register a server
  client_ports     where clients and RCs get the list (14921, 14922, 14923)
  accounts         \"\": open mode, every account and password gets in (a lab).
                   A file: only its accounts; add them with
                   seihai-listserver add-account FILE NAME --password-env VAR
  gserver_ip       \"peer\": list a gserver at the address its link comes from.
                   \"sent\": at the listserver_ip the gserver sends. Use \"sent\"
                   when gservers run on the listserver's machine but players
                   are elsewhere (else the list shows 127.0.0.1)
  gservers         an allow-list with HQ passwords, for gservers that send
                   one (the fork, `setup host-listserver --allow-gserver`).
                   seihai-gserver sends none, so leave it empty for it

Pointing a gserver at a listserver: in the gserver's config/server.txt,
`listserver = HOST:GSERVER_PORT` (seihai-gserver) or listip/listport in
serveroptions.txt (the fork; `setup public --listserver HOST:PORT`).
The link from seihai-gserver is encrypted; the gserver pins the
listserver's key in config/listserver.pin on first contact.";

pub const GSERVER: &str = "\
THE GSERVER (seihai-gserver)

The world folder holds everything: config/server.txt (one `key = value` a line;
seihai-gserver's sample world lists every key), config/allowedversions.txt
(which client builds may log in), accounts/, levels/, files/, npcs/.

  name, description   how the server shows in the list
  bind, port          where players and RCs connect (127.0.0.1:14802 in a lab)
  create_accounts     make an account on its first login: a lab only
  listserver          HOST:PORT of the listserver's gserver port; empty: unlisted
  listserver_accounts check passwords with the listserver (needs its accounts
                      file; open mode would let any password in)
  listserver_ip       the address the list shows (with gserver_ip = \"sent\")
  npcserver           off, auto or external (`guide npcserver`)
  max_players, start_level, anticheat, ...   see the sample server.txt

Accounts: `seihai-gserver account NAME --password-env VAR --world DIR` makes one
or sets its password (the password comes from the environment variable, never
the command line). An account file is accounts/NAME.txt.

Staff: an account may log in with an RC when its file has
  STAFF 1
  IPRANGE 127.0.0.1,10.0.0.0/8      where its RC may connect from (no line:
                                     loopback only; * is anywhere)
  FOLDER rw levels/*                 file-browser rights, one folder a line:
  FOLDER r config/*                  r or rw, then a folder and file mask
`setup new` writes these for the staff accounts it makes. Bans:
`seihai-gserver ban NAME [--for 3d] [--reason TEXT]`, `seihai-gserver unban NAME`.";

pub const NPCSERVER: &str = "\
THE NPC-SERVER (seihai-npcserver)

It runs the server-side NPC scripts of one gserver (from the world's npcs/
folder) over a private link on 127.0.0.1. Each gserver that needs scripts has
its own; in config/server.txt:

  npcserver = auto       the gserver starts seihai-npcserver from next to its
                         own program (or npcserver_path) with a fresh token,
                         and restarts it if it stops. Nothing else to set up.
  npcserver = external   you start it. The gserver waits on npcserver_bind and
                         npcserver_port with npcserver_token; start it with
                         seihai-npcserver --gserver 127.0.0.1:PORT
                                          --token-file FILE --world DIR
                         where FILE holds the same token (16 to 256 bytes).
                         `setup new` writes a random token to
                         config/npcserver-token.txt and start.sh does this.
  npcserver = off        no NPC-server (the default).

Keep npcserver_bind on 127.0.0.1 and the token private: whoever has it can
log in as the NPC-server. The gserver's log says `NPC-server connected` when
it is in; the NPC-server's log says `authenticated: server ...`.

Staff reach the scripts through NPC Control: `seihai-rc nc ...` (`guide rc`).
The fork (seihai-gserver-v2) has its NPC-server built in; ns_ip in its
config/adminconfig.txt is the address it gives RCs for NPC Control.";

pub const RC: &str = "\
REMOTE CONTROL (seihai-rc)

Who may log in: an account that is staff on the gserver (STAFF 1 in its file
for seihai-gserver; listed in `staff =` in serveroptions.txt for the fork),
connecting from an address in its IPRANGE (none: loopback only), with its
password. `setup new` makes such accounts and prints their passwords once.

The password goes in an environment variable, never on the command line:

  export SEIHAI_RC_PASSWORD=...           (Windows: set SEIHAI_RC_PASSWORD=...)

Log in and look around:

  seihai-rc login --server 127.0.0.1:14802 --account admin --for 30
                                     RC chat and who joins and leaves
  seihai-rc players list --server 127.0.0.1:14802 --account admin
  seihai-rc servers --listserver 127.0.0.1:14921 --account admin
                                     the listserver's RC list
  seihai-rc players list --listserver 127.0.0.1:14921 --pick \"My Server\" --account admin

Common tasks (each takes --server HOST:PORT --account NAME):

  seihai-rc players say \"Restart in 5 minutes\"      admin message to everyone
  seihai-rc players pm bob \"Meet at the shop\"
  seihai-rc players kick bob
  seihai-rc players warp alice cave.nw 30.5 12
  seihai-rc players ban test1 --set on --reason \"Cheating\"
  seihai-rc players props test1 --set gralats=100
  seihai-rc accounts list \"%\"                  every account (% is a wildcard)
  seihai-rc accounts set test1 --email a@example.invalid
  seihai-rc files ls levels/                   the file browser, within FOLDER rights
  seihai-rc files get levels/onlinestartlocal.nw --out start.nw
  seihai-rc files put start.nw levels/onlinestartlocal.nw
  seihai-rc server options                     config/server.txt; --set-file to change
  seihai-rc server flags

NPC Control (with an NPC-server running). seihai-gserver takes NC logins on
its own port:

  seihai-rc nc npcs --server 127.0.0.1:14802 --account admin
  seihai-rc nc npc get 12 --server 127.0.0.1:14802 --account admin
  seihai-rc nc weapons --server 127.0.0.1:14802 --account admin

Rights:
  STAFF 1            may log in with an RC at all
  IPRANGE            where from: addresses, ADDR/BITS ranges, or *
  FOLDER r|rw MASK   what the file browser may read or write, per folder
  RCRIGHTS N         the RC's rights bits (`players rights NAME --set N`); kept
                     and shown, but seihai-gserver does not limit RC actions
                     by them yet: every staff RC may do every RC action
On the fork the rights bits do apply; set them from an RC's account dialog.

The window: `seihai-rc gui` (built with --features gui), and the LAT editors in
`seihai-servertools gui` (levels, ganis, NPCs, packages over the RC file link).
`seihai-rc --help` lists every command.";

pub const TROUBLE: &str = "\
TROUBLESHOOTING

First look at the end of each log (logs/ in a `setup new` folder) and run
`seihai-servertools check gservers/NAME` on a world.

A server will not start
  - \"address in use\": another program has the port. `setup new` warns about
    busy ports; stop the old copy (./stop.sh) or pick another port.
  - Ports below 1024 need admin rights; use the 14xxx ports.
  - start.sh says a program is not found: put the server programs on PATH,
    or run with SEIHAI_BIN_DIR=/path/to/programs ./start.sh.

The gserver is not on the list
  - Its log should say `listserver link to HOST:PORT: registered`. If it says
    `retrying`, the listserver is not up or `listserver =` names the wrong
    port: it is the listserver's gserver_port (14900), not a client port.
  - The listserver has a `gservers` allow-list: seihai-gserver sends no HQ
    password, so it is never listed. Empty the list for it.
  - `listserver_secure`: seihai-listserver needs the encrypted link (the
    default). A GServer-v2 style listserver needs listserver_secure = false.
  - A changed listserver key: delete the gserver's config/listserver.pin only
    if you know why the key changed.
  - Players elsewhere see 127.0.0.1 in the list: set gserver_ip = \"sent\" on
    the listserver and listserver_ip = <public-host> on the gserver.

The NPC-server does not connect
  - The gserver's log should say `NPC-server connected`.
  - auto: `npcserver: cannot start ...` means seihai-npcserver is not next to
    seihai-gserver; put it there or set npcserver_path.
  - external: `refused: bad token` means the token file and npcserver_token
    differ; `already connected` means two NPC-servers point at one gserver.
    Check that --gserver names that gserver's npcserver_port.

The RC cannot log in
  - \"Account name or password is wrong\": wrong password or account; reset
    with seihai-gserver account NAME --password-env VAR --world DIR.
  - Refused as not staff: STAFF 1 missing from accounts/NAME.txt.
  - Refused from your address: add it to IPRANGE (loopback is the default).
  - With listserver_accounts = true the password is checked by the listserver:
    the account must be in its accounts file.
  - `seihai-rc nc discover` gets no answer from seihai-gserver yet; connect NC
    to the gserver's own port instead (`nc npcs --server HOST:PORT`).

Players cannot connect from other machines
  - bind is 127.0.0.1 (a lab). Use `setup new --public HOST`, or set bind
    = 0.0.0.0 in each config, then open the gserver and client ports in the
    firewall and forward them on the router. Keep the NPC-server link and
    (where you can) the listserver's gserver_port closed to the outside.
  - A client build refused at login: config/allowedversions.txt.
  - `setup test-connection HOST:PORT` makes one TCP connection to check a port.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_is_in_the_usage() {
        for (name, text) in TOPICS {
            assert!(USAGE.contains(&format!("  {name} ")), "{name}");
            assert!(!text.is_empty());
        }
        assert!(topic("RC").is_some());
        assert!(topic("nope").is_none());
    }

    #[test]
    fn every_tool_is_in_the_tools_topic() {
        let tools = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for e in std::fs::read_dir(tools).unwrap() {
            let e = e.unwrap();
            if e.path().join("Cargo.toml").is_file() {
                let name = e.file_name().into_string().unwrap();
                assert!(
                    TOOLS.contains(&format!("\n  {name} ")),
                    "tools/{name} is not in `guide tools`"
                );
            }
        }
    }

    #[test]
    fn no_real_hosts() {
        // AGENTS.md: placeholders and the lab ports only.
        for (_, t) in TOPICS {
            for w in t.split(|c: char| c.is_whitespace() || c == '(' || c == ')') {
                if let Some((host, _)) = w.split_once(':') {
                    if host.chars().all(|c| c.is_ascii_digit() || c == '.')
                        && host.chars().any(|c| c.is_ascii_digit())
                    {
                        assert!(host.starts_with("127.") || host == "0.0.0.0", "{w}");
                    }
                }
            }
        }
    }
}
