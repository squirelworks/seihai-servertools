# Suite stress, first runs (2026-11-04)

The first runs of `seihai-suite-stress` (`docs/lab/suite-stress.md`) against
what was merged on 2026-11-04. Cloud container, 4 cores, 16 GB, everything on
`127.0.0.1` (the Wine run inside a network namespace with only `lo`).

| part | commit |
|---|---|
| seihai-gserver | `082a10a` (stage 2.1, the world task owns who is online) |
| seihai-listserver | `e72bb43` |
| seihai-npcserver | `61a4423` (`seihai-core` v0.0.5) |
| seihai-rc | `d7f84ef` (stage 2 first slice, players on the command line) |
| bots | `seihai-core` at this repo's pin (`72bb8d2`) |

## Compatibility: every version profile

`seihai-suite-stress compat`: one bot per Seihai version profile, list fetch
then gserver login, 5 s each.

- **All 46 profiles got the list and logged in**, from 1.21 to 6.03r7 Linux:
  1.21, 1.28 beta 2, 1.3 test, 1.31–1.39, 1.41, 1.42, 2.00–2.05 (and 2.02
  rev2), 2.1, 2.12–2.17, 2.22, 2.3, 2.31, 3.04, 3.153, 4.04r2, 4.20r6,
  4.20r8, 5.00r7, 5.12, 5.24r4, 5.30r3, 5.31r4, 6.00r2, 6.015, 6.034,
  6.03r7 and 6.03r7 Linux. None was excluded.
- No drops, no server log lines of note, file probes answered for every build
  that reads `PLO_FILESENDFAILED` (1.21 reads no file answer, so it has no
  probe).
- Login p50 105 ms, max 168 ms. Most of it is the gserver hashing the new
  account's password (PBKDF2, 100 000 rounds) on its blocking pool.
