# Lab start with original Windows clients on the Rust gserver (2026-11-18)

Closes the part of roadmap #50 that `2026-11-14-lab-start-rust-listserver.md`
left open: original Windows clients getting the list from a `lab start`
listserver and logging in to the Rust gserver itself. **Result: PASS.**

Machine: the Debian lab VM. Everything ran inside a loopback-only network
namespace (`bwrap --unshare-net`, only `lo`, old list host names mapped to
127.0.0.1 by a private hosts file). The clients ran under user-space Wine on
Xvfb screens and were driven with XTest. Screenshots and logs stay in the
private lab folder; this page has the derived result.

## Builds

| Component | Commit |
|---|---|
| seihai-servertools | main `44ed39a` |
| seihai-listserver | main `d542419` |
| seihai-gserver | main `0afe61d` plus seihai-gserver#159 (early other-player props; does not touch login) |
| Clients | real Graal 2.22 (`Graal_222.exe`, `GNW03014`) and 2.31 (`Graal_231.exe`, `GNW28015`) from the Reborn 2.22.0 folder |

## World

```
W/
  seihai-servertools, seihai-listserver   release copies
  seihai-listserver.toml                  the repo sample, silent_list_after_ms = 500
  rust/                                   seihai-gserver's sample world/, with in config/server.txt:
                                            port = 14806
                                            listserver = 127.0.0.1:14900
                                            start_level = door45.nw (a 64x64 grass level with two door NPCs)
                                            create_accounts = true
                                          and allowedversions.txt = *
```

## Run

```
seihai-servertools lab plan  --world . --rust rust --rust-gserver ./seihai-gserver
seihai-servertools lab start --world . --rust rust --rust-gserver ./seihai-gserver
# 2.22, then 2.31: log in with a new account, pick "Seihai Lab" on the list
# then SIGINT to `lab start`
```

## Result table

| Step | Seen | Pass |
|---|---|---|
| `lab plan` | `listserver ... (gservers register on 127.0.0.1:14900)` and `rust  rust on 127.0.0.1:14806` | PASS |
| `lab start` | started listserver and rust; gserver `listserver link to 127.0.0.1:14900: registered (encrypted)`; listserver `encrypted link (SEI00001 SVLINK01)`, `newserver Seihai Lab port 14806` | PASS |
| real 2.22 | list shown with Seihai Lab; `st222 [GNW03014 2.22, Gen5] logged in as player 1`; in the start level with its HUD, both door NPCs drawn; listserver `player add 1 ... st222` | PASS |
| real 2.31 | same path; `st231 [GNW28015 2.31, Gen5] logged in as player 2`; draws st222 by name in the same level; listserver `player add 2 ... st231` | PASS |
| SIGINT | `Stopping...`; gserver `stopping: saved 0 online player(s)` (the clients had quit); every process of the run gone | PASS |

## Not covered

- 1.x clients through `lab start` (1.39 dials the gserver directly and needs no list).
- 4.x to 6.x through the Login lobby (the sample world has `lobby_port = 0`).
- The LAT account session (separate checkbox).
