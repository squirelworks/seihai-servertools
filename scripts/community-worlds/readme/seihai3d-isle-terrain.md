# Seihai 3D test world: Isle (streamed terrain)

A 4 x 4-level island playerworld with its 3D side: hills, beaches and water
from one height field, as a 2D gmap and as a streamed 256 x 256 terrain
with four textures, trees and rocks, a sun, and the spawn on flat grass.

## What it demonstrates

- `seihai-servertools gen terrain ... --3d --stream --playerworld`: a 2D
  playerworld and its 3D world from one height field, ready to serve.
- Streamed terrain (protocol 24): 256 chunks in the client's own encoding,
  `heights.info` with `.png` names, terrain pushed at login so a 5.12's
  first visit is textured, not white.
- The same world for both servers.

## Contents

| Folder | For |
|---|---|
| `seihai-gserver/` | `seihai-gserver --world seihai-gserver` |
| `gserver-v2/` | seihai-gserver-v2 with the Graal 3D add-on: copy `config/`, `world/` into your server folder and add the lines of `config/foldersconfig-3d.txt` to `config/foldersconfig.txt` |

## Run it

On seihai-gserver (0.2.0 or a current `main`):

```sh
seihai-gserver --world seihai-gserver
```

Settings already in `seihai-gserver/config/server.txt`: `name = 3 isle`,
`graal3d = true`, `graal3d_pushterrain = true`, and the start level under the
3D spawn.

On seihai-gserver-v2 with the add-on (4.0.0-seihai.6, or the add-on package
`seihai-gserver-v2-graal3d-addon`), in `config/serveroptions.txt`:

```text
graal3d = true
graal3dpushterrain = true
```

and `hq_level = 4` in `config/adminconfig.txt` so the listserver tags it `3 `.

## Known limits

- The terrain texture fades to its first material within about 70 units of
  the camera; that is the 5.12's own behaviour.
- 4.20r8 draws this terrain unshaded and grows no ground cover.
