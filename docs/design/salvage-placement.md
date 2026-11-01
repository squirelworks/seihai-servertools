# Where a recovered tool goes

A short decision tree for anyone holding an old Graal helper, editor, or
archive. The crate map is [ARCHITECTURE.md](../ARCHITECTURE.md).
Operator abuse notes are [security/README.md](../security/README.md).
This page does not list recovered archives.

Ask in order. Stop at the first yes.

```
Recovered artifact or behaviour note
│
├─ Raw binary, installer, archive, decompiler database,
│  memory dump, or disassembly?
│     → keep it out of this repo
│
├─ Client trainer, bot, auto-key, digger, AFK helper,
│  packet logger, or modified client?
│     → out of scope (do not port it)
│
├─ Offline level editor, tile or sprite drawing, or a format parser?
│     → squirelworks/seihai, then pin that git tag here
│
├─ RC or NC session, or the live file link?
│     → squirelworks/seihai-rc (pin by tag).
│       lat-core only holds the RemoteFiles trait
│
├─ Game server, listserver, or NPC-server runtime,
│  including rate limits and packet authority?
│     → seihai-gserver, seihai-listserver, or seihai-npcserver
│       (the GPL fork stays in seihai-gserver-v2)
│
├─ Lab start, process group, or world-folder layout?
│     → crates/setup-core (lab, run) and scripts/lab/
│
├─ serveroptions.txt, listserver TOML, go-public, backups,
│  status lights, or test-connection?
│     → crates/setup-core
│
├─ Live level, NPC, or gani editing, once shared drawing
│  and the RC file link exist?
│     → crates/lat-core, then a page in crates/app
│
├─ World or config checks (links, gmap holes, options safety)?
│     → crates/lat-core check, the check command, the Check page
│
└─ Unsure?
      → do not add a crate. Leave the artifact out of git
```

## Never commit

- Executables, libraries, installers, and archives of recovered tools.
- Decompiler projects, memory dumps, decompiled source, and binary
  addresses.
- GPL code from GServer-v2, gs2lib, or gs2-parser, including a
  "reference copy".
- A second parser for levels, gmaps, ganis, or GS1. Those come from
  `seihai-core` by tag.

Behaviour that is worth keeping is rewritten in this repo's own words,
in `setup-core` or `lat-core`, with tests. The command line prints. The
window holds state. Config edits keep comments, order, and unknown
keys. Hosts in docs and tests are `127.0.0.1`, the lab ports, or
`<listserver-host>` / `<public-host>`.

Do not run classic `ListServer.exe`. It sends registry data and saved
passwords to a remote host.
