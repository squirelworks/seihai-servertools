# Lab runbooks

Prepared steps for the servertools items that need a real lab. Each runbook
says what to run, what the output should be, and where to write the result.
They were written without a lab: the expected values come from the code, the
stand-in runs in this folder and the issue text, not from a real run.

Labs run on the team's own machines on `127.0.0.1` (`AGENTS.md`). Write each
run up next to this folder as `docs/lab/YYYY-MM-DD-<what>.md` (see
`../HOWTO.md`), with no keys, script bodies or real hosts.

| Runbook | Issue | Needs |
|---|---|---|
| [47-real-fork.md](47-real-fork.md) | #47 | Linux, a seihai-gserver-v2 release, `seihai-listserver`, Wine and a real 1.41 rev 1 |
| [37-script-cache.md](37-script-cache.md) | #37 | A real 5.24r4 to 6.x client and a way to read packet 140 |
| [38-client-reach.md](38-client-reach.md) | #38 | Real old clients; the GS1/GS2 cut-off decision first |

Helpers, both POSIX `sh`, read only:

- `scripts/lab/check-consoles.sh WORLD [SERVER...]`: expected-vs-actual
  rows for the listserver and gserver consoles. Exits 1 on any FAIL.
- `scripts/lab/code-key.sh WORD1 WORD2`: the two packet 140/197 key words
  as the `--key HEX16` that `code` takes.

`fixtures/` holds three made-up worlds (`good`, `bad`, `public`) with the
console lines `lights.rs` reads. Run the console check on them first to see
what PASS and FAIL look like:

```sh
scripts/lab/check-consoles.sh docs/lab/runbooks/fixtures/good       # all PASS
scripts/lab/check-consoles.sh docs/lab/runbooks/fixtures/bad        # 5 FAIL, exit 1
scripts/lab/check-consoles.sh --addr 127.0.0.2 docs/lab/runbooks/fixtures/public default
```

The fixture lines are the stand-in's, not captured from the real fork. If
the real fork prints them differently, the console check and `lights.rs`
both need the fix (#47 step 3).
