# Lab run: real-fork lab start, go-public, and 1.41 rev 1 (#47)

Date: 2026-11-12. Runbook: `docs/lab/runbooks/47-real-fork.md`, steps A to D.
Machine: a Debian 13 x86_64 guest on the lab host. Loopback only, with no
port forwarded to the host.

## Builds

| Component | Source | Commit / release |
|---|---|---|
| gserver | seihai-gserver-v2 release `v4.0.0-seihai.1` (Linux x86_64 tarball, has `servers/modern`) | release target `576c954`; `gs2emu` prints "Preagonal GS2Emu version 4.0.0-beta (2026-11-11 10:22)" |
| seihai-listserver | main | `4504aa6` (v0.1.1) |
| seihai-servertools | main | `0ca4257` |
| seihai-mae | seihai main, `crates/seihai-mae` | `f62d253`, built with `--no-default-features --features dev` (no `audio`: libasound2-dev is not installed) |
| Wine | Debian trixie `wine32:i386` 10.0, unpacked without root and overlaid on `/usr` with bwrap | win32 prefix |
| 1.41 rev 1 | the archived `graal_1411/Graal.exe` | — |

`cargo build --release`, rustc 1.99.1.

## How the run was isolated

The guest was already running another Seihai lab stack and a harness stack
on the lab ports. Both were left running. The whole #47 run used its own
network namespace instead (`bwrap --unshare-user --unshare-net`, entered with
`nsenter`). It had only `lo`, plus a dummy `lab0` for step C, and no route out.

## Result table

| Step | Expected | Seen | Pass |
|---|---|---|---|
| A listserver | 127.0.0.1:14900 | `listening on 127.0.0.1:14900 for gservers and 127.0.0.1:14921, 127.0.0.1:14922, 127.0.0.1:14923 for clients; accounts: open (every account gets in)` | PASS |
| A default/classic/modern | listening, linked | `Listening on: 127.0.0.1:14802.` / `:14803.` / `:14805.`, each with `listserver - Connected to 127.0.0.1:14900.`; `check-consoles.sh`: 10/10 PASS, exit 0 | PASS |
| A console lines match the fixtures | yes | Same wording, with a timestamp prefix (`[04:37 PM]      Listening on: …`). `lights.rs` uses `find` and strips the trailing `.`, so no fix is needed | PASS |
| B seihai-mae --list-only | 3 servers | `There are 3 server(s) online.`: Lab Modern :14805, Lab Classic :14803, My Server :14802 | PASS |
| B Ctrl+C | all stopped, exit 0 | Real Ctrl+C through a pty: `^CStopping...`, exit 0, nothing left listening | PASS |
| C setup public + start | 127.0.0.2:14802, linked | `--interface 127.0.0.2` is BLOCKED: every 127.x address is loopback. On a namespace-only dummy interface `192.0.2.2`: `[ ok ] 192.0.2.2:14802`, `[ ok ] connected to 127.0.0.1:14900`, `[ ?? ]` reachable from outside; `check-consoles.sh --addr 192.0.2.2`: all PASS | FAIL as written; PASS on 192.0.2.2 |
| C setup lab / undo / lab | back to lab values | Each `setup lab` puts back the interface, serverip, localip and localhost; `undo` restores the public save. The diff against the first backup keeps `staff` and five `# Changed by Seihai server setup` lines | PASS for values |
| D 1.41 rev 1 | gets the list | The real `Graal.exe` under Wine dials its built-in list name (mapped to 127.0.0.1 in a sandboxed hosts file) on 14923, sends nothing, and gets the list 500 ms later: My Server, Lab Classic and Lab Modern | PASS |

## Findings, and what this PR does with them

1. **Step C can't use 127.0.0.2.** `setup public` blocks every 127.x
   interface for a public server, which is right. But the message said
   "not 127.0.0.1" whatever the address was. **Fixed:** it now names the
   address. The runbook now uses a dummy interface in a network namespace.
2. **A stock release world needs two more things for `setup public`:**
   `accounts/YOURACCOUNT.txt` (password `super`) blocks it, and `--yes`
   needs `--account`. **Runbook updated.**
3. **The diff after public → lab → undo → lab isn't empty.** `staff` keeps the
   new account (`setup lab` only sets the network values), and each changed
   key has a `# Changed by Seihai server setup` line. They don't pile up on
   repeated runs. **Runbook updated** to expect that.
4. **Two green lights, not three.** "Reachable from outside" is `[ ?? ]` on a
   lab machine. **Runbook updated.**
5. **`lab start --world .` also starts `default` on the public address**, as
   the runbook warned. **Runbook updated:** `./start.sh classic` for the
   listserver, then `setup start default`, is now the main step.
