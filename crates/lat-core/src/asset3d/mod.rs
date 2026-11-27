//! 3D assets for a playerworld: other people's models and pictures as
//! shapes the 3D clients draw.
//!
//! The 5.12 client draws a GameObject's `.3ds` with textured materials when
//! the server sends the shape and its textures (Graal_analysis
//! `torque/live-512-own-server-textures-2026-11-25.md`, T3D-032/T3D-033), so
//! the target here is one `.3ds` an asset with its textures beside it, kept
//! inside what a real 5.12 was seen to draw:
//!
//! | Limit | Why | Source |
//! |---|---|---|
//! | textures 8-bit PNG, 1024 px a side at most | 16-bit PNG, BMP, TGA and DDS draw white; 2048 px drew white on the 5.12 | `live-512-shapes-2026-11-25.md` |
//! | texture names lower case, no folder, no extension | the client lower-cases the name, strips folder and extension and asks once | T3D-033 |
//! | alpha cut to 0 or 255 | alpha 0 is cut out, anything else draws opaque (no blending) | T3D-032 |
//! | 65 535 points and faces a mesh chunk | the `.3ds` counts are 16-bit; past that the shape draws nothing | T3D-124 |
//! | 250 000 points a shape | a shape past about 265 000 draws nothing, however it is split | T3D-124 |
//! | every face has a material | a mesh with no material is not drawn | T3D-032 |
//!
//! The steps: a reader ([`obj`], [`gltf`], [`stl`], [`tds`], [`card`] for
//! pictures) gives a [`Model`]; [`prepare`] turns it to the clients' axes
//! (+z up, one unit a tile, the front facing south), scales and stands it
//! on the ground, makes the textures safe ([`texture`]) or bakes them into
//! face colours; [`write`] writes the `.3ds`. [`manifest`] keeps
//! `config/assets3d.txt`, the list of assets added to a world folder.

pub mod card;
pub mod gltf;
pub mod manifest;
pub mod obj;
pub mod prepare;
pub mod stl;
pub mod tds;
pub mod texture;
pub mod write;

use crate::preview::Rgba;
use std::path::{Path, PathBuf};

/// A point of a mesh: where it is and where it reads its texture.
/// `uv` has `v` going up from the bottom of the picture, as in `.3ds` and
/// `.obj` (glTF's top-down `v` is flipped on reading).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub p: [f32; 3],
    pub uv: [f32; 2],
}

/// A triangle: three [`Vertex`] indices and a [`Material`] index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub v: [u32; 3],
    pub m: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub colour: [u8; 3],
    /// Index into [`Model::textures`].
    pub texture: Option<usize>,
}

/// Which way is up in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Up {
    Y,
    Z,
}

/// What a reader gives: one mesh of triangles, its materials and the
/// pictures they use, and notes on what was not taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub materials: Vec<Material>,
    pub textures: Vec<Rgba>,
    /// The source format's usual up axis (glTF and OBJ +y, STL and 3DS +z).
    pub up: Up,
    pub notes: Vec<String>,
}

impl Model {
    pub fn new(up: Up) -> Self {
        Model {
            vertices: Vec::new(),
            faces: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            up,
            notes: Vec::new(),
        }
    }

    /// The smallest and largest corner of the points the faces use.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self
            .faces
            .iter()
            .flat_map(|f| f.v)
            .filter_map(|i| self.vertices.get(i as usize));
        let first = it.next()?.p;
        Some(it.fold((first, first), |(lo, hi), v| {
            (
                [lo[0].min(v.p[0]), lo[1].min(v.p[1]), lo[2].min(v.p[2])],
                [hi[0].max(v.p[0]), hi[1].max(v.p[1]), hi[2].max(v.p[2])],
            )
        }))
    }

    /// A material index for a plain colour, added if new.
    pub fn colour_material(&mut self, rgb: [u8; 3]) -> usize {
        let name = format!("c{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        if let Some(i) = self.materials.iter().position(|m| m.name == name) {
            return i;
        }
        self.materials.push(Material {
            name,
            colour: rgb,
            texture: None,
        });
        self.materials.len() - 1
    }
}

/// The input kinds, by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Obj,
    Gltf,
    Glb,
    Stl,
    Tds,
    Image,
    /// `.dts` or `.dif`: copied as they are after a version check.
    Torque,
}

