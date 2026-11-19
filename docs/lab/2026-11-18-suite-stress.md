# Suite stress rerun (2026-11-18)

The `compat` pass and a 92-bot `run` of `seihai-suite-stress`
(`docs/lab/suite-stress.md`) against the server suite as merged on
2026-11-18. Debian lab VM (4 GB RAM, shared with another lab), everything on
`127.0.0.1` inside a loopback-only network namespace. The bots ramp at 5 a
second (half the 2026-11-04 rate) to keep the shared VM comfortable.

| part | commit |
|---|---|
| seihai-gserver | main `0afe61d` plus seihai-gserver#159 (early other-player props) |
| seihai-listserver | main `d542419` |
| seihai-npcserver | main `5a57df5` |
| seihai-rc | main `5c23ba3` |
| seihai-servertools (rig) | main `44ed39a` plus this branch's RC account fix |

## Compatibility: every version profile

`compat --versions all`: **all 49 profiles got the list and logged in**,
1.21 to 6.03r7 Linux, including the 4.04r2 Mac, 5.12 Mac and 6.00r4 profiles
added since 2026-11-04. No drops, no server log lines of note. Login p50 30 ms,
max 111 ms.

## Load: run92

`run --versions all --bots 92 --stay 90 --ramp 5`

| | run92 |
|---|---|
| bots in / dropped / peak online | 92 / 0 / 92 |
| list fetches | 92 of 92 |
| login ms p50 / p95 / p99 / max | 40 / 111 / 216 / 216 |
| file round trip ms p50 / p95 / p99 / max | 3.4 / 19 / 35 / 75 (4050 answered, 0 lost) |
| ring warps ms p50 / p95 / max | 10 / 33 / 98 (1176 done, 0 lost) |
| gmap seam crossings ms p50 / p95 / max | 10 / 35 / 92 (700 done, 0 lost; 50 bots on the gmap) |
| client → server, server → client | 801 msgs/s, 3217 frames/s |
| list player count vs online | 91 of 92 |
| gserver RSS start → peak → end, MiB; slope | 6.6 → 11.1 → 11.1; +0.03 MiB/min |
| gserver CPU (mean), peak threads / fds | 19 %, 7 / 108 |
| npcserver RSS, CPU | 3.4 → 3.6 MiB, 5 % |
| listserver RSS, CPU | 4.2 → 5.1 MiB, 0 % |

No server process exited, and no RSS kept growing once the bots were in.

## Findings

- **The `players list` RC probe was always refused.** The rig logs its RC in
  as `stressrc`, but an RC login never makes an account (`create_accounts`
  is for players), and the gserver admits only `STAFF` accounts to RC. Every
  probe failed with "Account name or password is wrong" in the first run.
  Fixed on this branch: the rig makes `stressrc` with
  `seihai-gserver account` before the gserver starts and marks it
  `STAFF 1`. In the rerun all 6 probes logged in.
- **`players list` under load takes the full 10 s timeout.** With 92 bots
  warping every 3 s the list never stays quiet for the 500 ms `--settle`,
  so the probe ends on its timeout. The same command against 4 bots
  returns the full list at once. Not changed here.
- The 1.41 list fetch still takes about 3 s (the rig's listserver keeps
  `silent_list_after_ms` at 0, as noted on 2026-11-04).

## Not covered

- Real 2.x clients under Wine next to the bots (`--base-port 14800`): open.
- RC bots beyond the two probes.
