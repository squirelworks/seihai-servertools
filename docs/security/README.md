# Security notes

Operator notes for a Seihai-stack world: what to check before anyone else
can connect, and which repo has to enforce it.

These pages are behaviour and checklists. They contain no exploits, no
sample binaries, and no decompiler output. The rules in
[AGENTS.md](../../AGENTS.md) still apply: loopback and placeholders only,
and nothing from the GPL fork is copied here.

| Page | Read it when |
|---|---|
| [threat-overview.md](threat-overview.md) | You want the abuse picture and where each control lives |
| [operator-checklist.md](operator-checklist.md) | You are about to publish, or you are answering an incident |
| [gserver-hooks.md](gserver-hooks.md) | You work on `seihai-gserver` and need the rate-limit and authority list |
| [../design/salvage-placement.md](../design/salvage-placement.md) | You are deciding whether a recovered tool belongs in this repo |

## What this repo can do

`seihai-servertools` reads and writes the owner's config and checks a world
folder on disk. `crates/setup-core` refuses unsafe publish combinations
([go-public](../design/go-public.md)). `crates/lat-core` reports levels,
gmaps and ganis that do not parse, broken links, and gmap holes.

Passwords come from an environment variable or a prompt. A test connection
opens one TCP socket to the address the owner typed. There is no telemetry
and no built-in list of servers.

## What this repo does not enforce

Chat caps, gather cooldowns, movement and inventory authority, and
unknown-message budgets run inside the game server. They are listed in
[gserver-hooks.md](gserver-hooks.md) for
[seihai-gserver](https://github.com/squirelworks/seihai-gserver). Putting
them in a new crate here would not see the packets.

The Rust listserver
([seihai-listserver](https://github.com/squirelworks/seihai-listserver))
owns account hashes and, later, the gserver allow-list. The GPL fork stays
in its own repo.

Written 2026-11-01 from a static review dated 2026-10-31. No client was run
for this note.