/// The kind of `path` by its extension, or why it is not supported.
pub fn kind_of(path: &Path) -> Result<Kind, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    Ok(match ext.as_str() {
        "obj" => Kind::Obj,
        "gltf" => Kind::Gltf,
        "glb" => Kind::Glb,
        "stl" => Kind::Stl,
        "3ds" => Kind::Tds,
        "png" | "gif" | "mng" | "jpg" | "jpeg" | "bmp" | "tga" => Kind::Image,
        "dts" | "dif" => Kind::Torque,
        other => return Err(unsupported(other)),
    })
}

/// Why a format is not read, and what to do instead.
pub fn unsupported(ext: &str) -> String {
    let why = match ext {
        "fbx" => "FBX is a closed format with no published specification",
        "blend" => "a .blend file is Blender's own working file",
        "dae" => "Collada is not read yet",
        "ply" => "PLY is not read yet",
        "md2" | "md3" | "mdl" => {
            "the 5.12 has no MD2/MD3 loader (4.04r2 had MD3) and shapes do not animate"
        }
        "gani" => "a gani is a 2D animation; GameObject shapes do not animate on the 5.12",
        "dds" => "the 5.12 draws DDS textures white",
        "" => "the file has no extension",
        _ => "the format is not known",
    };
    format!(
        "{why}. Open it in Blender (or another editor) and export glTF (.glb) or OBJ, \
         or save a picture as PNG; see `seihai-asset3d formats`"
    )
}

/// How to convert.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Up axis of the source; `None` takes the format's usual one.
    pub up: Option<Up>,
    /// Source units a tile (applied before `height`).
    pub scale: f32,
    /// Scale so the asset is this many tiles tall.
    pub height: Option<f32>,
    /// Degrees about +z, counter-clockwise seen from above, after the
    /// front was turned to face south.
    pub turn: f32,
    pub origin: Origin,
    /// Largest texture side in pixels.
    pub max_texture: u32,
    /// Bake textures into face colours (for clients that draw no
    /// textures, such as 4.20r8).
    pub colour_only: bool,
    /// Bits a channel for baked colours (1 to 8).
    pub colour_bits: u8,
    /// Pictures: pixels a tile.
    pub px_per_tile: f32,
    /// Pictures: a board of pixel blocks instead of one textured card.
    pub solid: bool,
    /// Allow more points than the 5.12 draws.
    pub force: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            up: None,
            scale: 1.0,
            height: None,
            turn: 0.0,
            origin: Origin::Ground,
            max_texture: texture::MAX_SIDE,
            colour_only: false,
            colour_bits: 4,
            px_per_tile: 16.0,
            solid: false,
            force: false,
        }
    }
}

/// Where the asset's own origin goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The middle of its footprint, standing on z = 0.
    Ground,
    /// The middle of its box.
    Centre,
    /// As in the source.
    Keep,
}

/// One converted asset.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub name: String,
    /// The shape file name (`<name>.3ds`, or the `.dts`/`.dif` copied).
    pub shape: String,
    /// Files to write: name and bytes (the shape and its textures).
    pub files: Vec<(String, Vec<u8>)>,
    /// Width (x), depth (y) and height (z) in tiles.
    pub size: [f32; 3],
    pub points: usize,
    pub faces: usize,
    /// Texture names as the client asks for them.
    pub textures: Vec<String>,
    pub notes: Vec<String>,
}

/// The asset name for `path`: its file stem, lower case, letters and
/// digits only (the client lower-cases texture names, and `_` separates a
/// texture's number).
pub fn asset_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    clean_name(&stem)
}

/// `s` lower case, letters and digits only, at most 24 characters.
pub fn clean_name(s: &str) -> String {
    let mut n: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .take(24)
        .collect();
    if n.is_empty() || n.starts_with(|c: char| c.is_ascii_digit()) {
        n.insert(0, 'a');
    }
    n
}

