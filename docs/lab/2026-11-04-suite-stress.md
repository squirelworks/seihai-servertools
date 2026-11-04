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

## Stage 2.5 gmaps, and the follow-up PRs (squirelworks/seihai-gserver#19–#23)

The rig now also writes a 3×3 gmap whose cell (0, 0) is the start level, and
bots the server puts on it cross seams instead of taking links.

- **#22 (gmaps, with #18):** compat put 25 builds on the gmap (2.1 to
  6.03r7 Linux; the 21 older builds get the start level as a plain level),
  all 46 logged in. 92 bots, 90 s: 1450 crossings, 0 lost (p50 3.4 ms, max
  48 ms), 1218 warps, 0 lost, 0 file answers lost. The gen-2 warp fix
  (`ae58da6`) holds: 1.32 and 1.39 now lose 0 of 1194 answers each.
- **#19 + #21 (RC refusal, password-check cap):** `seihai-rc` now shows
  "This server does not take RC logins yet." 300 logins at 30 a second
  with walking off: login times unchanged (CPU-bound), but players already
  in see file round trips p99 30 ms instead of 845 ms, and the blocking pool
  stays at 12 threads instead of 64.
- **#23 (props batched on the 50 ms tick):** 92 bots in one level, frames
  27,558 → 1,818 a second and gserver CPU 115 % → 43 %; 300 bots in one
  level, RSS growth +7.2 MiB/min → flat and the slowest login 13.7 s → 5.2 s,
  but still near a full core.

## #25 with per-worker zlib, the file p95, and a second client program

- **#25 at `15637a3`** (zlib state per worker thread): 300 bots in one
  level, all 300 in, gserver RSS peak 26.9 MiB (113 MiB before this commit,
  20.7 on #23) and 60.9 CPU-s (78.8 on #23).
- **The file p95 tail was the bots.** `seihai-core`'s client socket has no
  `TCP_NODELAY`, so a `WANTFILE` sent right after a walk prop waits for the
  server's delayed ACK (about 40 ms). The same tail showed with the servers
  and the bots pinned to separate cores and with 10 bots, on main and #23
  too. A scratch rig with `set_nodelay(true)` in `connect_any`: 92 bots
  p95 39.6 → 3.8 ms, 10 bots 30.6 → 2.7 ms. Until seihai-core sets it,
  file p95 from this rig measures the bots, not the gserver.
- **graal-delphi's rebuilt 2.05r2 as external clients** (graal-delphi#5,
  `--external`): 92 bots of all 46 profiles plus 20 `Graal205 --headless`
  walking at random and taking a link every 3 s, 60 s, on #25. All 92 bots
  and all 20 external clients in and ended clean, external login p50 151 ms,
  134 of 135 links taken (the last one tried as the stay ended), client tick
  p99 under 3.6 ms, 4.7 MiB each. gserver 53 % of a core, 12.1 MiB.

## gserver stage 2.9 anti-cheat (claude/anticheat, 78728a5)

The first runs against it refused every bot warp (bots asked for the next
level from wherever they stood) and flagged every gmap crossing as a
teleport (bots jumped to the seam). The bots now walk honestly (see
`suite-stress.md`). With that, 92 bots of all 46 profiles, 60 s: no warp,
teleport or rate hits, 0 warps or crossings lost. What is left are speed
hits, reported to the gserver thread as most likely false:

- the first step after a warp arrival is measured from the link in the old
  level (`speed 7.5 tiles to 29.5 34`, arrival 30, 34.5);
- after a logged hit the budget is 0, so honest steps the server reads in a
  backlog come out as runs of hits; the count grows with load (113 gmap
  hits at 92 bots, 1543 with 20 more clients);
- the rebuilt 2.05r2 steps 1 tile at a time and gets long runs of 1.0-tile
  hits.

**Correction:** the slowdown blamed on a noisier host was the rig. The
honest-walk bots (servertools #31) busy-looped while a warp was due and
they had not reached the link yet, and gmap bots always: their wake time
stayed in the past. That made every run since #31 slow (#25's file p50
3 → 42 ms at 92 bots) and capped 300-bot runs at about 100 logins, the
other 200 timing out with the gserver itself at a third of a core. Fixed
in the next PR; the numbers below are with the fix.

**gserver #31 at 517b42e** (anti-cheat fixes for the three false causes
above), with the fix:

| run | logged in | file ms p50 / p95 | warp ms p50 | crossing ms p50 | lost | anticheat lines |
|---|---|---|---|---|---|---|
| compat, all 46 | 46 | 2.8 / 28 | 13 | - | 0 | 0 |
| #25, 92 bots | 92 | 2.3 / 5.8 | 7.0 | 6.5 | 0 | - |
| #31, 92 bots + 20 Graal205 | 92 + 20 | 2.4 / 6.5 | 8.0 | 7.2 | 0 (382 of 382 links) | 0 |
| #25, 300 bots | 271 | 37.6 / 195 | 1608 | 255 | 193 warps, 410 crossings | - |
| #31, 300 bots | 298 | 35.7 / 220 | 1559 | 866 | 198 warps, 355 crossings | 10,610 |

At 300 bots spread over levels and the gmap this 4-core box is saturated
(rig plus suite), on #25 and #31 alike: warps and crossings time out. The
anti-cheat lines at 300 come in one 8 s burst while it is saturated: speed
on 1.6-tile gmap steps (the bots then walked 16 tiles/s, now 10) and warps
to the level the bot already is in (a bot whose warp timed out at 10 s
asks again before its reader has caught up). At 92 there are none.
