# Suite stress rig (`seihai-suite-stress`)

`crates/suite-stress` starts the whole Rust server suite on `127.0.0.1` and
drives bot clients at it, so each part can be stress-tested on its own and
together, and the same run can be repeated as the parts grow.

What one run does:

1. Writes a throwaway lab folder: a listserver TOML (open mode, one client
   port, the encrypted gserver link on) and a gserver world
   (`create_accounts = true`, `allowedversions.txt` = `*`, `npcserver =
   auto`, `listserver = 127.0.0.1:…`).
2. Starts `seihai-listserver`, then `seihai-gserver`, which starts
   `seihai-npcserver` itself. It waits for each port.
3. Starts N bots at `--ramp` per second. Each bot is one Seihai version
   profile, speaking that build's wire through `seihai-core` (the same codecs
   the Seihai client uses, so nothing is padded or invented):
   - fetches the server list from the listserver as that build would
     (`--list-fraction` of the bots), and reads the player count our gserver
     shows there;
   - logs in to the gserver and times connect → level and board;
   - walks (walk props every `--walk-ms`, 100 ms like a real 2.x), chats
     every `--chat-secs`, and every `--probe-secs` sends a `WANTFILE` for a
     missing file and times the answer (the round-trip probe);
   - only sends messages its build can send (`Connection::can_send`).
4. Every `--rc-every` seconds runs the real `seihai-rc`: `servers` against
   the listserver and `players list` against the gserver.
5. Once a second samples each server process from `/proc` (RSS, CPU time,
   threads, open files; Linux only) and the bots' counters.
6. Stops the suite and writes `report.md`, `timeline.tsv` (one row per
   second) and `bots.tsv` (one row per bot) next to `lab/` (which keeps the
   server logs).

## Build and run

```
# the four servers, each from its own repo
cargo build --release   # in seihai-gserver, -listserver, -npcserver, -rc
mkdir -p ~/suite-bin && cp */target/release/seihai-{gserver,listserver,npcserver,rc} ~/suite-bin/

# this repo
cargo build --release -p suite-stress
./target/release/seihai-suite-stress versions          # every version profile
./target/release/seihai-suite-stress compat --bin-dir ~/suite-bin --out out/compat
./target/release/seihai-suite-stress run --bin-dir ~/suite-bin --versions all \
    --bots 92 --stay 90 --ramp 10 --out out/run92
```

`compat` is `run` with one bot per version profile for 5 s: it answers
"which builds does the suite take today". `run --help` lists every option.
Ports start at `--base-port` (default 24800: gserver +2, NPC link +10,
listserver gserver port +100, client port +121), clear of the usual 14xxx
lab ports. `--base-port 14800` gives the classic lab ports (gserver 14802,
list 14921) that real 2.x clients under Wine expect.

## Scaling

Each bot is three threads (its own, and the `seihai-core` reader's two).
300 bots ran in this container (4 cores, 16 GB) with the rig itself well
under a core. For 1000 bots raise the open-file limit (`ulimit -n` 8192 or
more), keep `--max-players` above the bot count (the rig writes 2048; the
gserver's own default is 256), and give the listserver
`--list-max-connections` above the number of list fetches in flight (its
default is 256). A 1000-bot login burst costs about 0.15 CPU-seconds per
login on the gserver (PBKDF2, 100 000 rounds), so ramp slowly
(`--ramp 20`) or expect login times of several seconds.

## Real clients next to the bots

Run the rig inside a network namespace whose only interface is `lo` (see
seihai-gserver's `tools/lab-wine.md`), with a long `--stay`, then start a
real client under Wine in the same namespace and point it at the gserver
port. The gserver log shows the real client's login and props next to the
bots'. Loopback only, never a public server.

## Rules

Loopback only (`check_loopback`). Lab accounts and the lab password are
placeholders; the gserver makes each account on its first login. Keep
`out/` folders out of the repo.
