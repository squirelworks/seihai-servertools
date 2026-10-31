# Next steps

Work in this order, one branch and pull request each. Tick the box in the
same PR. Rules: `AGENTS.md`. Layout: `docs/ARCHITECTURE.md`.

## Done

- [x] Scaffold: workspace, licences, `AGENTS.md`, docs, CI, release workflow.
- [x] One-step lab start: `seihai-servertools lab start|plan`, `scripts/lab/`
      (`docs/lab/2026-10-31-lab-start.md`).
- [x] World checker, first checks: `seihai-servertools check`.

## Setup track (go-public), `docs/design/go-public.md`

Squirrel asked for all three parts in the first version (2026-10-31): the
window wizard, the `setup` command line, and Host a listserver.

- [x] **Step 1** `config::options_txt` writer (`set`, `remove`, the
      `# Changed by Seihai server setup, YYYY-MM-DD` comment) and
      `config::backup` (dated backup, temp + rename, undo). Round-trip tests:
      comments, unknown keys, CRLF and order survive.
- [x] **Step 2** `profile` (detect Lab / Public, Back to lab in the
      `GSERVER_KEYS` order, `localhost = true` last) and `checks` (every
      safety-check rule and refused combination, one test each).
- [x] **Step 3** `seihai-servertools setup --public | --lab` on the command
      line: the wizard's steps as prompts, `--yes` plus options for scripts,
      a before/after diff before saving.
- [ ] **Step 4** Start and check: run the server with `run`, tail its log,
      the three lights (listening on, listserver registered, reachable from
      outside: cannot be proven behind NAT, say so). Loopback e2e test with
      `seihai-listserver` on `127.0.0.1` and `127.0.0.2` as the "outside"
      interface on Linux.
- [ ] **Step 5** Window: the Join a listserver page (same steps, same core
      calls, no logic in the page).
- [ ] **Step 6** Host a listserver: `config::listserver_toml` editor
      (format-preserving), the wizard on the command line
      (`setup --host-listserver`) and in the window; accounts file required,
      `add-account` through the listserver binary. Until `seihai-listserver`
      has its gserver allow-list, warn and tell the owner to keep the gserver
      port closed to the outside.

## Lab track

- [ ] Lab page in the window: start and stop the plan on a worker thread,
      show each console's tail.
- [ ] Rust gserver: a gserver kind in `LabOptions` once `seihai-gserver` can
      run a world folder.
- [ ] Lab run with a real `gs2emu` from a fork release (today's run used a
      stand-in; `docs/lab/2026-10-31-lab-start.md`).

## LAT track, `docs/design/lat-tools.md`

- [ ] **Shared drawing and level document** (in `squirelworks/seihai`, ask
      Squirrel first): move tile/sprite drawing out of `seihai-mae` and the
      `Document` out of `tools/level-editor` into a library crate, keep the
      client and its offline editor on it, tag a release, then pin the tag
      here.
- [ ] Level editor on a local folder (`LocalFolder`), using the shared crate.
- [ ] `RcFiles`: `RemoteFiles` over `seihai-rc`'s file browser, once
      `seihai-rc` stage 4 is merged and tagged. Fake file browser in tests,
      lab run against the fork with a LAT account.
- [ ] Level editor on a live server: download, edit, diff, refuse when the
      server's copy changed, upload.
- [ ] NPC editor: level NPCs with a GS1 syntax check; NC objects after
      `seihai-rc` stage 5.
- [ ] Gani editor with preview.

## Checker track

- [ ] Missing images and sounds named by levels, NPCs and ganis.
- [ ] GS1 syntax errors in level NPCs.
- [ ] `serveroptions.txt` checks from `setup_core::checks` (after setup step 2).
- [ ] `check --json`.
