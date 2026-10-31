# Go-public setup: design

Status: being built, one PR per step (see "Build order" below and
`docs/NEXT.md`). Written 2026-10-31 from `bin/docs/lab-world.md` in
`seihai-gserver-v2` PR #2; copied here from the project handoff
(`handoff/cursor/10-go-public-gui.md`) when this repo was scaffolded.

Squirrel's answers (2026-10-31) to the open questions at the end:

- **GUI toolkit:** egui/eframe 0.29, the toolkit of the Seihai client and its
  level editor; the LAT editors use it too.
- **Command line:** yes, `seihai-servertools setup ...` with the same steps as
  prompts, for servers with no screen.
- **Host a listserver:** in the first version, written against
  `seihai-listserver`'s existing config (`bind`, ports, `accounts`,
  `gserver_ip`, `site_url`, `upgrade_url`). The listserver has no "who may
  register" allow-list yet: until it does, the wizard says so and tells the
  owner to keep the gserver port closed to the outside (the listserver's own
  README, "Hosting for other machines").

## Goal

A server owner unpacks the lab world, it runs on loopback, and then they want
real players. Today that means hand-editing `serveroptions.txt` and knowing
which four or five settings matter and in what combination. The GUI walks them
through it, refuses unsafe combinations, and can put everything back to the lab
defaults with one button.

It covers two jobs:

1. **Join a listserver**: open a gserver to other machines and register it with
   a listserver the owner names.
2. **Host a listserver**: run your own listserver for other people's gservers and
   clients. It then has to listen on outside interfaces and ports too.

Plus **Back to lab**: restore the loopback defaults.

## Rules it must follow

- **No real hostnames or IPs ship anywhere**: not in the GUI, its defaults, its
  tests or its docs (`00-rules.md`). The listserver address field starts empty,
  with the placeholder `<listserver-host>`. There is no "pick a known listserver"
  list. The GUI may remember the last address the owner typed, in the owner's
  local settings only.
- Tests run only against the lab listserver on `127.0.0.1`.
- No telemetry and no outbound calls the owner did not ask for. A "test
  connection" button connects only to the address the owner typed.
- The GUI never needs admin rights. UPnP and firewall changes are opt-in and
  explained.

## Where it lives

Recommended: a "Server setup" page in **`seihai-servertools`** (MIT OR
Apache-2.0, this repo). It only reads and writes config keys and starts the
server binary, so it copies no GPL code. Config key names and file format are
facts, not code. The same wizard later writes the Rust `seihai-gserver` and
`seihai-listserver` TOML configs; the fork's `serveroptions.txt` is the first
backend.

## The settings it changes

Gserver (`servers/<name>/config/serveroptions.txt`):

| Setting | Lab value | Public value | Wizard step |
|---|---|---|---|
| `serverinterface` | `127.0.0.1` | `AUTO` (every interface) or one chosen interface | Network |
| `serverip` | `127.0.0.1` | `AUTO`, or the owner's public address | Network |
| `localip` | `127.0.0.1` | `AUTO` | Network |
| `serverport` | `14802` / `14803` | unchanged unless the owner edits it | Network |
| `localhost` | `true` | `false` (logins checked by the listserver) | automatic |
| `listip` / `listport` | `127.0.0.1` / `14900` | what the owner types | Listserver |
| `upnp` | `false` | owner's choice, default off | Router |
| `name`, `description`, `url` | `My Server`, empty url | owner's text | Server details |
| `staff` | `(Manager),YOURACCOUNT` | the owner's real account | Safety check |

Listserver (Rust `seihai-listserver` config, `seihai-listserver.toml`):

| Setting | Lab value | Public value |
|---|---|---|
| bind address | `127.0.0.1` | `0.0.0.0` / `::` or a chosen interface |
| gserver port | `14900` | same, open to outside |
| client ports | `14921`, `14922`, `14923` | same, open to outside |
| accounts | open mode | accounts file required, hashed passwords |
| address handed out for a gserver | `127.0.0.1` | the gserver's real address (see below) |
| site / upgrade URLs | empty | owner's text or empty |
| who may register | anyone on loopback | allow-list of gserver names + passwords |

## Wizard: Join a listserver

0. **Pick a server folder** (`default`, `classic`, or another under `servers/`).
   Show its current state: "Lab (loopback only)" or "Public".
1. **Safety check**. Block or warn before anything changes:
   - `staff` still contains `YOURACCOUNT`: block, ask for the owner's account.
   - Server name still `My Server`: warn.
   - Accounts left over from lab testing in `accounts/`: list them, offer to
     remove them.
   - The default account and start level: show them so the owner knows what new
     players get.
2. **Server details**: name, description, url (optional), max players.
3. **Network**:
   - "Listen on": *Every interface (AUTO)*, or a list of this machine's
     interfaces. Never offers to keep `127.0.0.1` in this wizard.
   - "Address players use": *Let the listserver decide (AUTO)* or a typed
     address. `localip` goes to `AUTO`.
   - Port, with a note that it must be free and reachable from outside.
4. **Listserver**: host and port fields, both empty, placeholder
   `<listserver-host>` and `14900`. Optional "Test connection" does a TCP connect
   to that address only and reports open / refused / timed out. `localhost` is
   set to `false` here automatically, with one line why: "Logins are now checked
   by the listserver. Leaving this on would let anyone log in as anyone."
