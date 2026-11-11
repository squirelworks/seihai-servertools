# Probe: one scripted client

`seihai-suite-stress probe ADDR VERSION ACCOUNT STEPS` logs in to a loopback
gserver you started yourself as one version profile (password `labpw`; a lab
gserver with `localhost = true` takes any), runs a steps file and prints every
message the server sends, with a timestamp. `STEP_MS` sets the wait after each
step (default 300 ms). Only `127.0.0.1` / `localhost` are accepted.

It came from seihai-gserver-v2's `tools/lab/npcprobe` (moved 2026-11-11), where
it drove that fork's NPC-server lab
(`docs/lab/2026-10-31-npcserver-gs1.md` there).

Steps file, one per line, `#` for comments:

| Step | Sends |
|---|---|
| `wait SECONDS` | nothing; reads from the server |
| `walk X Y [DIR]` | a position (DIR defaults to 2) |
| `chat TEXT` | a chat line |
| `trigger X Y ACTION [PARAMS]` | a trigger action |
| `warp LEVEL X Y` | a level warp |
| `enter` | the enter-level messages again |

Files here:

- `npcdemo-steps.txt`: touches every NPC in seihai-gserver-v2's `npcdemo`
  world (`servers/npcdemo`, 127.0.0.1:14804).
- `microtests.gs1`: chat-driven GS1 micro tests. Put it as an NPC script in a
  lab level and say `t1`, `t2`, ... next to it.

Example, from a seihai-gserver-v2 `bin/` folder with `gs2emu --server npcdemo`
running:

```
seihai-suite-stress probe 127.0.0.1:14804 2.31 probe1 path/to/npcdemo-steps.txt
```
