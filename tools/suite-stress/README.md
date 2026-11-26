# seihai-suite-stress

The integration and stress rig for the Seihai server suite: it writes a
throwaway lab folder, starts seihai-listserver, seihai-gserver (with
seihai-npcserver) and seihai-rc on `127.0.0.1`, drives bot clients at them
and writes a report. [docs/lab/suite-stress.md](../../docs/lab/suite-stress.md)
explains the report.

It is one tool of [seihai-servertools](../../README.md), built, tested and
run on its own. It is not part of the all-in-one `seihai-servertools`
program or the window.

## Build, test, run

From the repository root:

```sh
cargo build --release -p seihai-suite-stress    # target/release/seihai-suite-stress
cargo test -p seihai-suite-stress
target/release/seihai-suite-stress run --bin-dir path/to/server/programs
```

The server programs themselves come from their own repositories; give
their folder with `--bin-dir` (default: next to this program).

## Depends on

- ctrlc
- seihai-core: used directly (codecs and version profiles for the bots), at
  the one pin in the root `Cargo.toml` (`[workspace.dependencies]`, tag
  `v0.7.2` of `squirelworks/seihai-core`, never a branch).

It does not depend on any other tool in `tools/`.

## Usage

```text
seihai-suite-stress: run the Seihai server suite on 127.0.0.1 and drive bots at it

USAGE:
  seihai-suite-stress run    [options]   bots for --stay seconds, then a report
  seihai-suite-stress compat [options]   one bot per version profile (--versions all),
                                         5 s each: which profiles the suite takes today
  seihai-suite-stress versions           list every Seihai version profile
  seihai-suite-stress probe ADDR VERSION ACCOUNT STEPS
                                         one client against a loopback gserver you
                                         started: log in, run the STEPS file, print
                                         every server message (see docs/lab/probe/)

OPTIONS:
  --bin-dir DIR        where seihai-gserver, -listserver, -npcserver and -rc are
                       (repeatable; the first hit wins). Default: next to this binary.
  --out DIR            lab folder and results (default: ./suite-stress-out/<time>)
  --base-port N        gserver N+2, NPC link N+10, listserver N+100 / N+121 (default 24800)
  --bots N             bot clients (default 24)
  --versions LIST|all  comma-separated version labels, or all (default: a spread from 1.21 to 6.03r7)
  --stay SECS          how long each bot stays logged in (default 60)
  --ramp N             bots started per second (default 20)
  --list-fraction F    share of bots that fetch the server list first (default 1.0)
  --walk-ms MS         walk props every MS (default 100, as real 2.x)
  --chat-secs S        a chat line every S seconds (default 5)
  --probe-secs S       a file request (round-trip probe) every S seconds (default 2)
  --warp-secs S        take a link to the next ring level every S seconds (default 3; 0 = off)
  --levels N           levels in the warp ring written to the world (default 8; 0 = none,
                       for a gserver before stage 2.2)
  --gmap N             an N by N gmap whose cell (0, 0) is the start level (default 3;
                       0 = none). Builds the server puts on it cross cells every
                       --warp-secs instead of taking links
  --rc-every S         run seihai-rc servers and players list every S seconds (default 10; 0 = off)
  --max-players N      gserver max_players (default 2048)
  --list-max-connections N   listserver max_connections (default: its own default)
  --no-npcserver       leave the NPC-server out
  --no-listserver      leave the listserver out
  --verbose            start the gserver with --verbose
  --anticheat on|off   anticheat in the gserver's server.txt (default: its own default)

EXTERNAL CLIENTS (another client program next to the bots, one process each):
  --external PROG      the program, e.g. graal-delphi's Graal205. It is started with
                       --server --account --password-env --headless --seconds --walk
                       --walk-ms --seed [--link-every] and must print one JSON line
                       with "kind":"summary" on stdout when it ends
  --external-clients N how many (default 1 when --external is given)
  --external-arg ARG   one more argument for each of them (repeatable), e.g. --nodelay
  --external-link-secs S   --link-every for them (default: --warp-secs; 0 = none)
```
