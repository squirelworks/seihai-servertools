# Threat overview

Who this is for: someone running a Seihai-stack world (`seihai-gserver` or
the GPL fork, `seihai-listserver`, staff RC/NC) and someone changing
`seihai-servertools`.

Date: 2026-11-01. Source: a static review of old client-side abuse tools
(readmes and strings only). None of those programs were run. This page
keeps the operator picture and drops sample hashes, archive names, and
decompiler notes.

Companion: [operator-checklist.md](operator-checklist.md). Rate limits and
packet authority: [gserver-hooks.md](gserver-hooks.md).

## Confidence

| Picture | Level | What that means |
|---|---|---|
| Timed chat bots, gameplay macros, chat and appearance floods | High | Several old Windows helpers agree: they drive the real client, they do not open their own game socket |
| A caps-lock toggler filed with the cheats | High | Noise. It does not speak the game protocol |
| An archive the file host refused as abusive | Unknown | Not examined. Treat unknown executables from old packs as hostile, and do not download or run them |
| Version-string fakery and packet logging on modified clients | Medium | Seen in neighbouring client packs, not in the helper set above |
| Cleartext classic logins, a 2003 listserver binary that phones home, `localhost = true`, open gserver registration | High | Already called out by `AGENTS.md` and the setup wizard |

## What shows up on a live world

1. **Timed chat bots.** An external program focuses the client and types
   into chat on a timer (often about every 3 seconds). The server sees
   ordinary chat with a mechanical gap. Stock phrases cluster around
   "I am AFK" and "I am using an AFK Bot". A scroller grows one letter at
   a time. The point is usually online-time or a reward that treats chat
   as proof the player is there.
2. **Gameplay macros.** The player stands on a resource, focuses the
   client, and a helper sends keys and mouse input (gather, dig, pray,
   water, walk). Again the server sees a normal client, held on one tile,
   repeating one action.
3. **Chat and appearance floods.** A text-sender window posts shouts,
   to-everyone lines, nick changes, and body changes through the focused
   client. One family announces itself with the window title "Graal Text
   Sender". A hostile script in the same family sets nick and body and
   then counts down in chat.
4. **Unexamined hostile files.** One pack was blocked by the file host
   before anyone here could read it. It stays out of this repo. Do not
   fetch it to "finish the review".
5. **Misfiled harmless tools.** A caps-lock toggler was stored next to
   the helpers above. A matching file name is not proof of a cheat, and a
   harmless name is not proof of safety.
6. **Stack-adjacent abuse**, outside those packs:
   - a client that lies about its version token, or one that logs and
     forges player packets;
   - classic logins that send the password in the clear;
   - the 2003 `ListServer.exe`, which sends registry data and saved
     passwords to a remote host (`AGENTS.md`);
   - `localhost = true` on a public bind (anyone can log in as anyone);
   - a listserver in open mode, or a gserver-registration port that any
     host can use;
   - world scripts that grant items, warps, or staff rights because the
     server trusted the client.

## Technique cards

Each card says what the owner can see, what to do, and who enforces it.
`seihai-servertools` never sees live packets.

### T1 — Timed chat

**See:** fixed gaps between chats, repeated phrases, online time with
almost no movement.

**Do:** cap messages per minute, then mute or kick. NPC rewards for
"being online" require movement or a real interaction. A client timer is
not evidence.

**Where:** [gserver-hooks.md](gserver-hooks.md). This repo can only
document the policy. A log parser for periodic chat is a possible later
tool; it is not built.

### T2 — Gather and prayer macros

**See:** a high interact rate on one tile, one action type for a long
session, many accounts with the same timing.

**Do:** server-side cooldowns and yield caps per account (and, if you
choose, per address). Soften the yield before a ban when you are unsure.

**Where:** gserver and NPC scripts. `lat-core` does not yet lint "this
script has no cooldown". Publishing does warn when extra account files
are still in `accounts/` (see below); that warning is not a bot detector.

### T3 — Nick, body, and shout floods

**See:** nick or body changes in a burst, prop churn with no movement,
shout or to-everyone mode.

**Do:** rate-limit nick, appearance, shout, and to-everyone, and reuse
the T1 chat cap. Mute, put the nick and body back, and treat harassment
as a staff problem.

**Where:** gserver.

### T4 — Version string is not a capability

**See:** the version token says one build, and the bytes behave like
another.

