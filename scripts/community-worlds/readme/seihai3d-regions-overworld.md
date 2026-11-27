# Seihai 3D test world: Regions overworld (8,192 units, 6 towns)

A big 3D overworld built from an 8 x 8 grid of regions of 1,024 units each
(8,192 units a side, 4,096 terrain chunks): hills, open fields, 9,415 trees
and rocks and six dense towns of 8,640 houses in all. 18,055 objects.

## What it demonstrates

- seihai-gserver's `regions` world line: the server stitches the regions
  near the player into one sliding 256 x 256 terrain window, so a region
  edge has no seam, stall or world switch.
- Distance scoping: only objects within 1,100 units of a player are sent
  (at most 3,584, nearest first), so a world far past the 5.12's 4,096
  ghosts plays.
- `seihai-regions demo` from seihai-servertools; `seihai-regions stats
  config/graal3dworld.txt --root .` prints the density per region (the
  densest spot is about 3,100 objects within reach).

## Run it

seihai-gserver only (seihai-gserver-v2 has no `regions` line yet). Needs
seihai-gserver 0.2.0 or a `main` with #230.

```sh
seihai-gserver --world .
```

Required settings, already in `config/server.txt`:

```text
name = 3 Seihai Regions
graal3d = true
graal3d_scope = on            # required: the world is far past the ghost table
graal3d_pushterrain = true
```

Optional: `graal3d_sendms = 10` made joins about 3 times faster on loopback
in the lab (2.4 s to phase 2 next to a town), but has not been tried on a
real network; the default is 50.

## What a lab 5.12 showed (seihai-gserver #230, 2026-11-27)

| | Result |
|---|---|
| Join next to a town (1,952 objects in scope) | phase 2 in 2.4 s at `sendms 10`, about 7 s at 50 |
| A region edge | a window slide, relit 0.1 to 0.3 s later; no seam, stall or pop-in |
| Ghost table | peak 2,075 of 4,096 |
| Frame rate (software rendering) | 32 to 41 FPS in the open, 18 to 20 in a 1,440-house town |
| Memory | at most 621 MB committed of the 2 GB a 5.12 can address |

## Known limits

- Dense towns are slow under software rendering; that is the client's frame
  rate, not the server.
- The player shape is a plain rock; there is no walker script, so the camera
  moves only where a client script or the server moves it (the world is a
  streaming and scoping test, not a game).
