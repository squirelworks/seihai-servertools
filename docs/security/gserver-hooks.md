# Hooks that belong in seihai-gserver

Chat limits, gather cooldowns, movement and inventory authority, and the
monitoring signals on this page run in the game server. They read live
packets. `seihai-servertools` edits `serveroptions.txt` and the
listserver TOML, starts lab processes, and checks a world folder on
disk. It does not sit on the player socket, so it cannot enforce any
row below. Do not add an anticheat crate to this repo.

[seihai-gserver](https://github.com/squirelworks/seihai-gserver) already
reserves this work in its own `docs/PLAN.md` (stage 7, `src/anticheat.rs`):
movement, warps, items, a per-connection message rate, and a policy that
can log, correct, kick, or ban. The numbers in that plan are that
repo's to settle. This page is the operator list that stage should
cover, taken from the abuse picture in
[threat-overview.md](threat-overview.md). Suggested alert lines are
starting points for a loopback lab. They are not shipped defaults, and
they are not a second copy of the gserver plan.

The GPL fork does not gain these limits by anything committed here.

## Rate limits

Old helpers drive the real client (keys into the focused window, or
chat typed on a timer). The bytes look legal. Caps are what make them
expensive.

| Limit | Why | Starting point to lab |
|---|---|---|
| Chat messages per account | Timed AFK bots and scrollers | Alert above 20 per minute, sustained for 5 minutes |
| Identical chat text | The same phrase on a loop | Alert at 10 repeats in 5 minutes |
| Shout and to-everyone | Text-sender flood modes | Stricter than normal chat |
| Nick and appearance changes | Burst setnick / setbody | A small per-minute budget |
| Gather, dig, pray, water, interact | Macros stood on one tile | Cooldown plus a yield cap; alert above 40 interacts per minute on one tile for 10 minutes |
| Messages per second, all kinds | A modified client flooding the socket | A soft drop and a hard kick, with room for real walking (the gserver plan calls this out) |

Online-time and AFK rewards in NPC scripts should require movement or a
deliberate interaction. Chat alone is not enough. That rule lives in
the world's scripts; the gserver cannot see the author's intent, but it
can expose a reliable "last moved" fact scripts are allowed to trust.

## Packet authority

A packet log on a modified client can replay or invent player messages.
Authority stays on the server:

| Control | Rule |
|---|---|
| Movement | Reject speed over the configured max, positions outside the level, and teleports the server did not cause. Alert on any over-speed once the check exists |
| Inventory, chests, gralats, hearts | The server's count wins. A client raise needs a source the server already granted |
| Combat | Damage is checked against range and weapon on the server |
| Staff, RC, and NC message ids | Refuse them on a normal player session |
| Unknown message ids | Count them. Alert above 3 unknown ids in a session. A single unknown id is not a kick by itself (real builds send ids a young server may not handle yet) |
| Frame size | Drop or disconnect an oversized frame |
| Version token | Log it. Do not treat it as proof of what the client will do. `SEI00001` is the Seihai secure login, not a generic "trusted client" flag; classic clients cannot send it |
| Scripts the client can read | No secret reward that is safe only because the client is trusted |

`seihai-gserver` and `seihai-listserver` already speak `SEI00001`.
Requiring it refuses classic clients. Document that choice; do not
silently turn it on from `seihai-servertools`.

## Signals worth paging a person

| Signal | Suggested alert |
|---|---|
| Chat messages per account per minute | Above 20, sustained for 5 minutes or more |
| The same message text | 10 or more times in 5 minutes |
| Interact or use on one tile | Above 40 per minute for 10 minutes or more |
| Speed over the configured max | Any, once movement authority exists |
| Unknown message ids | Above 3 in a session |
| Version token outside the world's allow-list | Info by default; kick only if the owner chose a strict policy |
| RC or NC login from outside the staff address set | Page the owner |
| A new gserver registration | Page the listserver host |
| Failed passwords from many addresses | Lockout on the listserver or gserver |

Listserver-shaped rows (registration, password lockout, the allow-list)
belong in `seihai-listserver`. Until the allow-list exists, keep
registration port 14900 reachable only from your own gservers. The
setup wizard already warns about that.

## What stays in seihai-servertools

| Concern | Where |
|---|---|
| `localhost`, listen address, UPnP on loopback, `YOURACCOUNT`, placeholder `listip`, open listserver accounts | `crates/setup-core` Blocks. See [threat-overview.md](threat-overview.md) |
| Lab account files left in `accounts/` | A warning from `check_server_folder`, not a packet check |
| Levels, links, gmap holes | `seihai-servertools check` |
| Publishing this list in the window | Only after the gserver emits the signals. The window does not invent them |

## Lab, when the gserver side exists

On `127.0.0.1` only, with accounts you created:

- chat faster than the cap, from a test account, ends in mute or kick;
- interact held on one tile hits the cooldown;
- a position the server did not grant is put back;
- a player session that sends a staff-only id is dropped;
- a normal walk with a real client produces an empty violation log.

Record the client version and the gserver commit in `docs/lab/` of the
repo that gained the check. Do not run recovered helpers, and do not
aim the test at `<public-host>`.