**Do:** do not trust the version string by itself. Prefer the Seihai
secure login (`SEI00001`) on worlds that can require it, and say so to
players. A mismatch disconnect should read as "outdated or modified
client", with a ban only on repeats. Spec:
[Seihai secure login](https://github.com/squirelworks/seihai/blob/main/documentation/secure-login.md).
Classic clients cannot speak it, so a mixed world still accepts classic
logins. That tradeoff is the owner's.

**Where:** `seihai-gserver` and `seihai-listserver` already implement
`SEI00001`. This repo does not set their keys.

### T5 — Forged player packets

A logged client can replay or invent messages. The server has to be the
authority for movement, inventory, chests, and combat. Staff-only
message ids on a normal player session are refused. Unknown ids, huge
frames, and message floods get a budget. Secrets do not live in
client-visible scripts.

**Where:** gserver (and the NPC server for script grants). See
[gserver-hooks.md](gserver-hooks.md).

This repo's share: the publish checks below, and never running classic
`ListServer.exe`. A denylist for LAT uploads is not built.

### T6 — Stolen passwords and a careless publish

Classic login is cleartext. The 2003 listserver binary is a password
stealer. `localhost = true` off loopback, an open listserver, and lab
accounts left in place are the same class of mistake: the world is
reachable and the check is still the lab check.

**Do:** hashed listserver accounts, the registration port closed except
to your own gservers, the secure-login key backed up and its fingerprint
published, lab accounts removed before publish. If that old listserver
binary ever ran on a staff machine, replace every saved Graal password,
not only the server's.

**Where:** listserver for hashes and the future allow-list; this repo
for the publish Blocks below.

### T7 — Hostile world content

A level or NPC can hand out rights when the server trusts the script.
The file name is not proof either way.

**Do:** `seihai-servertools check` before publish and after large
uploads. Read scripts that warp, give items, or mention staff. LAT
uploads keep a diff, refuse when the server copy changed, and log puts
and deletes. Upload rights stay on the LAT accounts that need them.

**Where:** `seihai-servertools check` today covers parse errors in
levels, gmaps and ganis, missing link targets, and gmap holes
([lat-tools](../design/lat-tools.md)). GS1 syntax and
`serveroptions.txt` checks are still on the checker track in
[NEXT.md](../NEXT.md). Live upload conflict checks wait on `seihai-rc`.

## What the setup wizard already does

From `crates/setup-core` ([go-public](../design/go-public.md)). A save is
refused while any finding on that save is a Block.

For a gserver listen address, loopback means the 127. range, `::1`, or
the name `localhost`. The listserver check is narrower: only the bind
`127.0.0.1`, `::1`, or `localhost`. The sentences in the wizard name
`127.0.0.1`.

| Check | Severity | When |
|---|---|---|
| `localhost = true` and the listen address is not loopback | Block | Every gserver save. Anyone could log in as anyone |
| `localhost = false` and `listip` is empty | Block | Every gserver save |
| `upnp = true` and the listen address is loopback | Block | Every gserver save. The router would open a port the server is not using |
| Listen address is loopback | Block | Saving a public profile |
| `staff` still contains `YOURACCOUNT` | Block | Saving a public profile |
| `listip` is still a `<placeholder>` | Block | Saving a public profile |
| Listserver bind is outside its loopback set and `accounts` is empty | Block | Listserver save. Open mode would accept every password |
| Server name still "My Server" | Warn | Saving a public profile. Players see it in the list |
| Extra files in `accounts/` besides `defaultaccount.txt` | Warn | Folder check. Saving is still allowed |
| Listserver gserver port has no allow-list yet | Warn | Listserver save off loopback. Anyone who reaches the port can register and check passwords |
| The accounts path is set and the file is not there | Warn | Listserver save. Nobody can log in until the first account is added |
| `defaultaccount.txt` present | Info | Folder check. New players start from that file |
| `gserver_ip = "peer"` | Info | Listserver save off loopback. A local gserver is advertised as `127.0.0.1`, which other players cannot use |

`setup status` prints the gserver rows for the profile it detects,
plus the folder rows. A gserver save is gated on the gserver rows
alone, so a lab-account warning does not stop the save. The window
uses the same findings. `check_gserver` also has the public rows for
a host-listserver target; the listserver wizard does not pass that
target, and checks the TOML with the listserver rows.

## Order to build the rest

1. Keep the Blocks above. Extending them (louder lab-account warnings,
   leftover staff passwords) is a later change in `setup-core`, with
   tests. It is not this note.
2. Chat and gather limits on `seihai-gserver` (T1–T3).
3. Movement, inventory, and chest authority on `seihai-gserver` (T5).
   That repo's own plan already reserves this work. A new crate here
   would not see the packets.
4. Listserver allow-list for who may register, on top of hashed accounts.
5. Tell players whether the world accepts classic logins, `SEI00001`
   only, or both.
6. Grow `lat-core` checks (GS1, hostile grants) along [NEXT.md](../NEXT.md).
7. Alert on the signals in [gserver-hooks.md](gserver-hooks.md).

## Left out on purpose

Sample hashes, packed-binary notes, decompiler paths, and the archive
inventory stay out of this repository. Client trainers and bots stay
out of the crate map
([salvage-placement](../design/salvage-placement.md)).
