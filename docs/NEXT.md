# Next steps

Work in this order, one branch and pull request each. Tick the box in the
same PR. Rules: `AGENTS.md`. Layout: `docs/ARCHITECTURE.md`.

## Handed to the team (Squirrel, 2026-10-31)

Squirrel left these for the other agents (Cursor, Grok, Chiaki); the Claude
session that built the setup did not do them:

- [ ] **Real-fork lab run** of the whole setup: build `seihai-gserver-v2`,
      run it with `seihai-listserver` through `lab start`, take `default`
      public with `setup public` (127.0.0.2 as the outside interface on
      Linux), check `setup start` shows the gserver listening and linked, and
      that a Seihai client gets it in the server list. Steps: "Step 4 lab
      run" below and `docs/lab/HOWTO.md`. So far the gserver side was only a
      stand-in script (`docs/lab/2026-10-31-lab-start.md`,
      `docs/lab/2026-10-31-host-listserver.md`).

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
- [x] **Step 4** Start and check: `setup start` runs the server with `run`,
      reads its console and shows the three lights (`setup_core::lights`;
      "reachable from outside" always says it cannot be proven from here);
      `setup test-connection HOST[:PORT]` (`setup_core::net`). Tested with a
      stand-in gserver that prints the fork's console lines.
- [ ] **Step 4 lab run** (needs a built fork; handed to the team, see the top): `seihai-listserver` on
      `127.0.0.1`, the fork's `default` folder taken public with
      `setup public --interface 127.0.0.2 --listserver 127.0.0.1` on Linux,
      `setup start` shows listening on 127.0.0.2 and the listserver linked,
      a Seihai client gets the server in the list. Write it up in
      `docs/lab/`. If the fork prints its lines differently, fix
      `lights.rs` and its table.
- [x] **Step 5** Window: the Server setup page (`crates/app/src/pages/setup.rs`):
      the seven steps on one page, Test connection on a worker thread,
      Review Join a listserver / Back to lab with the diff and findings,
      Save only when nothing blocks, Undo, Start and check with the lights.
      All through `setup_core::folder::ServerFolder`, which the command line
      uses too. `seihai-servertools gui --world DIR --page setup` opens it.
- [ ] Window polish: a list of this machine's interfaces for "Listen on"
      (`net::interfaces`, planned), a folder picker, and a test that drives
      the page (egui_kittest or a state-only test of `Form::answers`).
- [x] **Step 6** Host a listserver: `config::listserver_toml::ListserverToml`
      (on `toml_edit`: comments, order and unknown keys kept),
      `folder::ListserverConfig`, `setup host-listserver | listserver-lab |
      listserver-undo` on the command line, the Host a listserver half of the
      Server setup page, and the first account through
      `seihai-listserver add-account` (password only in the child's
      environment). Lab run: `docs/lab/2026-10-31-host-listserver.md`.
- [ ] When `seihai-listserver` gets its gserver allow-list and HQ password
      (its `docs/NEXT.md`), add the "Who may register" step's fields and
      drop the warning.

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
