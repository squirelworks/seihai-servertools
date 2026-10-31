# Lab how-to

Every run is on `127.0.0.1`, in a VM, a container or CI, with servers you
start yourself (`AGENTS.md`). Write each run up in this folder as
`YYYY-MM-DD-<what>.md`: what was run (versions, commits), what happened
before and after, and the log files.

## A lab world

1. Build or unpack `seihai-gserver-v2` (its `bin/` folder is the world:
   `gs2emu`, `servers/default`, `servers/classic`; see that repo's
   `bin/docs/lab-world.md`).
2. Build `seihai-listserver` (`cargo build --release`) and copy the binary
   into the world folder.
3. Build this repo and copy `target/release/seihai-servertools` and
   `scripts/lab/start.sh` into the world folder.
4. `./start.sh` (or `seihai-servertools lab start --world .`).

Check: `logs/listserver-console.txt` says it listens on `127.0.0.1:14900`;
each `servers/<name>/logs/console.txt` shows the gserver up on its port.

Without the fork, a stand-in `gs2emu` is enough to test the start and stop
logic:

```sh
#!/bin/sh
echo "stand-in gs2emu $*"; trap 'echo got TERM; exit 0' TERM; while :; do sleep 1; done
```

## A client

Seihai's `seihai-mae` logs in headless (see `docs/lab/HOWTO.md` in
`seihai-rc` and `07-lab-testing.md` in the project handoff):

```
export GRAAL_PW=labpw
seihai-mae --online 127.0.0.1:14922 --list-only --account test1 --password-env GRAAL_PW --version 2.22
```