/// Read and convert one input file. `name` overrides the asset name.
pub fn convert(path: &Path, name: Option<&str>, o: &Options) -> Result<Asset, String> {
    let name = match name {
        Some(n) => {
            let c = clean_name(n);
            if c != n {
                return Err(format!(
                    "name `{n}`: use lower-case letters and digits only (such as `{c}`)"
                ));
            }
            c
        }
        None => asset_name(path),
    };
    let kind = kind_of(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let model = match kind {
        Kind::Torque => return torque_copy(path, &name, &bytes),
        Kind::Obj => obj::read(&bytes, &FileLoader { dir })?,
        Kind::Gltf => gltf::read_gltf(&bytes, &FileLoader { dir })?,
        Kind::Glb => gltf::read_glb(&bytes, &FileLoader { dir })?,
        Kind::Stl => stl::read(&bytes)?,
        Kind::Tds => tds::read(&bytes, &FileLoader { dir })?,
        Kind::Image => {
            let img = texture::decode(&bytes)?;
            if o.solid || o.colour_only {
                card::blocks(&img, o.px_per_tile, o.colour_bits)
            } else {
                card::card(img, o.px_per_tile)
            }
        }
    };
    prepare::finish(model, &name, o)
}

/// Finds the files a model names (textures, `.mtl`, `.bin`) beside it.
pub struct FileLoader {
    pub dir: PathBuf,
}

impl FileLoader {
    /// The bytes of `rel` (relative to the model's folder; `\` taken as
    /// `/`). A name not found as written is looked for by its file name
    /// in any case, as exporters often write absolute or other-case paths.
    pub fn load(&self, rel: &str) -> Result<Vec<u8>, String> {
        let rel = rel.trim().replace('\\', "/");
        let direct = self.dir.join(&rel);
        if let Ok(b) = std::fs::read(&direct) {
            return Ok(b);
        }
        let leaf = rel.rsplit('/').next().unwrap_or("").to_lowercase();
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().to_lowercase() == leaf {
                    return std::fs::read(e.path()).map_err(|e| format!("{rel}: {e}"));
                }
            }
        }
        Err(format!("{rel}: not found beside the model"))
    }
}

/// `.dts` and `.dif` are copied as they are, after the version checks the
/// 5.12 makes: a `.dts` must say version 24 (17 to 23 pass the client's
/// check but a body it misreads hangs or crashes it, T3D-029), a `.dif`
/// exactly 44 (T3D-030).
fn torque_copy(path: &Path, name: &str, bytes: &[u8]) -> Result<Asset, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let word = |i: usize| -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(i..i + 4)?.try_into().ok()?))
    };
    let mut notes = vec![format!(
        "copied as it is; its textures are not converted: put them beside it in the \
         world's files/ as lower-case .png ({ext} materials are not read here)"
    )];
    if ext == "dts" {
        let v = word(0).ok_or("a .dts shorter than its version word")? & 0xff;
        match v {
            24 => {}
            17..=23 => notes.push(format!(
                "version {v}: the 5.12 accepts it, but only version 24 shapes were seen to draw"
            )),
            _ => {
                return Err(format!(
                    "a version {v} .dts: the 5.12 loads versions 17 to 24 only (save it as 24)"
                ))
            }
        }
    } else {
        let v = word(0).ok_or("a .dif shorter than its version word")?;
        if v != 44 {
            return Err(format!(
                "a version {v} .dif: the 5.12 loads interiors of version 44 only"
            ));
        }
    }
    let shape = format!("{name}.{ext}");
    Ok(Asset {
        name: name.to_string(),
        shape: shape.clone(),
        files: vec![(shape, bytes.to_vec())],
        size: [0.0; 3],
        points: 0,
        faces: 0,
        textures: Vec::new(),
        notes,
    })
}

/// The supported and unsupported inputs, for `formats`.
pub const FORMATS: &str = "\
Read:
  .glb .gltf   glTF 2.0: every mesh of the default scene with its node transforms,
               base colour and base colour texture (PNG or JPEG, in the file, beside
               it or as a data: URI). Triangles, strips and fans; points and lines
               are skipped. Skins, morph targets and animations are dropped (the
               mesh is taken as it rests).
  .obj         Wavefront OBJ with its .mtl: Kd colour and map_Kd texture; polygons
               are cut into triangles; groups and smoothing are ignored.
  .stl         binary or text STL, in one grey material (STL has no agreed colours).
  .3ds         3D Studio: meshes, material colours and texture maps; rewritten with
               safe texture names and sizes.
  .png .gif .mng .jpg .jpeg .bmp .tga
               a picture as an upright card facing south, textured, 16 pixels a
               tile; transparent pixels are cut out. --solid builds it from small
               coloured blocks instead (for clients that draw no textures).
  .dts .dif    Torque shapes and interiors, copied after the 5.12's version checks
               (.dts version 24, .dif version 44). Their textures are not converted.

