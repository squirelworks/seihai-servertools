# Seihai 3D test world: Scoping grid (20,164 objects)

142 x 142 = 20,164 red cubes, 56 units apart, on a flat streamed terrain
(square size 32, so the grid spans about 8,000 units), with a six-colour sky
box and a sun. About five times what a 5.12 can hold at once.

## What it demonstrates

- Distance scoping (`graal3d_scope`): with a radius of 1,100 units a client
  holds about 1,400 of the 20,164 objects, and the nearest come in first, so
  the camera and terrain are never crowded out of the ghost table.
- Without scoping, the same world fills the 4,096-slot table and a 5.12's
  view stays black for the whole session (Graal_analysis T3D-166). Turn
  `graal3d_scope` off to see that for yourself.

## Run it

On seihai-gserver (0.2.0 or a current `main`):

```sh
seihai-gserver --world .
```

Required settings, already in `config/server.txt`:

```text
name = 3 Seihai Scoping Grid
graal3d = true
graal3d_scope = on
graal3d_scoperadius = 1100
graal3d_pushterrain = true
```

seihai-gserver-v2 with the add-on also has distance scoping
(`graal3dscope = on` in `config/serveroptions.txt`); put `files/` and
`conf_ter/` in `world/3d/` there. Not checked on v2 with this package.

## What a lab 5.12 showed (Graal_analysis `torque/3d-big-world-streaming-512-2026-11-27.md`)

| Radius | Peak ghosts | Join | Walking |
|---|---|---|---|
| 1,100 | 1,417 | 3 to 5 s | about 40 creates and 40 deletes a second, 2.5 KB/s |
| 1,500 | 2,533 | 3 to 5 s | similar |

Nothing visibly pops in: the 5.12 draws nothing past about 1,000 units.

## Known limits

- There is no walker script: the player stands at the spawn unless a client
  script or the server moves it.
