# seihai-edit3d

A standalone 3D level editor for seihai-gserver worlds. It edits a 3D
world file (`config/graal3dworld.txt`, or `config/graal3dworlds/<name>.txt`)
and the streamed terrain its `terrain` line names, with a live preview.

```
seihai-edit3d FOLDER [--world NAME] [--renderer CMD]      # the window
seihai-edit3d worlds FOLDER
seihai-edit3d show FOLDER [--world NAME]
seihai-edit3d add FOLDER NAME SHAPE X Y Z [--turn DEG] [--ground] [--for ACCOUNTS]
seihai-edit3d move FOLDER NAME X Y Z [--turn DEG] [--ground]
seihai-edit3d remove FOLDER NAME
seihai-edit3d spawn FOLDER X Y Z [--ground]
seihai-edit3d terrain FOLDER raise|lower|flatten|smooth|paint X Y RADIUS VALUE
seihai-edit3d render FOLDER OUT.png [--size WxH] [--eye X,Y,Z --yaw DEG --pitch DEG] [--fov DEG] [--renderer CMD]
seihai-servertools edit3d ...      # the same, from the front door
seihai-edit3d --help | --version
```

`FOLDER` is a gserver world folder, the one with `config/` in it: what
`seihai-pw3d`, `gen terrain --3d --playerworld` and `gen 3d --playerworld`
write. Shapes and textures are found by name anywhere in it, as the
server's file folders find them.

## Why a separate editor

The 3D clients draw what the server sends; the world lives in server files
(seihai-gserver `docs/GRAAL3D.md`, "The world file"). The 5.12 client has
Torque's `WorldEditor`, `TerrainEditor` and `EditManager` classes, and a
`weapon3d` can create them, but nothing they do is saved: no `.mis`, `.ter`
or `.cs` is written, and `TerrainEditor` cannot attach to the server's
terrain (Graal_analysis `torque/3d-batch-w-rows-2026-11-26.md`, T3D-126;
`torque/3d-clientside-editor-gating-420r6-2026-11-26.md`). So the editor
works on the server's files directly.

## The window

- **Left**: the world's objects (filter by name). Picking one centres the
  view on it.
- **Middle**: the preview. Right-drag turns the view, middle-drag (or
  shift + right-drag) moves it, the wheel zooms. Left-click:
  - *Select*: picks the object under the pointer;
  - *Place*: adds the chosen shape where the pointer meets the ground,
    named after the shape (`tree`, `tree2`, ...);
  - *Terrain*: applies the brush (raise, lower, flatten to a height,
    smooth, paint a `heights.info` material) under the pointer; dragging
    paints a stroke, one undo step a stroke.
- **Right**: the selected object's name, shape, place and turn about +z
  (Copy, Delete, Stand on the ground), the spawn (or "Spawn at the view's
  centre"), the sun's direction, light and ambient colours, and the
  problems the server would log for this world.
- **Top**: the world (switching asks before dropping unsaved changes),
  Save, Undo, Redo (40 steps, terrain included), Reload files.

## Saving

Saving keeps the world file as written: comments, blank lines, order and
lines the editor does not know stay byte for byte; a changed line keeps
its indent and trailing comment; a new object goes after the last object
line. The old file is kept as `<file>.bak-YYYYMMDD-HHMMSS` and the new
one written through a temporary file and a rename (as the setup tools
save `serveroptions.txt`). When the terrain changed, its folder is first
copied to `backups/<folder name>.bak-YYYYMMDD-HHMMSS` under the world
folder (not beside it, where the server would serve the copy too), then
all 256 chunks and
`heights.info` are written in the client's own encoding, so the stamps a
returning client sends match and only changed chunks go out again
(`lat_core::terrain_chunks`). Terrain holes are not kept yet.

## The preview

The built-in preview (`lat_core::world3d::view`) draws the `.3ds` shapes
(material colours, no textures), the streamed terrain (each material the
average colour of its texture when the texture is in the folder) and water
surfaces, lit flat by the world's sun, with the spawn as a yellow box.
`.dts` and `.dif` shapes, missing shapes and shapes over 200,000 triangles
are drawn as magenta boxes and listed under Problems. It uses the
conventions the Graal3D renderer work confirmed against a real 5.12: one
unit a tile, the clients' turn sense (a positive turn about +z swings +y
toward +x), a water surface at z + its z size. It is for placing things,
not a pixel match of the client.

### Another renderer

`--renderer CMD` hands every frame to another program, such as a build of
the Graal3D renderer that reads world files:

```
CMD WORLD_FILE ROOT OUT.png EX EY EZ YAW PITCH FOV W H
```

`WORLD_FILE` is the world being edited, saved or not, written to a
temporary file; `ROOT` is the world folder; the camera is at `EX EY EZ`,
turned `YAW` degrees right of +y and `PITCH` degrees down, with a
horizontal field of view of `FOV` degrees; the program writes a `W` x `H`
PNG to `OUT.png`. If it fails, the window says so and goes back to the
built-in preview. Object picking needs the built-in preview.

## Build and test

```
cargo build -p seihai-edit3d
cargo test -p seihai-edit3d
cargo build -p seihai-edit3d --no-default-features    # no window
```

The world file document, terrain field and brushes, `.3ds` reader and
preview live in `crates/lat-core` (`lat_core::world3d`: `doc`, `terrain`,
`shape`, `view`), with their unit tests; this crate reads the command line,
saves with backups (`setup-core`'s `config::backup`) and runs the window
(eframe, the `window` feature, on by default; the front door turns it on
with its `gui` feature). It depends on no other tool, only on `lat-core`,
`setup-core` and eframe/egui, and reaches `seihai-core` (pinned once in the
root `Cargo.toml` by tag, v0.7.2) through `lat-core`. The tests build a
world with servertools' own 3D generator and run the program on it.

## Not done yet

- Objects in `.dts` and `.dif`, textures on `.3ds` shapes, sky boxes.
- `replicator`, `objectfor` accounts, `weapon3d` and `camera` lines are
  kept and shown but not edited in the window (`--for` adds an `objectfor`
  from the command line).
- Moving objects by dragging in the preview; a grid.
- Live edits on a running server (the server reads the world file at start).
