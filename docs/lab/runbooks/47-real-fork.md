# Runbook: real-fork lab start, go-public, and 1.41 rev 1 (#47)

Closes three `docs/NEXT.md` boxes: #47, "Step 4 lab run" and "Lab run with
a real `gs2emu`". Linux, loopback only. About an hour.

## Set up

1. Unpack a seihai-gserver-v2 release that has `servers/modern` into
   `~/lab47`. Note its version or commit.
2. Build `seihai-listserver` (`cargo build --release`) and this repo, and
   copy both binaries, `scripts/lab/start.sh` and
   `scripts/lab/check-consoles.sh` into `~/lab47`. Note both commits.
3. Have `seihai-mae` built (seihai-rc) for step B.

## A. `lab start` with the real fork

```sh
cd ~/lab47 && ./start.sh          # leave it running; second terminal:
./check-consoles.sh .
```

Expected: every row PASS. The listserver on `127.0.0.1:14900`; `default`,
`classic` and `modern` each "Listening on: …:14802/14803/14805" and
"listserver - Connected to 127.0.0.1:14900".

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
"Stopping...", every process gone (`pgrep gs2emu` prints nothing), exit 0.

## C. Go public on 127.0.0.2 and back

```sh
./seihai-servertools setup status --world . default
./seihai-servertools setup public --world . default --interface 127.0.0.2 --listserver 127.0.0.1 --yes
./seihai-servertools lab start --world . &        # the listserver, for the link
./seihai-servertools setup start --world . default --for 30
./check-consoles.sh --addr 127.0.0.2 . default
```

Expected: `setup public` prints a preview and keeps a dated backup of
`serveroptions.txt`. `setup start` shows three green lights: listening on
`127.0.0.2:14802`, linked to the listserver. The console check is all PASS.

If `lab start` already runs `default`, the port is taken: start only the
listserver with `./start.sh classic` instead, then `setup start` for
`default`.

Then:

```sh
./seihai-servertools setup lab --world . default --yes
./seihai-servertools setup status --world . default   # back to the lab values
./seihai-servertools setup undo --world . default     # restores the public save
./seihai-servertools setup lab --world . default --yes
```

Expected: `setup lab` puts the interface back to the lab value;
`diff` of `serveroptions.txt` against the first dated backup shows no change
after the last `setup lab`.

## D. Real 1.41 rev 1 gets the list

```sh
./seihai-servertools setup host-listserver --world . --bind 127.0.0.1 --silent-list-ms 500 --yes
./start.sh
wine Graal1.41r1.exe        # point it at 127.0.0.1 (its usual list port)
```

Expected: the client shows the server list within a second or two. The
listserver console logs the 1.41 connection and the list sent after the
500 ms silence. Then `setup listserver-lab --world . --yes`.

## Result table for the run note

| Step | Expected | Seen | Pass |
|---|---|---|---|
| A listserver | 127.0.0.1:14900 | | |
| A default/classic/modern | listening, linked | | |
| A console lines match the fixtures | yes | | |
| B seihai-mae --list-only | 3 servers | | |
| B Ctrl+C | all stopped, exit 0 | | |
| C setup public + start | 127.0.0.2:14802, linked, 3 lights | | |
| C setup lab / undo / lab | back to lab values | | |
| D 1.41 rev 1 | gets the list | | |

Done when the note is in `docs/lab/`, the three NEXT boxes are ticked, and
any `lights.rs` fix is in the same PR.
