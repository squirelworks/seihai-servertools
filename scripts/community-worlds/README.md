# Community 3D test worlds

`build.sh` builds the 3D test worlds we use in the lab as one tarball each,
for anyone running seihai-gserver (or seihai-gserver-v2 with the Graal 3D
add-on) to try:

| Tarball | World | Server |
|---|---|---|
| `seihai3d-isle-terrain-<date>.tar.gz` | a 4 x 4-level island with streamed terrain (`gen terrain --3d --stream --playerworld`) | seihai-gserver, and a gserver-v2 layout |
| `seihai3d-regions-overworld-<date>.tar.gz` | the 8,192-unit, 8 x 8-region overworld with 6 towns, 18,055 objects (`seihai-regions demo`) | seihai-gserver (`regions`, `graal3d_scope = on`) |
| `seihai3d-scoping-grid-<date>.tar.gz` | 20,164 cubes on a flat streamed terrain (distance scoping) | seihai-gserver (`graal3d_scope = on`) |
| `seihai3d-interiors-sample-<date>.tar.gz` | a small original playerworld with a house and its interior in one world, joined by `portal` doors (`seihai-pw3d`) | seihai-gserver (`portal`, `graal3d_scope = on`) |
| `seihai3d-conformance-worlds-<date>.tar.gz` | the conformance suite's 40 redistributable cases, one server folder each | seihai-gserver |

```sh
scripts/community-worlds/build.sh OUT --graal-analysis ../Graal_analysis --date 2026-11-27
```

Each tarball holds the world, a `README.md` (what it shows, how to run it,
the settings it needs, the expected client, known limits), `LICENCE.md`,
`BUILD-INFO.txt` (the commits it was built from) and `SHA256SUMS` over its
files; `OUT` also gets a `.sha256` per tarball and one `SHA256SUMS`. The
tarballs are reproducible: the same commits and `--date` give the same
checksums.

Everything in them is generated from nothing by our own tools. Left out on
purpose: the conformance cases that need a held third-party content pack
(C016, C017), lab-injector and gserver-v2-runner cases, CC BY 3.0 `.dif`
interiors from other projects, client files, lab captures and logs.

`worlds.py` holds what no servertool writes itself: the interiors sample's
2D levels, the scoping grid, the conformance export (both from
Graal_analysis `tools/conformance/`), and `config/server.txt` for worlds
whose tool writes none. `readme/` holds the package READMEs and
`LICENCE-WORLDS.md` the licence note.
