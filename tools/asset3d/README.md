# seihai-asset3d

Turns other people's 3D models and pictures into shapes and textures the
5.12 3D client draws, and adds them to a seihai-gserver world folder, so a
playerworld's builders can place more than the shapes the generators make.

```
seihai-asset3d add FOLDER INPUT... [--name NAME] [options] [--force]
seihai-asset3d convert INPUT OUT_DIR [--name NAME] [options] [--force]
seihai-asset3d list FOLDER
seihai-asset3d remove FOLDER NAME
seihai-asset3d check FILE...
seihai-asset3d formats
seihai-servertools asset3d ...     # the same, from the front door
seihai-asset3d --help | --version
```

## What it reads

| Input | What comes across | What is dropped |
|---|---|---|
| `.glb`, `.gltf` (glTF 2.0) | every mesh of the default scene, placed by its node transforms; base colour and base colour texture (PNG or JPEG, inside the file, beside it or as a `data:` URI); triangles, strips, fans | skins and morph targets (the model comes as it rests), animations, normals, metal/roughness/emissive maps, a second UV set, points and lines. Draco and meshopt compression are refused |
| `.obj` + `.mtl` | polygons (cut into triangles), `Kd` colour, `map_Kd` texture | normals, groups, smoothing, other maps |
| `.stl` (binary or text) | triangles, in one grey material | (STL has no agreed colours) |
| `.3ds` | meshes, material colours, texture maps | rewritten with safe texture names and sizes |
| `.png .gif .mng .jpg .jpeg .bmp .tga` | an upright card facing south, 16 pixels a tile, cropped to its opaque part, transparent pixels cut out; `--solid` builds it from flat coloured strips instead | |
| `.dts`, `.dif` | copied after the 5.12's own checks: a `.dts` must be version 24 (17 to 23 pass the client's check, but a body it misreads hangs or crashes it), a `.dif` exactly version 44 | their textures are not converted |

Not read, with the reason (`formats` prints this):

- **FBX**: a closed format with no published specification. **`.blend`**:
  Blender's working file. **Collada (`.dae`), PLY**: not read yet. For all
  of these, open the file in Blender and export glTF (`.glb`) or OBJ.
- **MD2, MD3**: the 5.12 has no loader for them (4.04r2 had MD3), and
  shapes do not animate.
- **`.gani`**: a 2D animation, and GameObject shapes do not animate on the
  5.12. Save the frame you want as a PNG and convert that.
- **DDS textures**: the 5.12 draws them white.

## What it writes, and why

One `.3ds` an asset and its textures, kept to what a real 5.12 was seen to
draw (Graal_analysis `torque/live-512-shapes-2026-11-25.md`,
`live-512-own-server-textures-2026-11-25.md`, T3D-032, T3D-033, T3D-124):

- **Textures** are 8-bit PNG (16-bit PNG, BMP, TGA and DDS draw white on
  the 5.12), at most 1024 pixels a side (2048 drew white; `--max-texture`),
  and stretched to power-of-two sides (the nearest one; growing keeps
  pixels sharp). The 5.12 pads a texture with other sides and keeps the
  texture places, so a 44 x 64 picture filled only 44/64 of its card in
  the lab; stretching keeps every texture place, tiled ones too. Alpha is cut to 0 or 255: the 5.12 cuts out
  alpha 0 and draws any other alpha opaque, so half-transparent pixels
  become one or the other here rather than on the client.
- **Names** are lower case: the asset is `<name>.3ds`, its textures
  `<name>_0.png`, `<name>_1.png`. The client lower-cases a texture name,
  drops its folder and extension and asks the server for it once, so a
  name that is not already like that is a texture that never arrives.
  The asset name is the input's file name in letters and digits, or
  `--name`.
- **Mesh chunks** of at most 65 535 points and faces (the format's 16-bit
  counts; past them the shape draws nothing), and **at most 250 000 points
  a shape**: a 5.12 drew a shape of about 265 000 points and nothing of one
  of about 397 000, however it was split. Bigger models are refused
  (simplify them first, for example with Blender's Decimate) unless
  `--force`.
- **Every face has a material** (a mesh with none is not drawn).
- **Axes**: the clients' +z up, one unit a tile. glTF and OBJ are +y up
  (`--up` changes it); their front (+z) is turned to face south (-y), the
  way things in a 2D level face the player. `--turn` turns it further.
  `--scale` is tiles a source unit (default 1: a 1.8 m glTF person is 1.8
  tiles tall; a 2D player is about 3); `--height` scales it to a height
  instead. The origin goes to the middle of its footprint, on z = 0
  (`--origin centre|keep`), so `seihai-edit3d add ... --ground` stands it
  on the ground.
- **`--colour-only`** writes no textures: each textured face is cut into a
  grid of up to 16 x 16 pieces (about one a 8 texture pixels, up to 60 000
  faces a shape) and each piece takes the texture's colour at its middle,
  `--colour-bits` bits a channel. This is for the 4.20r8, which draws no
  `.3ds` textures at all, and for worlds that want no texture downloads.

The 5.12 draws both sides of every face (no back-face culling), ignores the
`.3ds` two-sided flag and has no blending; a card's back shows the picture
mirrored.

## Adding to a world: `add`, `list`, `remove`

`add FOLDER INPUT...` converts each input and writes its shape and
textures into `FOLDER/files/`, where seihai-gserver serves them by name,
and a line into `FOLDER/config/assets3d.txt`:

```
asset NAME SHAPE WIDTH DEPTH HEIGHT TEXTURES SOURCE
asset chair chair.3ds 1.2 1.1 1.5 chair_0,chair_1 chair.glb
```

That list is for the world's builders (what there is, how big, from
which file); the server does not read it. Any world file then places an
asset with an object line naming its shape:

```
seihai-edit3d add FOLDER chair1 chair.3ds 40 -22 0 --ground
```

or with the editor window's Place. Adding an asset of the same name again
replaces it (and removes textures it no longer has); a file in `files/`
that belongs to another asset or to the world itself is not overwritten
unless `--force`. Files are written through a temporary file and a
rename. `list` shows the assets and any of their files missing from
`files/`; `remove` deletes an asset's files and line (world files that
still place it will log it as missing).

`convert INPUT OUT_DIR` writes the same files to a folder of their own,
for a server laid out some other way.

`check FILE...` reads a `.3ds`, `.dts`, `.dif` or picture and lists what a
5.12 would have trouble with: too many points, a texture name with a
folder or capitals, a texture missing beside the shape, a 16-bit PNG, a
BMP, TGA or DDS, a texture over 1024 pixels, a `.dts` or `.dif` version
the client refuses.

## Lab check

See `docs/lab/2026-11-27-asset3d-512.md`.

## Build and test

```
cargo build -p seihai-asset3d
cargo test -p seihai-asset3d
```

The readers, the limits and the `.3ds` writer live in `crates/lat-core`
(`lat_core::asset3d`: `gltf`, `obj`, `stl`, `tds`, `card`, `texture`,
`prepare`, `write`, `manifest`) with their unit tests; this crate reads the
command line and writes files. It depends on no other tool, only on
`lat-core` (which reaches `seihai-core`, pinned once in the root
`Cargo.toml` by tag, v0.7.2, for MNG pictures), plus the `image` crate
for decoding pictures and `serde_json` for glTF (both MIT OR Apache-2.0).
The tests make their own models and pictures and run the program on them.
