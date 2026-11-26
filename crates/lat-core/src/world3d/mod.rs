//! The 3D level editor's logic (`seihai-edit3d`): a seihai-gserver world
//! folder's 3D world file, its streamed terrain and a preview of both.
//!
//! - [`doc`]: the world file (`config/graal3dworld.txt` or
//!   `config/graal3dworlds/<name>.txt`) as a document edited line by line;
//! - [`terrain`]: the `terrain` line's folder as one height field, with
//!   brushes;
//! - [`shape`]: `.3ds` triangles and colours for the preview;
//! - [`view`]: the preview renderer;
//! - [`World`] here ties them to a folder on disk: where the files are,
//!   which shapes the preview found, and saving (through
//!   [`World::save_files`], which the tool writes with a backup).

pub mod doc;
pub mod shape;
pub mod terrain;
pub mod view;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use doc::WorldDoc;
use terrain::Field;
use view::Scene;

pub use terrain::Brush;

/// Shapes past this many triangles are drawn as markers in the preview.
pub const MAX_SHAPE_TRIS: usize = 200_000;

/// The object index the spawn point's marker carries in a [`Scene`].
pub const SPAWN_MARKER: usize = u32::MAX as usize - 1;

/// A world opened from a gserver world folder.
#[derive(Debug, Clone)]
pub struct World {
    /// The gserver world folder (the one with `config/`).
    pub root: PathBuf,
    /// The world file, inside `root`.
    pub file: PathBuf,
    /// The world's name: `default`, or the `<name>` of
    /// `config/graal3dworlds/<name>.txt`.
    pub name: String,
    pub doc: WorldDoc,
    /// The terrain the `terrain` line names, if it loaded.
    pub terrain: Option<Field>,
    /// Why the terrain did not load.
    pub terrain_error: Option<String>,
    /// Lower-cased file name to path, for every file under `root`.
    index: HashMap<String, PathBuf>,
    shapes: HashMap<String, Result<Vec<shape::Tri>, String>>,
}

/// The world names in a folder: `default` first, then
/// `config/graal3dworlds/*.txt` by name.
pub fn world_names(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if root.join("config/graal3dworld.txt").is_file() {
        out.push("default".to_string());
    }
    let mut others: Vec<String> = fs::read_dir(root.join("config/graal3dworlds"))
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.strip_suffix(".txt").map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    others.sort();
    out.extend(others);
    out
}

/// The world file of `name` in `root`.
pub fn world_file(root: &Path, name: &str) -> PathBuf {
    if name == "default" {
        root.join("config/graal3dworld.txt")
    } else {
        root.join("config/graal3dworlds")
            .join(format!("{name}.txt"))
    }
}

fn index_files(root: &Path) -> HashMap<String, PathBuf> {
    let mut out = HashMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let n = e.file_name().to_string_lossy().to_ascii_lowercase();
                // The first one found wins, as a file folder search would.
                out.entry(n).or_insert(p);
            }
        }
    }
    out
}

