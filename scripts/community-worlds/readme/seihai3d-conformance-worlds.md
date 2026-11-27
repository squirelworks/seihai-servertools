# Seihai 3D test worlds: the conformance cases

The small worlds of the Graal 3D client conformance suite (Graal_analysis
`torque/conformance/`), one server folder each. Each case exercises one
thing the 3D capability board found: the `3 ` tag, the sky box, a red cube,
textures, client scripts (`weapon3d` probes), streamed terrain, water,
grass and ground cover, two players, broken world lines, object density,
cache refetch, reconnects, and more.

`CASES.md` lists every folder with what it checks, and the cases left out.
`EXPECTED.md` is the suite's expected result for each case on each build
(3.153, 4.04r2, 4.20r8, 4.20r6L, 5.12, 6.037), with the source of each.

## What it is for

- Server authors: run the same worlds against your own server and compare
  with `EXPECTED.md`.
- World builders: the smallest working example of each world line.
- Client authors: what a real 5.12 does with each.

## Run one

On seihai-gserver (0.2.0 or a current `main`):

```sh
seihai-gserver --world c005-cube-red
```

Each folder's `config/server.txt` has the settings that case needs. They
keep `graal3d_trace = g3d.tsv`, a lab trace of every 3D packet that the
suite's checks read; delete that line if you do not want the file.
Cases with probe scripts (`conf/*.gs2`) report through the server log
(`--verbose`): look for `ConfProbe` lines.

The suite's own runner (`tools/conformance/run.py` in Graal_analysis) drives
real clients in lab VMs and judges each run; these folders are what it
serves.

## Cases to know about

- C002 is named without `3 ` on purpose: the client should stay 2D.
- C015 expects you to stop the server half way through the visit.
- C028 and C029 are visited more than once (see `sessions` in CASES.md);
  C028 swaps `conf_red.3ds` for `conf_red_v2.3ds` between visits.
- C020 needs two clients.
- C042 ends with a 5.12 killing itself (self-mount plus `move3d`): that is
  the finding it records.
- C041 is for 4.20r8 under GDI OpenGL only.