Not read (export glTF or OBJ from Blender instead):
  .fbx         closed format with no published specification
  .blend       Blender's own working file
  .dae .ply    not read yet
  .md2 .md3    the 5.12 has no MD2/MD3 loader, and shapes do not animate
  .gani        2D animation; GameObject shapes do not animate on the 5.12. Save the
               frame you want as a PNG and convert that.

Written: one .3ds an asset (16-bit mesh chunks, every face with a material) and its
textures as 8-bit PNG, at most 1024 pixels a side, alpha 0 or 255, lower-case names
<asset>_<n>. Animations, bones, normals, smoothing, transparency between 0 and 255,
second UV sets, emissive, metal and roughness maps are dropped.";

/// What `check` found in one file: facts, and problems a 5.12 would have.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckReport {
    pub facts: Vec<String>,
    pub problems: Vec<String>,
}

/// Check a shape, interior or picture against what a 5.12 draws.
pub fn check(path: &Path) -> Result<CheckReport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut r = CheckReport::default();
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    match ext.as_str() {
        "3ds" => {
            let m = tds::read(&bytes, &FileLoader { dir: dir.clone() })?;
            let pieces = write::pieces(&m.vertices, &m.faces);
            let points: usize = pieces.iter().map(|p| p.vertices.len()).sum();
            r.facts.push(format!(
                "{points} points, {} faces, {} materials",
                m.faces.len(),
                m.materials.len()
            ));
            if points > prepare::MAX_POINTS {
                r.problems.push(format!(
                    "{points} points: a 5.12 draws nothing of a shape past about {}",
                    prepare::MAX_POINTS
                ));
            }
            for name in tds::texture_names(&bytes) {
                let leaf = name.rsplit(['/', '\\']).next().unwrap_or("").to_string();
                let bare = leaf.split('.').next().unwrap_or("").to_string();
                r.facts.push(format!(
                    "texture {name} (asked for as `{}`)",
                    bare.to_lowercase()
                ));
                if leaf != name {
                    r.problems.push(format!(
                        "texture {name}: the client drops the folder and asks for `{}`",
                        bare.to_lowercase()
                    ));
                }
                if bare != bare.to_lowercase() {
                    r.problems.push(format!(
                        "texture {name}: the client asks for it in lower case; name the file `{}.png`",
                        bare.to_lowercase()
                    ));
                }
                match (FileLoader { dir: dir.clone() }).load(&leaf) {
                    Ok(b) => r.problems.extend(picture_problems(&leaf, &b)),
                    Err(_) => r.problems.push(format!(
                        "texture {name} is not beside the shape; the shape draws white without it"
                    )),
                }
            }
            r.problems.extend(m.notes);
        }
        "dts" | "dif" => match torque_copy(path, "x", &bytes) {
            Ok(a) => {
                r.facts.push(format!("a {ext} the 5.12 loads"));
                r.problems.extend(a.notes.into_iter().skip(1));
            }
            Err(e) => r.problems.push(e),
        },
        _ => {
            let img = texture::decode(&bytes)?;
            r.facts
                .push(format!("a {} x {} picture", img.width, img.height));
            r.problems.extend(picture_problems(
                &path.file_name().unwrap_or_default().to_string_lossy(),
                &bytes,
            ));
        }
    }
    Ok(r)
}

/// Why a picture would draw white (or crash) on a 5.12, if it would.
fn picture_problems(name: &str, bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let kind = if bytes.starts_with(b"\x89PNG") {
        if bytes.get(24) == Some(&16) {
            out.push(format!("{name}: a 16-bit PNG draws white on the 5.12"));
        }
        "png"
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        "jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else if bytes.starts_with(b"BM") {
        out.push(format!("{name}: a BMP draws white on the 5.12; convert it"));
        "bmp"
    } else if bytes.starts_with(b"DDS ") {
        out.push(format!("{name}: a DDS draws white on the 5.12; convert it"));
        "dds"
    } else {
        out.push(format!(
            "{name}: not PNG, JPEG or GIF (a TGA draws white on the 5.12); convert it"
        ));
        "other"
    };
    if kind != "dds" && kind != "other" {
        if let Ok(img) = texture::decode(bytes) {
            if img.width.max(img.height) > texture::MAX_SIDE {
                out.push(format!(
                    "{name}: {} x {}; the 5.12 drew 2048 px textures white (keep to {})",
                    img.width,
                    img.height,
                    texture::MAX_SIDE
                ));
            }
        }
    }
    out
}