impl World {
    /// Open world `name` (`default` or a `graal3dworlds` name) in the world
    /// folder `root`. A world file that does not exist yet opens empty.
    pub fn open(root: &Path, name: &str) -> Result<World, String> {
        if !root.is_dir() {
            return Err(format!("{}: not a folder", root.display()));
        }
        let file = world_file(root, name);
        let text = match fs::read(&file) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("{}: {e}", file.display())),
        };
        let mut w = World {
            root: root.to_path_buf(),
            file,
            name: name.to_string(),
            doc: WorldDoc::parse(&text),
            terrain: None,
            terrain_error: None,
            index: index_files(root),
            shapes: HashMap::new(),
        };
        w.reload_terrain();
        Ok(w)
    }

    /// The terrain folder on disk, from the `terrain` line.
    pub fn terrain_dir(&self) -> Option<PathBuf> {
        self.doc.terrain().map(|t| self.root.join(t.folder))
    }

    /// Read the terrain the world line names again.
    pub fn reload_terrain(&mut self) {
        self.terrain = None;
        self.terrain_error = None;
        let Some(line) = self.doc.terrain() else {
            return;
        };
        match Field::load(&self.root.join(&line.folder), line.square) {
            Ok(f) => self.terrain = Some(f),
            Err(e) => self.terrain_error = Some(e),
        }
    }

    /// The path of a file the world names, looked up by name anywhere in
    /// the world folder (case ignored), as the server's file folders do.
    pub fn find(&self, name: &str) -> Option<&PathBuf> {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        self.index.get(&base.to_ascii_lowercase())
    }

    /// Every `.3ds`, `.dts` and `.dif` in the world folder, by name.
    pub fn shape_files(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .index
            .keys()
            .filter(|n| n.ends_with(".3ds") || n.ends_with(".dts") || n.ends_with(".dif"))
            .cloned()
            .collect();
        out.sort();
        out
    }

    /// The triangles of a shape, read once.
    pub fn shape(&mut self, name: &str) -> &Result<Vec<shape::Tri>, String> {
        let key = name.to_ascii_lowercase();
        if !self.shapes.contains_key(&key) {
            let r = if !key.ends_with(".3ds") {
                Err(format!("{name}: the preview reads only .3ds"))
            } else {
                match self.find(name) {
                    None => Err(format!("{name}: not in the world folder")),
                    Some(p) => fs::read(p)
                        .map_err(|e| format!("{name}: {e}"))
                        .and_then(|b| shape::read_3ds(&b).map_err(|e| format!("{name}: {e}")))
                        .and_then(|t| {
                            if t.len() > MAX_SHAPE_TRIS {
                                Err(format!("{name}: {} triangles, drawn as a box", t.len()))
                            } else {
                                Ok(t)
                            }
                        }),
                }
            };
            self.shapes.insert(key.clone(), r);
        }
        &self.shapes[&key]
    }

    /// Forget read shapes (after files changed on disk).
    pub fn rescan(&mut self) {
        self.index = index_files(&self.root);
        self.shapes.clear();
    }

    /// The colour of each terrain material: its texture's average when the
    /// texture is in the world folder, else one picked from its name.
    pub fn terrain_colours(&self) -> Vec<[u8; 3]> {
        let Some(t) = &self.terrain else {
            return Vec::new();
        };
        t.names
            .iter()
            .map(|n| {
                self.find(n)
                    .and_then(|p| fs::read(p).ok())
                    .and_then(|b| crate::render::decode_image(&b))
                    .and_then(|img| average(&img))
                    .unwrap_or_else(|| name_colour(n))
            })
            .collect()
    }

    /// Everything to draw, and a note for every object the preview could
    /// not read (drawn as a box). `terrain_step` thins the terrain mesh.
    pub fn scene(&mut self, terrain_step: usize) -> (Scene, Vec<String>) {
        let sun = self.doc.sun().unwrap_or_default();
        let fog = self.doc.sky().and_then(|s| s.fog);
        let mut scene = Scene::new(&sun, fog);
        let mut notes = Vec::new();
        if let Some(t) = &self.terrain {
            let colours = self.terrain_colours();
            scene.add_terrain(t, &colours, terrain_step);
        }
        if let Some(e) = &self.terrain_error {
            notes.push(format!("terrain: {e}"));
        }
        for w in self.doc.waters() {
            scene.add_water(&w);
        }
        for (i, o) in self.doc.objects().iter().enumerate() {
            match self.shape(&o.shape).clone() {
                Ok(tris) => scene.add_shape(&tris, o.pos, o.turn, i),
                Err(e) => {
                    notes.push(format!("{}: {e}", o.name));
                    scene.add_marker(o.pos, [220, 40, 200], i);
                }
            }
        }
        if let Some(s) = self.doc.spawn() {
            scene.add_marker([s[0], s[1], s[2] - 1.0], [250, 220, 0], SPAWN_MARKER);
        }
        (scene, notes)
    }

    /// A camera that sees the world: the `camera` line if there is one,
    /// else over the spawn (or the origin), looking down at it.
    pub fn default_camera(&self) -> view::Camera {
        let target = self.doc.spawn().unwrap_or([0.0, 0.0, 0.0]);
        view::Camera::orbit(target, 40.0, 0.0, 0.6)
    }

    /// The files a save writes, relative to `root`: the world file, and the
    /// terrain folder's chunks and `heights.info` when `with_terrain`.
    pub fn save_files(&self, with_terrain: bool) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
        let mut out = vec![(self.file.clone(), self.doc.text().into_bytes())];
        if with_terrain {
            if let (Some(t), Some(dir)) = (&self.terrain, self.terrain_dir()) {
                for (name, bytes) in t.files()? {
                    out.push((dir.join(name), bytes));
                }
            }
        }
        Ok(out)
    }

    /// The ground height under (x, y): the terrain's, else 0.
    pub fn ground(&self, x: f64, y: f64) -> f64 {
        self.terrain
            .as_ref()
            .and_then(|t| t.height_at(x, y))
            .unwrap_or(0.0)
    }

    /// Where a ray from `eye` meets the ground (terrain, else z = 0), by
    /// stepping along it; `None` if it never does within 4096 units.
    pub fn hit_ground(&self, eye: [f64; 3], dir: [f64; 3]) -> Option<[f64; 3]> {
        if self.terrain.is_none() {
            if dir[2] >= 0.0 {
                return None;
            }
            let t = -eye[2] / dir[2];
            return Some([eye[0] + dir[0] * t, eye[1] + dir[1] * t, 0.0]);
        }
        let mut prev = 0.0;
        let mut t = 0.0;
        while t < 4096.0 {
            let p = [
                eye[0] + dir[0] * t,
                eye[1] + dir[1] * t,
                eye[2] + dir[2] * t,
            ];
            if p[2] <= self.ground(p[0], p[1]) {
                // Halve back to the crossing.
                let (mut lo, mut hi) = (prev, t);
                for _ in 0..20 {
                    let mid = (lo + hi) / 2.0;
                    let q = [
                        eye[0] + dir[0] * mid,
                        eye[1] + dir[1] * mid,
                        eye[2] + dir[2] * mid,
                    ];
                    if q[2] <= self.ground(q[0], q[1]) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                let q = [eye[0] + dir[0] * hi, eye[1] + dir[1] * hi];
                return Some([q[0], q[1], self.ground(q[0], q[1])]);
            }
            prev = t;
            t += 0.5;
        }
        None
    }
}