5. **Router**:
   - "Open the port on my router automatically (UPnP)": off by default.
   - Otherwise show what to forward by hand: protocol TCP, the server port, this
     machine's LAN address. Show the OS firewall step for Windows, macOS and
     Linux as text; do not run it.
6. **Review**: a before/after of the changed lines. "Save" writes them (see
   "Writing the file").
7. **Start and check**: start the server (same as `start.sh <name>` /
   `start.bat <name>`), tail its log and show three lights:
   - Listening on: the real interface, not `127.0.0.1`.
   - Listserver: connected and registered, or the error from the log.
   - Reachable from outside: the GUI cannot prove this from behind NAT. Say so,
     and suggest asking a friend to connect.

## Wizard: Host a listserver

Squirrel's point: the listserver itself must listen on outside interfaces and
ports, not just the gservers.

1. **Safety check**: an accounts file is required (no open mode once public).
   Offer to create the first account; passwords are hashed.
2. **Network**: bind to every interface or a chosen one. Show all four ports
   (gserver 14900, clients 14921 to 14923) and let the owner change them.
3. **Who may register**: allow-list of gserver names and their passwords
   (`SERVERHQPASS`). Default: only gservers on this machine.
4. **Addresses handed to clients**: the gserver's real address as the listserver
   sees it, or the address the gserver sends (`serverip`) when it is not `AUTO`.
   Never `127.0.0.1` for a gserver that connected from another machine.
5. **Site and upgrade URLs**: empty by default, owner may type them.
6. **Router**: UPnP off by default; otherwise list the four TCP ports to forward.
7. **Review, save, start and check**: as above, with lights for "listening on
   outside interface" and "gservers registered: N".
8. Offer to point this machine's own gservers at it (`listip = 127.0.0.1` works
   when they share a machine), then run "Join a listserver" for them.

The Python lab listserver (`docs/reference/listserver.py` in `seihai-listserver`) is **not** a target for
this wizard. It is lab-only: it always hands out `127.0.0.1` for every gserver,
keeps plain passwords, and still has a hardcoded site URL. Only the Rust
`seihai-listserver` gets a public mode, and its port plan should add the
"address handed out" and "who may register" settings above.

## Back to lab

One button per server folder: restore every setting in the tables to its lab
value, `localhost = true` last (only after `serverinterface = 127.0.0.1` is
written). Same for the listserver: bind `127.0.0.1`.

## Combinations the GUI refuses to save

- `localhost = true` with `serverinterface` not `127.0.0.1`.
- `localhost = false` with an empty `listip`.
- A public listserver in open-accounts mode.
- `upnp = true` while `serverinterface = 127.0.0.1` (pointless, and confusing).

The fork's `gs2emu` could also print a loud warning at startup for the first
case. That is a small separate fork change; note it in `CHANGES-SEIHAI.md` if
done.

## Writing the file

- Edit `serveroptions.txt` line by line: keep comments, order and unknown keys.
  Never rewrite the whole file from a template.
- Keep a dated backup next to it (`serveroptions.txt.bak-YYYYMMDD-HHMMSS`) and
  offer "Undo last change".
- Add a comment line above each changed key: `# Changed by Seihai server setup, YYYY-MM-DD`.

## Tests

- Unit: round-trip `serveroptions.txt` (comments and unknown keys survive),
  every refused combination, Back to lab restores the exact lab values.
- End to end, loopback only: lab listserver on `127.0.0.1`, run the wizard with
  `127.0.0.1` typed as the listserver and a second loopback address (for
  example `127.0.0.2` on Linux) as the "outside" interface, check the gserver
  registers and a Seihai client gets it in the list. Never against a public
  listserver.

## Architecture and module layout

As built in this repo (`docs/ARCHITECTURE.md` has the full map):

```
crates/
  setup-core/              # no UI; all logic and tests live here
    src/config/options_txt.rs     line-preserving reader/writer for serveroptions.txt
    src/config/listserver_toml.rs seihai-listserver.toml (lab scan today; editor in step 6)
    src/config/backup.rs          dated backups + undo
    src/profile.rs                Profile { Lab, Public, HostListserver }; the key tables above as data
    src/checks.rs                 safety check + refused combinations -> Vec<Finding { severity, key, text }>
    src/net.rs                    interfaces, TCP "test connection" to a typed address only, UPnP opt-in
    src/run.rs                    start processes, console to files, stop together (shared with `lab`)
    src/lab.rs                    one-step lab start (listserver + gservers)
  seihai-servertools/      # the command line; `setup` is the design's "setup-cli"
  app/                     # the window; the setup wizard is one page that calls setup-core
```

The design called the command line `setup-cli`; it is the `setup`
subcommand of the one `seihai-servertools` binary, so owners download one
program.

Build order, one PR each:

1. `setup-core::config::options_txt` writer + `backup`, with round-trip tests
   (no UI). Works against the fork today.
2. `profile` + `checks` + Back to lab, with tests for every refused combination.
3. `seihai-servertools setup` on the command line (headless servers get it first).
4. `run` lights (start + log), loopback e2e test with the lab listserver.
5. Window: the Join a listserver wizard page.
6. `listserver_toml` editor and the Host a listserver wizard (command line and
   window), on `seihai-listserver`'s existing settings.

Licence: MIT OR Apache-2.0. Read the fork's config comments for key meanings,
copy none of its code.
