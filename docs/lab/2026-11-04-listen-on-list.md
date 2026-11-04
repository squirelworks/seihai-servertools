# "Listen on" pick lists, 2026-11-04

What: the window's Server setup and Host a listserver pages have a Pick
menu next to "Listen on", filled from `setup_core::net::interfaces` (the
local interface table, read with `if-addrs`). The command line prints the
same list before its Listen on questions.

## Setup

- Linux container, Xvfb at 1100x800, software GL, XTest clicks.
- Interfaces: `lo` (127.0.0.1) and one `eth0` IPv4 address. No IPv6
  addresses were up apart from link-local ones, which the list leaves out.
- World folder with one lab server, `servers/default`.

## Result

1. Server setup, Join a listserver: Listen on showed `AUTO` and a Pick
   menu with `AUTO (every interface)`, the eth0 address `(eth0)` and
   `127.0.0.1 (lo, this machine only)`, then "Read the addresses again".
   Picking `127.0.0.1` put it in the field.
2. Host a listserver: Listen on showed `0.0.0.0`, and Pick offered
   `0.0.0.0 (every IPv4 interface)`, `:: (every IPv6 interface)`, the eth0
   address and `127.0.0.1 (lo, this machine only)`.
3. Command line, `setup public` by prompts: before "Listen on: AUTO ...",
   it printed "This machine's addresses:" and the eth0 and loopback lines.
   The `setup_public_by_prompts_then_lab_then_undo` test checks for the
   loopback line.

## Not covered

- Windows and macOS: the CI runs `net::tests` there (it needs a loopback
  address in the list); the pages were not opened on those systems.
- The folder picker (NEXT.md, window polish part 2).
