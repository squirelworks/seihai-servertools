//! `config/assets3d.txt`: the 3D assets added to a world folder, one line
//! an asset, so a world's builders can see what there is to place and how
//! big it is. The shapes and textures themselves are in `files/`, where
//! seihai-gserver serves them by name; any world file places one with an
//! `object NAME SHAPE X Y Z` line (`seihai-edit3d add` writes those).
//!
//! ```text
//! asset NAME SHAPE WIDTH DEPTH HEIGHT TEXTURES SOURCE
//! asset chair chair.3ds 1.2 1.1 1.5 chair_0,chair_1 chair.glb
//! ```
//!
//! `TEXTURES` is `-` for none; `SOURCE` (the rest of the line) is the
//! file it was made from, without its folder. Other lines (comments,
//! blank lines) are kept as written.

/// The manifest's place in a world folder.
pub const PATH: &str = "config/assets3d.txt";

const HEADER: &str = "\
# 3D assets for this world, kept by seihai-asset3d. Shapes and textures are in files/.
# asset NAME SHAPE WIDTH DEPTH HEIGHT TEXTURES SOURCE";

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub shape: String,
    /// Width (x), depth (y), height (z) in tiles.
    pub size: [f32; 3],
    pub textures: Vec<String>,
    pub source: String,
}

impl Entry {
    fn line(&self) -> String {
        let tex = if self.textures.is_empty() {
            "-".to_string()
        } else {
            self.textures.join(",")
        };
        let f = |v: f32| crate::world3d::doc::fmt_num(((v * 100.0).round() / 100.0) as f64);
        format!(
            "asset {} {} {} {} {} {} {}",
            self.name,
            self.shape,
            f(self.size[0]),
            f(self.size[1]),
            f(self.size[2]),
            tex,
            if self.source.is_empty() {
                "-"
            } else {
                &self.source
            }
        )
    }

    fn parse(line: &str) -> Option<Entry> {
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.first() != Some(&"asset") || w.len() < 7 {
            return None;
        }
        let n = |s: &str| s.parse::<f32>().ok();
        Some(Entry {
            name: w[1].into(),
            shape: w[2].into(),
            size: [n(w[3])?, n(w[4])?, n(w[5])?],
            textures: if w[6] == "-" {
                Vec::new()
            } else {
                w[6].split(',').map(str::to_string).collect()
            },
            source: w.get(7..).map(|r| r.join(" ")).unwrap_or_default(),
        })
    }

    /// Every file of this asset in `files/`.
    pub fn files(&self) -> Vec<String> {
        let mut f = vec![self.shape.clone()];
        f.extend(self.textures.iter().map(|t| format!("{t}.png")));
        f
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manifest {
    lines: Vec<String>,
}

impl Manifest {
    pub fn parse(text: &str) -> Self {
        Manifest {
            lines: text.lines().map(str::to_string).collect(),
        }
    }

    pub fn text(&self) -> String {
        let mut lines = self.lines.clone();
        if lines.is_empty() || !lines[0].starts_with('#') {
            lines.splice(0..0, HEADER.lines().map(str::to_string));
        }
        let mut t = lines.join("\n");
        t.push('\n');
        t
    }

    pub fn entries(&self) -> Vec<Entry> {
        self.lines.iter().filter_map(|l| Entry::parse(l)).collect()
    }

    pub fn get(&self, name: &str) -> Option<Entry> {
        self.entries().into_iter().find(|e| e.name == name)
    }

    /// Replace the entry of the same name, or add it at the end.
    pub fn set(&mut self, e: &Entry) {
        let line = e.line();
        match self
            .lines
            .iter()
            .position(|l| Entry::parse(l).is_some_and(|x| x.name == e.name))
        {
            Some(i) => self.lines[i] = line,
            None => self.lines.push(line),
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Entry> {
        let i = self
            .lines
            .iter()
            .position(|l| Entry::parse(l).is_some_and(|x| x.name == name))?;
        Entry::parse(&self.lines.remove(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_other_lines_and_replaces_by_name() {
        let mut m = Manifest::parse("# mine\n\nasset a a.3ds 1 2 3 - a.obj\n");
        let mut e = m.get("a").unwrap();
        assert_eq!(e.size, [1.0, 2.0, 3.0]);
        e.textures = vec!["a_0".into()];
        e.size[2] = 1.234;
        m.set(&e);
        m.set(&Entry {
            name: "b".into(),
            shape: "b.dts".into(),
            size: [0.0; 3],
            textures: vec![],
            source: "my b.dts".into(),
        });
        assert_eq!(
            m.text(),
            "# mine\n\nasset a a.3ds 1 2 1.23 a_0 a.obj\nasset b b.dts 0 0 0 - my b.dts\n"
        );
        assert_eq!(m.get("b").unwrap().source, "my b.dts");
        assert_eq!(m.get("a").unwrap().files(), vec!["a.3ds", "a_0.png"]);
        assert!(m.remove("a").is_some());
        assert_eq!(m.entries().len(), 1);
        assert!(Manifest::default().text().starts_with("# 3D assets"));
    }
}
