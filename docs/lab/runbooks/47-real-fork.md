# Runbook: real-fork lab start, go-public, and 1.41 rev 1 (#47)

Closes three `docs/NEXT.md` boxes: #47, "Step 4 lab run" and "Lab run with
a real `gs2emu`". Linux, loopback only. About an hour. First run:
`docs/lab/2026-11-12-real-fork-lab-run.md` (this runbook was corrected from it).

If the machine already runs another lab on these ports, run everything in
its own network namespace (`bwrap --unshare-user --unshare-net`, or
`unshare -rn`) and leave the other lab alone.

## Set up

1. Unpack a seihai-gserver-v2 release that has `servers/modern` into
   `~/lab47`. Note its version or commit.
2. Build `seihai-listserver` (`cargo build --release`) and this repo, and
   copy both binaries, `scripts/lab/start.sh` and
   `scripts/lab/check-consoles.sh` into `~/lab47`. Note both commits.
3. Have `seihai-mae` built for step B. It is in seihai (`crates/seihai-mae`),
   not seihai-rc; `--no-default-features --features dev` builds it without
   the audio libraries.

## A. `lab start` with the real fork

```sh
cd ~/lab47 && ./start.sh          # leave it running; second terminal:
./check-consoles.sh .
```

Expected: every row PASS. The listserver on `127.0.0.1:14900`; `default`,
`classic` and `modern` each "Listening on: …:14802/14803/14805" and
"listserver - Connected to 127.0.0.1:14900". The real lines have a
timestamp prefix; that is fine.

The listserver logs `gserver 1 connected` then `closed (no REGISTERV3 line)`
before the real gservers register: that is `lab start`'s own probe, not a
failure. Run `check-consoles.sh` while the world is running: on Ctrl+C every
gserver logs `listserver - Disconnected.` (the listserver stops first), which
the check counts as an error.

If a row fails, open that console. If the server is fine but prints its lines
differently from the fixtures (for example no colon after "Listening on"),
copy the real lines into the run note: `lights.rs` and its table need the fix
in the same PR (#47 step 3).

## B. A client sees the list

```sh
export GRAAL_PW=labpw
seihai-mae --online 127.0.0.1:14922 --list-only --account test1 --password-env GRAAL_PW --version 2.22
```

Expected: three entries, one per server. Stop `start.sh` with Ctrl+C:
"Stopping...", every process gone, exit 0. (`pgrep gs2emu` only proves this
when no other gserver runs on the machine; otherwise check the listening
sockets.)

## C. Go public on a lab "outside" address and back

`setup public` refuses every loopback address (all of 127/8) for a public
server, so `127.0.0.2` can't stand in for the outside interface. Use a dummy
interface with a documentation address, inside the network namespace so it
never reaches a real network:

```sh
ip link add lab0 type dummy && ip addr add 192.0.2.2/32 dev lab0 && ip link set lab0 up
```

A stock release world also blocks on `accounts/YOURACCOUNT.txt` (password
`super`): move it aside for this step and put it back after. `--yes` needs
`--account` for the staff list.

```sh
./seihai-servertools setup status --world . default
mv accounts/YOURACCOUNT.txt /tmp/      # put back after step C
./seihai-servertools setup public --world . default --interface 192.0.2.2 --listserver 127.0.0.1 --account labstaff --yes
./start.sh classic &                   # the listserver (and classic), not default
./seihai-servertools setup start --world . default --for 30
./check-consoles.sh --addr 192.0.2.2 . default
```

Expected: `setup public` prints a preview and keeps a dated backup of
`serveroptions.txt`. `setup start` shows two `[ ok ]` lights (listening on
`192.0.2.2:14802`, connected to the listserver) and `[ ?? ]` for "Reachable
from outside", which a lab machine can't check. The console check is all PASS.

Don't use `lab start --world .` here: it would also start `default`, now on
the public address, and the port would be taken.

Then:

```sh
./seihai-servertools setup lab --world . default --yes
./seihai-servertools setup status --world . default   # back to the lab values
./seihai-servertools setup undo --world . default     # restores the public save
./seihai-servertools setup lab --world . default --yes
mv /tmp/YOURACCOUNT.txt accounts/
```

Expected: `setup lab` puts the interface, `serverip`, `localip` and
`localhost` back to the lab values. A `diff` of `serveroptions.txt` against
the first dated backup still shows `staff` with the new account (`setup lab`
only sets the network values) and one `# Changed by Seihai server setup` line
per changed key. Those lines don't pile up on repeated runs.

## D. Real 1.41 rev 1 gets the list

```sh
./seihai-servertools setup host-listserver --world . --bind 127.0.0.1 --silent-list-ms 500 --yes
./seihai-listserver add-account accounts.txt test1 --password-env GRAAL_PW
./start.sh
wine Graal1.41r1.exe        # point it at 127.0.0.1 (its usual list port)
```

Expected: the client shows the server list within a second or two. The
listserver console logs the 1.41 connection and the list sent after the
500 ms silence. 1.41 rev 1 dials its built-in list name: map it to
127.0.0.1 in a hosts file the client sees (a sandboxed `/etc/hosts` in the
namespace works). `host-listserver` points the listserver at `accounts.txt`,
which is why the account is added first; the silent list itself needs no
login, because 1.41 sends nothing. Then `setup listserver-lab --world . --yes`,
which says "nothing to change" after `--bind 127.0.0.1` and leaves `accounts`
and `silent_list_after_ms` in place.

## Result table for the run note

| Step | Expected | Seen | Pass |
|---|---|---|---|
| A listserver | 127.0.0.1:14900 | | |
| A default/classic/modern | listening, linked | | |
| A console lines match the fixtures | yes | | |
| B seihai-mae --list-only | 3 servers | | |
| B Ctrl+C | all stopped, exit 0 | | |
| C setup public + start | 192.0.2.2:14802, linked, 2 ok + 1 ?? | | |
| C setup lab / undo / lab | network values back to lab | | |
| D 1.41 rev 1 | gets the list | | |

Done when the note is in `docs/lab/`, the three NEXT boxes are ticked, and
any `lights.rs` fix is in the same PR.