6. **Step D needs an account file.** `setup host-listserver` points the
   listserver at `accounts.txt`, which doesn't exist yet. The silent list
   itself doesn't need an account, because 1.41 sends nothing. **Runbook
   updated** (`seihai-listserver add-account`).
7. **`setup listserver-lab` says "nothing to change"** after
   `host-listserver --bind 127.0.0.1`, and leaves `accounts` and
   `silent_list_after_ms` in place. **Runbook updated.**
8. **On Ctrl+C every gserver logs `listserver - Disconnected.`**, because
   the listserver stops first. `check-consoles.sh` and `lights.rs` treat that
   line as an error, so the check is only valid while the world runs.
   **Runbook updated.**
9. **Every `lab start` logs a probe:** `gserver 1 connected` then `closed
   (no REGISTERV3 line)` before the real gservers register. This is harmless.
   **Runbook updated.**
10. **`seihai-mae` lives in seihai, not seihai-rc.** **Runbook fixed.**
11. Minor: `pgrep gs2emu` only proves anything on a machine with no other
    gservers running. Under Wine, the 1.41 "Getting server list ..." box stays
    drawn over the list after it arrives, but the list is filled in and can be
    selected. This is likely a Wine/Xvfb repaint, not a listserver problem.

## Console excerpts

Step A, listserver:

```
07:37:15 seihai-listserver 0.1.1 starting, config (defaults)
07:37:15 listening on 127.0.0.1:14900 for gservers and 127.0.0.1:14921, 127.0.0.1:14922, 127.0.0.1:14923 for clients; accounts: open (every account gets in)
07:37:15 silent list (shape S): off
07:37:15 gserver 1 connected from 127.0.0.1
07:37:15 gserver 1 closed (no REGISTERV3 line): (no name), entry dropped
07:37:17 gserver 2 newserver Lab Modern port 14805 version 4.0.0-beta (2026-11-11 10:22) ip 127.0.0.1
07:37:17 gserver 3 newserver Lab Classic port 14803 version 4.0.0-beta (2026-11-11 10:22) ip 127.0.0.1
07:37:17 gserver 4 newserver My Server port 14802 version 4.0.0-beta (2026-11-11 10:22) ip 127.0.0.1
```

Step A, `servers/default/logs/console.txt` (classic and modern are the same
apart from the port):

```
[04:37 PM] :: Loading server configuration.
[04:37 PM]      Listening on: 127.0.0.1:14802.
[04:37 PM] :: listserver - Connected to 127.0.0.1:14900.
```

Step B:

```
Welcome to Lab listserver, test1.
There are 3 server(s) online.
  0) [H] Lab Modern  127.0.0.1:14805  4.0.0-beta (2026-11-11 10:22)  players: 0
  1) [H] Lab Classic  127.0.0.1:14803  4.0.0-beta (2026-11-11 10:22)  players: 0
  2) [H] My Server  127.0.0.1:14802  4.0.0-beta (2026-11-11 10:22)  players: 0
--- Ctrl+C through a pty
Lab world running. Stop with Ctrl+C.
^CStopping...
exit status: 0
```

Step C, `setup public` with `--interface 127.0.0.2` (before this PR's message fix):

```
Checks:
  BLOCKED [serverinterface]: A public server must listen on AUTO or a real interface, not 127.0.0.1.
  warning [name]: The server is still called "My Server"; players see this name in the list.
Not saved: fix the BLOCKED items first.
```

Step C on the dummy interface 192.0.2.2 (it exists only in the namespace):

```
Saved ./servers/default/config/serveroptions.txt.
Backup: ./servers/default/config/serveroptions.txt.bak-20261112-080024 (undo with `setup undo`).
--- setup start default --for 30
Listening on:            [ ok ] 192.0.2.2:14802
Listserver:              [ ok ] connected to 127.0.0.1:14900
Reachable from outside:  [ ?? ] cannot be checked from this machine: ask someone elsewhere to connect
Stopping...
--- check-consoles.sh --addr 192.0.2.2 . default
PASS  default: listening on 192.0.2.2:14802
PASS  default: linked to 127.0.0.1:14900
PASS  default: no socket or listserver errors
all checks passed
```

Step D, listserver with the real 1.41 rev 1:

```
08:02:15 listening on 127.0.0.1:14900 for gservers and 127.0.0.1:14921, 127.0.0.1:14922, 127.0.0.1:14923 for clients; accounts: 1 account(s) from ./accounts.txt
08:02:15 silent list (shape S): after 500 ms
08:03:19 client 5 on 14923 from 127.0.0.1
08:03:20 client 5 sent nothing; sending the list
08:03:20 client 5 closed
```

The client showed "Active Servers: My Server 0, Lab Classic 0, Lab Modern 0".
A screenshot is kept on the lab host.