fn average(img: &crate::preview::Rgba) -> Option<[u8; 3]> {
    let n = (img.width * img.height) as u64;
    if n == 0 {
        return None;
    }
    let mut sum = [0u64; 3];
    for p in img.pixels.chunks_exact(4) {
        for c in 0..3 {
            sum[c] += u64::from(p[c]);
        }
    }
    Some(sum.map(|s| (s / n) as u8))
}

/// A colour for a texture the preview does not have, from words in its
/// name (grass, sand, rock, ...), else from a hash of it.
fn name_colour(name: &str) -> [u8; 3] {
    let n = name.to_ascii_lowercase();
    for (word, rgb) in [
        ("grass", [60, 150, 50]),
        ("sand", [214, 196, 132]),
        ("rock", [120, 110, 100]),
        ("stone", [130, 130, 125]),
        ("dirt", [130, 95, 60]),
        ("snow", [235, 240, 245]),
        ("water", [40, 90, 200]),
    ] {
        if n.contains(word) {
            return rgb;
        }
    }
    let h = crate::code_cache::crc32(n.as_bytes());
    [
        80 + (h & 0x7F) as u8,
        80 + ((h >> 8) & 0x7F) as u8,
        80 + ((h >> 16) & 0x7F) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world folder made with servertools' own 3D generator (streamed
    /// terrain), as `gen 3d --stream --playerworld` lays it out.
    pub(crate) fn generated_world(dir: &Path) {
        let o = crate::generate::TerrainOptions {
            world: "isle".into(),
            width: 2,
            height: 2,
            seed: 7,
            base: 0,
            amplitude: 64.0,
            chaos: 0.5,
            even_borders: true,
        };
        let o3 = crate::terrain3d::Terrain3dOptions {
            stream: true,
            ..Default::default()
        };
        for (name, bytes) in crate::terrain3d::terrain3d(&o, &o3).unwrap() {
            let path = if name == "isle_graal3dworld.txt" {
                dir.join("config/graal3dworld.txt")
            } else if let Some(rest) = name.strip_prefix("isle_terrain/") {
                dir.join("world/isle_terrain").join(rest)
            } else {
                dir.join("world").join(name)
            };
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    #[test]
    fn opens_edits_and_draws_a_generated_world() {
        let dir = tempfile::tempdir().unwrap();
        generated_world(dir.path());
        fs::create_dir_all(dir.path().join("config/graal3dworlds")).unwrap();
        fs::write(
            dir.path().join("config/graal3dworlds/cave.txt"),
            "spawn 0 0 1\n",
        )
        .unwrap();
        assert_eq!(world_names(dir.path()), vec!["default", "cave"]);

        let mut w = World::open(dir.path(), "default").unwrap();
        let t = w.terrain.as_ref().expect("terrain loads");
        assert_eq!(t.missing, 0);
        assert!(
            w.doc.objects().iter().any(|o| o.shape.starts_with("isle_")),
            "{}",
            w.doc.text()
        );
        assert!(w.shape("isle_tree.3ds").is_ok());
        assert!(w.shape("ISLE_ROCK.3DS").is_ok());
        assert!(w.shape("house.dif").is_err());
        assert!(w.shape_files().contains(&"isle.3ds".to_string()));

        let (scene, notes) = w.scene(4);
        assert!(notes.is_empty(), "{notes:?}");
        assert!(scene.faces.len() > 1000);
        let cam = w.default_camera();
        let frame = view::render(&scene, &cam, 96, 64);
        // Something other than sky is in view.
        assert!(frame.image.pixels.chunks_exact(4).any(|p| p[1] > p[2] + 20));

        // A ray straight down from above the spawn meets the ground there.
        let s = w.doc.spawn().unwrap();
        let hit = w.hit_ground([s[0], s[1], 500.0], [0.0, 0.0, -1.0]).unwrap();
        assert!((hit[2] - w.ground(s[0], s[1])).abs() < 0.01);

        // Raise the ground under the spawn and save both.
        let before = w.ground(s[0], s[1]);
        w.terrain
            .as_mut()
            .unwrap()
            .apply(Brush::Raise(3.0), s[0], s[1], 24.0);
        w.doc.set_spawn([s[0], s[1], before + 4.0]);
        for (path, bytes) in w.save_files(true).unwrap() {
            fs::write(path, bytes).unwrap();
        }
        let again = World::open(dir.path(), "default").unwrap();
        assert!(again.ground(s[0], s[1]) > before + 2.5);
        assert_eq!(again.doc.spawn().unwrap()[2], before + 4.0 + 0.0);
    }

    #[test]
    fn missing_world_opens_empty_and_names_problems() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = World::open(dir.path(), "new").unwrap();
        assert_eq!(w.file, dir.path().join("config/graal3dworlds/new.txt"));
        assert!(w.doc.objects().is_empty());
        w.doc
            .add_object(&doc::Object {
                only_for: None,
                name: "box".into(),
                shape: "nope.3ds".into(),
                pos: [0.0; 3],
                turn: None,
            })
            .unwrap();
        let (_, notes) = w.scene(1);
        assert_eq!(
            notes,
            vec!["box: nope.3ds: not in the world folder".to_string()]
        );
        assert!(World::open(&dir.path().join("none"), "default").is_err());
    }
}
