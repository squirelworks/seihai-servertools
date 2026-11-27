# Seihai 3D test world: Interiors sample (doors without a world switch)

A small original 2D playerworld, a start level with a house and a one-room
interior, turned into 3D by `seihai-pw3d`. The interior is placed in the same
3D world 1,536 units south of the town and joined to the front door by
`portal` lines, so walking through the door is a move, not a world switch.

## What it demonstrates

- `seihai-pw3d` (seihai-servertools #134): each level as a coloured `.3ds`
  ground with raised walls and water, sign words on boards, and the
  `PW3DWalk` weapon that walks the player, stops it at walls and takes the
  links.
- Interiors in the same world with `portal` lines (seihai-gserver #231): the
  server has the far side ready before the player steps through, and
  nothing inside is drawn from outside (it is past the 5.12's draw cut).
- `input/` holds the 2D levels it was made from, so you can change them and
  run `seihai-pw3d input <out> --start sample_start.nw` again.

## Run it

seihai-gserver only (seihai-gserver-v2 has no `portal` line yet). Needs
seihai-gserver 0.2.0 or a `main` with #231.

```sh
seihai-gserver --world world
```

Required settings, already in `world/config/server.txt`:

```text
name = 3 Seihai Interiors
graal3d = true
graal3d_scope = on     # so the house is sent only when you reach its door
```

Walk with the arrow keys (the `PW3DWalk` weapon). The door is the gap in the
south wall of the house, north of the spawn.

## Known limits

- The interior keeps the outdoor sky and light.
- `--separate-interiors` (one world per interior) makes each door a world
  switch; give every such world a Sun or terrain (see the freeze below).
