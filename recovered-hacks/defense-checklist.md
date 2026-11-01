# Operator defense checklist (Seihai / classic Graal worlds)

Companion to [`DEFENSE_PROPOSALS.md`](DEFENSE_PROPOSALS.md) and [`THREAT_TAXONOMY.md`](THREAT_TAXONOMY.md). Tick before or right after going public. Items marked **[tools]** can grow into `seihai-servertools` checks/docs; **[gserver]** / **[listserver]** need runtime support.

Threat-intel confidence (second pass): **high** for Groups 1–3/5; **unknown** for Group 4 Miner (Drive-blocked); **medium** for stack-adjacent version faker / packet logger.

---

## A. Before first public publish

- [ ] Run `seihai-servertools setup status` / go-public wizard; fix every **Block** finding.
- [ ] Confirm `localhost=false` and `serverinterface` is not loopback. **[tools]**
- [ ] Replace `YOURACCOUNT` in `staff` with the real owner account. **[tools]**
- [ ] Remove lab/test accounts from `accounts/`; rotate any password that ever appeared in docs or chat. **[tools]**
- [ ] Ensure listserver uses an **accounts file** (hashed), not open mode, when bound outside loopback. **[tools]**
- [ ] Firewall: client ports open; **gserver registration port** reachable only from your gservers until allow-list exists. **[listserver]**
- [ ] No UPnP unless you understand it; prefer manual port forward. **[tools]**
- [ ] Do **not** run classic third-party `ListServer.exe` tools (known password stealer in 2003 “Rick” build). **[T6]**
- [ ] Decide version policy: allow classic tokens, Seihai-only (`SEI00001`), or mixed. Document for players. **[T4]**
- [ ] Snapshot empty economy / account DB for later rollback.

## B. Auth, sessions, staff

- [ ] Staff/RC/NC accounts: unique strong passwords; never default `super`/`super`.
- [ ] RC rights least-privilege; upload (`w`) only for LAT roles that need it.
- [ ] Prefer Seihai secure login for staff connections when available; pin server key fingerprint.
- [ ] Log and alert on RC/NC logins (account, IP, time). **[gserver]**
- [ ] Ban / lockout after repeated failed passwords. **[listserver]** / **[gserver]**

## C. Anti-bot / anti-macro (Groups 1–3)

- [ ] Chat rate limit + auto-mute. **[gserver]** **[T1 / Group 1]**
- [ ] Online-time / AFK rewards require movement or real interactions — not chat alone. **[gserver]** / scripts
- [ ] Server-side cooldowns on water/dig/mine/pray/gather loops. **[gserver]** / NPC server **[T2 / Group 2]**
- [ ] Hourly/daily yield caps per account (and optionally per IP). **[gserver]**
- [ ] Nick / appearance change rate limits. **[gserver]** **[T3 / Group 3]**
- [ ] Review logs weekly for periodic chat (≈3s / letter-scroll) and single-tile interact storms.

## D. Anti-cheat / anti-forge (T5)

- [ ] Movement: max speed, collision, level bounds enforced server-side. **[gserver]**
- [ ] Chests, items, combat damage authoritative on server. **[gserver]**
- [ ] Drop connections that send staff/RC/NC message ids on a normal player session. **[gserver]**
- [ ] Cap unknown message ids / oversized frames / messages per second. **[gserver]**
- [ ] Do not trust client version string alone; log token + behaviour mismatch. **[gserver]**
- [ ] World scripts: no secret rewards only the client can see; validate warps and gives. **[LAT]**

## E. Content & LAT (T7)

- [ ] Run `seihai-servertools check WORLD` before publish and after big uploads. **[tools]**
- [ ] Review levels/NPCs whose names or scripts suggest staff escalation (`hacked*`, free items, unrestricted warps).
- [ ] LAT uploads: diff + conflict check; keep local backups of replaced files. **[tools]** / **[rc]**
- [ ] Production puts only from named staff accounts; retain put/delete logs.

## F. Network & host

- [ ] Separate admin SSH/RDP from game ports; key-only SSH where possible.
- [ ] TLS termination is N/A for classic Graal TCP — compensate with secure login + firewall.
- [ ] Backups of `accounts`, levels, and NPC DB on a schedule; test restore once.
- [ ] Host clock synced (log correlation).

## G. Detection quick tests (lab or quiet hours)

- [ ] From a test account, flood chat → expect mute/kick (once limits exist).
- [ ] Sustained interact on one tile → expect cooldown/cap.
- [ ] Impossible speed / out-of-level position → expect reject (once authority exists).
- [ ] Login with `localhost` misconfig on a staging bind → wizard/check must **Block**.

## H. Incident response (short)

1. Snapshot logs + account row + recent inventory.
2. Kick/mute/ban; throttle IP if multi-account.
3. Freeze economy actions if item dupe suspected; diff vs last backup.
4. Rotate staff and listserver secrets if credential theft possible.
5. File a short postmortem in `docs/lab/` or world ops notes (no passwords).

## I. Patch priorities (owners → stack maintainers)

| Priority | Ask |
|---|---|
| P0 | Keep using `seihai-servertools` safety checks; never publish with Blocks |
| P0 | Firewall listserver gserver port; hashed accounts |
| P1 | Chat + gather rate limits / cooldowns on gserver (Groups 1–2) |
| P1 | Server-authoritative movement and items (T5) |
| P2 | Version/behaviour mismatch kicks; secure login option |
| P2 | `lat-core` script lints + richer `check` (T7) |
| P3 | Automated log queries / “lights” for abuse signals in the GUI |

---

*Static analysis only. Do not execute recovered trainers or patched clients against public servers. Do not commit hack binaries into seihai-servertools.*
