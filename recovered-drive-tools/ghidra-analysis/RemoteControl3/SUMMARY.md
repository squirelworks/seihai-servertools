# Salvage analysis: RemoteControl3

- **Binary:** `Developer Tools/# Graal Client Tools And Sources/RC [Remote Control]/windows_rc3_beta.rar.extracted/RemoteControl3.exe`
- **Also same PE in:** Main Client `v5/Graalv5.rar.extracted/…/RemoteControl3.exe`
- **Analyzed (UTC):** 2026-10-31 · Ghidra 12.2 · static only
- **Format:** PE32 GUI (x86), Windows toolchain, ~2.2 MB, heavily sectioned (16)

## Purpose (plain behaviour)

Staff **Remote Control** client for classic Graal servers. UI is **GTK** on Windows (gdk/glib/pixbuf imports) rather than Delphi.

Observed behaviour from symbols and strings (no offsets):

- Shows a server list, a version field, a password field that need not be saved, and a connect action.
- Talks to a listserver or server-pick path and to a separate NPC server, with a label once that NPC server is connected.
- After connect: a player list (refresh, mass private message, admin message), an NPC list and local NPCs, a level-NPC dump, folder and config pages, and profile, rights, account, and access editors.
- Staff actions: ban with time and reason, warp to a player, warp players, disconnect, update level, away mode when the player list closes, and an option to keep NPC-control chat separate from remote-control chat.
- Embeds bzip2 and zlib compress helpers and password encrypt/decrypt helpers. Treat the wire encoding as opaque for clean-room notes.
- Syntax highlighters for Graal script, ini, Pascal, and C++ — an in-client script editing surface.

**Do not run** this PE against public hosts; lab loopback only if ever needed.

## Recommended placement

| Tag | `seihai-rc` |
| --- | --- |
| Why | Live RC/NC session, staff file/NPC/player tooling over the wire. |
| Also | Raw PE + Ghidra dumps stay staged here (`analysis-only` artifacts). |
| Not | `setup-core`, `lat-core` (until shared edit crates exist), MIT paste of decompile |
| Licence | Proprietary/legacy client — clean-room protocol notes only |

## Ghidra stats (machine)

| Field | Value |
| --- | --- |
| Functions | ~3653 (internal ~3562) |
| Imports | ~529 (Win32 + GTK stack + Winsock/IPHLPAPI) |
| Interesting strings | ~405 |
| Decompiled (local only) | 25 — **not committed** |

Named areas, without addresses: sorting and init, listserver connect, NPC-server connect, connect to the selected server, password encrypt and decrypt, socket bind and connect, compression handling, and window builders for the player list, profile, rights, account, options, and access.

## Artifacts (local, gitignored except this SUMMARY)

`prelim/`, `functions.txt`, `imports.txt`, `exports.txt`, `strings_interesting.txt`, `decompiled/`, `logs/`, `summary_stats.txt`
