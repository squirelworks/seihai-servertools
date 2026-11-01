# Operator checklist

Tick this before the first public publish, and again after an incident.
Lab checks stay on `127.0.0.1` and the lab ports (gservers 14802 and
14803, listserver registration 14900, clients 14921 to 14923). Addresses
you publish are ones you type (`<listserver-host>`, `<public-host>`);
this repo ships none.

Tags: **[setup]** this repo can already flag it, **[you]** an owner
action, **[gserver]** needs `seihai-gserver`, **[listserver]** needs
`seihai-listserver`.

Threat picture: [threat-overview.md](threat-overview.md).

## A. Before the first public publish

- [ ] **[setup]** Run `seihai-servertools setup status` (or the Server
      setup page). Fix every Block before save.
- [ ] **[setup]** `localhost = false` once the listen address leaves
      loopback. `localhost = true` stays with a loopback
      `serverinterface`: the 127. range, `::1`, or the name `localhost`.
- [ ] **[setup]** Replace `YOURACCOUNT` on the `staff` line with the
      real owner account.
- [ ] **[you]** Remove lab accounts from `accounts/`. The wizard lists
      them as a warning and still allows save, so read the list. Rotate
      any password that appeared in a note or in chat.
- [ ] **[you]** If this world ever had the classic placeholder staff
      login (`super` / `super`), replace that account and password
      before anyone else can connect.
- [ ] **[setup]** A listserver bound to anything other than
      `127.0.0.1`, `::1`, or the name `localhost` uses an accounts
      file. Empty `accounts` is open mode, and the wizard blocks that
      save.
- [ ] **[you]** **[listserver]** Firewall: players need the client ports.
      The gserver registration port (lab default 14900) stays reachable
      only from your own gservers until an allow-list exists. The wizard
      warns that anyone who can reach it can register and check
      passwords.
- [ ] **[you]** Leave UPnP off unless you know which port it will open.
      The wizard blocks UPnP when the listen address is still loopback.
      Prefer a manual forward of the TCP ports you chose.
- [ ] **[you]** Do not run the classic `ListServer.exe` from 2003. It
      sends registry data and saved passwords to a remote host.
- [ ] **[you]** Write down the version policy: classic tokens, Seihai
      secure login only (`SEI00001`), or both. Classic clients cannot
      use the secure login.
- [ ] **[you]** Snapshot the empty economy and account store so a later
      rollback has a clean point.

## B. Staff sessions

- [ ] **[you]** Staff, RC, and NC accounts each have their own password.
      Passwords are typed or taken from the environment, never put on
      the command line.
- [ ] **[you]** RC rights stay small. The upload right is only on LAT
      accounts that upload.
- [ ] **[you]** For Seihai staff logins, pin the server key fingerprint
      from `seihai-gserver` / `seihai-listserver` and keep the key file
      backed up.
- [ ] **[gserver]** Log RC and NC logins (account, address, time) and
      notice ones from outside the staff set.
- [ ] **[listserver]** **[gserver]** Lock out or ban after repeated bad
      passwords.

## C. Bots and macros

These need the game server. This repo cannot apply them.

- [ ] **[gserver]** Chat rate limit and auto-mute.
- [ ] **[gserver]** **[you]** Online-time rewards require movement or a
      real interaction, not chat alone. Put that in the NPC scripts.
- [ ] **[gserver]** Cooldowns on water, dig, mine, pray, and gather loops.
- [ ] **[gserver]** Hourly or daily yield caps per account.
- [ ] **[gserver]** Rate limits on nick and appearance changes, and on
      shout / to-everyone.
- [ ] **[you]** Once a week, read logs for chat every few seconds and
      for one tile taking a stream of interacts. Suggested lines are in
      [gserver-hooks.md](gserver-hooks.md).

## D. Forged packets

- [ ] **[gserver]** The server enforces max speed, collision, and level
      bounds.
- [ ] **[gserver]** Chests, items, and combat damage follow the server's
      counts.
- [ ] **[gserver]** A normal player session that sends staff, RC, or NC
      message ids is dropped.
- [ ] **[gserver]** Unknown message ids, oversized frames, and messages
      per second have a budget.
- [ ] **[gserver]** Log the version token next to what the client does.
      Kick on mismatch only if the world said it would.
- [ ] **[you]** World scripts do not hide a reward that only the client
      can see. Warps and gives are checked server-side.

## E. Levels and LAT

- [ ] **[setup]** Run `seihai-servertools check WORLD` before publish
      and after a large upload. Today that is parse errors (levels,
      gmaps, ganis), missing link targets, and gmap holes.
- [ ] **[you]** Read levels and NPCs that warp freely, give items, or
      look like a staff grant. The file name is only a hint.
- [ ] **[you]** LAT uploads: keep the diff, keep a local copy of the
      file you replaced, and skip the upload if the server copy changed.
      That conflict check is specified in
      [lat-tools](../design/lat-tools.md) and is not built yet.
- [ ] **[you]** Production uploads come from named staff accounts, and
      puts and deletes are kept.

## F. The machine

- [ ] **[you]** Admin login (SSH or RDP) is not on the game ports. Prefer
      key-only SSH.
- [ ] **[you]** Classic game TCP has no TLS. Use the secure login and
      the firewall in section A instead.
- [ ] **[you]** Back up `accounts`, levels, and the NPC database on a
      schedule, and restore once onto a spare folder to prove it.
- [ ] **[you]** Keep the host clock synced so log lines can be compared.

## G. Quiet checks on loopback

Run these on a world you started yourself. Do not point them at a public
server.

- [ ] **[gserver]** A test account sends chat faster than the cap and is
      muted or kicked, once that cap exists.
- [ ] **[gserver]** Holding interact on one tile hits the cooldown or
      the yield cap.
- [ ] **[gserver]** A position past the level, or faster than the
      configured max, is put back.
- [ ] **[setup]** `setup status` reports a Block when `localhost = true`
      and the listen address is outside the 127. range (and is not
      `::1` or the name `localhost`).

## H. When something is wrong

1. Copy the logs, the account row, and a recent inventory snapshot.
   Leave passwords out of the copy.
2. Kick, mute, or ban. If several accounts share an address, throttle
   that address.
3. If items may have been duplicated, stop economy scripts and diff
   against the last backup.
4. If passwords may have leaked, replace staff and listserver secrets.
   If classic `ListServer.exe` ran on a staff PC, replace saved client
   passwords too.
5. Write a short note in `docs/lab/` or in the world's own ops notes:
   what you saw, what you changed, no passwords, no public hostnames.

## I. What to ask the stack for

| Priority | Ask |
|---|---|
| Now | Keep using the setup Blocks. Do not publish while one remains |
| Now | Hashed listserver accounts, and the registration port closed to strangers |
| Next | Chat and gather limits on `seihai-gserver` |
| Next | Server-side movement and items |
| Later | Version-mismatch handling you have documented, and secure login where classic clients can be turned away |
| Later | Richer `seihai-servertools check` (GS1, script grants) |
| Later | The same signals surfaced in the window, after the gserver actually emits them |