- The 1.41 list fetch takes 3 s: the listserver's `silent_list_after_ms`
  defaults to 0 (off), so the bot waits for nothing, then falls back to the
  classic login (Seihai's own fallback; the real 1.41 rev 1 has none). See
  "Findings".

What this does not show: 3.x–6.x log in, but the gserver is stage 2.1, so
there is no Login lobby, no server warp and no level traffic yet. "Logs in
and stays" is all that is checked for any build.

## Load

| run | bots | versions | stay | ramp | result |
|---|---|---|---|---|---|
| run92 | 92 | all 46, two each | 90 s | 10/s | 92 in, 0 dropped, 0 probes lost |
| run300 | 300 | all 46, round robin | 60 s | 30/s | 300 in, 0 dropped, 0 probes lost |

| | run92 | run300 |
|---|---|---|
| login ms p50 / p95 / p99 / max | 93 / 121 / 135 / 135 | 217 / 2609 / 2769 / 2879 |
| file round trip ms p50 / p95 / p99 / max | 2.3 / 2.6 / 4.1 / 48 | 2.4 / 8.9 / 61 / 195 |
| client → server | 871 msgs/s | 2548 msgs/s |
| gserver RSS start → peak, MiB | 5.0 → 8.7 | 4.9 → 13.2 |
| gserver CPU (mean over run) | 24 % | 68 % |
| gserver peak threads / fds | 10 / 107 | 63 / 315 |
| npcserver RSS, CPU | 3.4 → 3.7 MiB, 8 % | 3.5 → 4.3 MiB, 14 % |
| listserver RSS, CPU | 3.3 → 4.0 MiB, 0 % | 3.4 → 4.5 MiB, 1 % |
| list player count vs online | 90 of 92 | 299 of 300 |

- No server process exited, and no RSS kept growing once the bots were in
  (slope over the second half +0.00 to +0.04 MiB/min).
- The 300-bot login tail (p95 2.6 s) is the PBKDF2 hashing of 300 new
  accounts on 4 cores: 51 CPU-seconds in the gserver, almost all of it in the
  first 10 s. Round trips for bots already in stayed at p50 2.4 ms during
  that burst.
- Every walk goes to the NPC-server too (`PLAYER_PROPS` on the link). At
  300 walkers × 10 a second the NPC-server used 14 % of a core, so it is not
  near a limit at this size.
- The list's player count trails the bots by one or two: each bot reads the
  list before it logs in.

## Real client under Wine

A 24-bot run (12 versions from 1.21 to 6.03r7) with the real 1.39
(`graal139`, "One Player Online", server `127.0.0.1`, port 24802, Wine 9,
`nodirectinput=true`) logging in during it. The gserver logged it as
`GNW07080 1.39, Gen2`, player 25; it drew the start board and its walk props
reached the server (52 prop updates in 20 s of arrow keys). The bots were
not disturbed. Screenshot: project files `suite-stress/wine-139-on-rust-suite.png`.

2.05 and later find their list through their built-in listserver host name,
so they need `--base-port 14800` (list on 14921/14922) and that name pointed
at `127.0.0.1` inside the namespace, plus a 2.x game folder. Not done in
this run.

## RC

`seihai-rc servers` against the listserver: 17 of 17 worked (about 330 ms
each, most of it the RC list's tail wait). `seihai-rc players list` against
the gserver: 0 of 17, expected, since the gserver refuses RC logins until
stage 4. But the refusal does not reach the admin: the gserver answers the
gen-5 RC login (type 6) with its message in a plain zlib frame, and
`seihai-rc` prints `login failed: unknown gen-5 compression byte 0x78`
instead of the server's text. See "Findings".

## Findings

1. **seihai-gserver: a refused login of a known type goes out in the wrong
   framing.** A login type the gserver knows but does not serve yet (6, the
   gen-5 RC) is refused in a plain zlib frame ("the generation is not
   known"), but the type says it is gen 5, so a gen-5 client cannot read
   the reason. `seihai-rc` shows `unknown gen-5 compression byte 0x78`.
2. **Listserver defaults leave the real 1.41 rev 1 without a list.**
   `silent_list_after_ms = 0` means a client that connects and waits gets
   nothing. Seihai's 1.41 profile falls back after 3 s; the real client does
   not. Hosting a listserver from servertools should turn it on.
3. **Defaults below the 1000-player test:** gserver `max_players = 256`,
   listserver `max_connections = 256`. Not bugs, but the 1000-player run
   needs both raised, and servertools' hosting setup should say so.
4. **PBKDF2 at 100 000 rounds costs about 0.15 CPU-seconds a login.** A
   1000-player reconnect storm is about 150 CPU-seconds of hashing. The
   blocking pool keeps round trips for players already in low, but logins
   queue. Worth a limit on concurrent hashes, or a lower cost for verify
   against create, before the 1000-player run.

## Not covered yet

- Other players' props, warps, gmaps, files, NPCs: the gserver has none of
  them yet (stage 2.2 on). The bots already walk and probe, so the same
  runs will load them as they land.
- NPC-server scripts on players (milestone 3).
- RC against the gserver (stage 4).

## Later the same day: gserver stage 2.4 (squirelworks/seihai-gserver#18)

The rig now warps (an 8-level ring of full boards). Against the #18 branch
(`2c87935`, on main `9c5853a` with stages 2.2 and 2.3):

- `compat` with a warp every second: all 46 profiles logged in and warped
  (183 warps, none lost).
- `run --versions all --bots 92 --stay 90`: 92 in, 0 dropped, 2668 warps,
  none lost, link → name p50 4.2 ms, max 46 ms.
- **Gen 2 loses frames during warps.** 20 bots of one build, a warp every
  second and a file request every 0.5 s: 1.32 lost 11 of 985 file answers and
  1.39 lost 8 of 1042, while 2.05 lost none, and 1.32 lost none with warps
  off or on the pre-stage-2 gserver. Most likely a reply goes out between the
  gen-2 raw-data header frame and the board frame. Reported on #18.
- **Other players cost one frame per update per receiver** since stage 2.3:
  server → client went from 52 to 10,589 frames/s for 92 walkers, and gserver
  CPU from 24 % to 74 %. squirelworks/seihai-gserver#20 (batch on the 2.8
  tick).
