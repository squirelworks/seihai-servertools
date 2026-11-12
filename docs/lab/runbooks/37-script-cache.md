# Runbook: the rest of the real script-cache check (#37)

The Windows 6.113 check on 2026-11-12 (`docs/design/code-caches.md`) settled
the key layout, the clear tail, the 197 CRC (stored body) and that bodies are
GS2 bytecode, for weapon scripts from cache hits. Still unchecked:

1. **Packet 140**, a script download rather than a cache hit.
2. **Class caches**, `classes/<name>.code`.
3. **Other builds** in the GRESCR01 range, 5.24r4 to 6.03r7.

Use the same private lab method as that run (an unmodified client, memory
reads of its receive buffer only, no injection and no packets sent). Keep
keys, bodies and hosts out of the repo.

## Force a 140

Delete the script's `.code` file from the client's `weblevels/<host>/`
folder, or change the weapon on your lab server, then log in again. The
client should get 140 (the script) and write a fresh `.code`.

## Check each captured script

From the 140 or 197 header, take the two key values (two GINT5) as decimal
numbers and the CRC.

```sh
KEY=$(scripts/lab/code-key.sh WORD1 WORD2)
seihai-servertools code show weapons/<name>.code --key "$KEY"
seihai-servertools code crc  weapons/<name>.code                  # stored (default)
seihai-servertools code crc  weapons/<name>.code --domain plain --key "$KEY"
```

| Check | Expected (from the 6.113 run) |
|---|---|
| `code show` decrypts | yes; body starts `00000001 00000004` |
| `code crc` (stored) | equals the header's CRC |
| `code crc --domain plain` | does not equal it |
| 140 key | same layout as 197 |
| class cache | same as weapons, magic GRESCR01 |

One row per script in the run note: build, packet (140 or 197),
weapon or class, each check's pass or fail. A different result on any build
is the interesting one; say which.

`code-key.sh 1 2` prints `0100000002000000`, so a key read in the wrong order
shows up as a failed decrypt, not a wrong CRC.
