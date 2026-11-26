//! The world file, as a document the editor changes line by line.
//!
//! The format is seihai-gserver's `config/graal3dworld.txt` (its
//! `docs/GRAAL3D.md`, "The world file"): one entry a line, the first word
//! says what it is, `#` starts a comment. The file is the owner's, so the
//! document keeps every line as written: comments, blank lines, the order,
//! and lines it does not understand. An edit rewrites only the line it
//! changes (keeping that line's indent and trailing comment) or appends one,
//! so an unchanged world saves byte for byte as it was read.

use std::collections::HashMap;

/// Every key seihai-gserver reads in a world file.
pub const KEYS: [&str; 17] = [
    "object",
    "objectfor",
    "sun",
    "sky",
    "skyoptions",
    "playershape",
    "movetype",
    "spawn",
    "camera",
    "tgecontrol",
    "tgeplayershape",
    "weapon3d",
    "terrain",
    "water",
    "replicator",
    // Lab entries.
    "rawobject",
    "rawevent",
];

/// A rotation: an axis and an angle in radians, as the world file and the
/// clients take it. The clients turn the opposite way from the right-hand
/// rule (a positive angle about +z swings +y toward +x).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turn {
    pub axis: [f64; 3],
    pub angle: f64,
}

impl Turn {
    /// A turn about +z, the usual one for objects standing on the ground.
    pub fn about_z(angle: f64) -> Self {
        Turn {
            axis: [0.0, 0.0, 1.0],
            angle,
        }
    }
}

/// An `object` or `objectfor` line.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// `objectfor`'s account list (comma-separated, as written); `None` for
    /// an `object` line, which every player is sent.
    pub only_for: Option<String>,
    pub name: String,
    /// The shape file (`.3ds`, `.dts` or `.dif`), as the line names it.
    pub shape: String,
    pub pos: [f64; 3],
    pub turn: Option<Turn>,
    /// Words after the turn (seihai-gserver reads a scale `sx sy sz`
    /// there), kept as written.
    pub rest: Vec<String>,
}

/// The `sun` line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sun {
    pub dir: [f64; 3],
    pub colour: [f64; 4],
    pub ambient: [f64; 4],
}

impl Default for Sun {
    /// The sun seihai-servertools' generators write.
    fn default() -> Self {
        Sun {
            dir: [0.57, 0.57, -0.57],
            colour: [0.85, 0.85, 0.8, 1.0],
            ambient: [0.35, 0.35, 0.4, 1.0],
        }
    }
}

/// The `sky` line.
#[derive(Debug, Clone, PartialEq)]
pub struct Sky {
    pub textures: Vec<String>,
    pub visible: Option<f64>,
    pub fog: Option<[f64; 3]>,
}

/// The `terrain` line: a folder of streamed chunks, 256 squares a side,
/// centred on the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainLine {
    pub folder: String,
    /// Units a square (seihai-gserver's default is 8).
    pub square: u32,
    pub repeat: bool,
}

/// A `water` line: a box from `corner`, `size` big; its surface is at
/// `corner z + size z` (live 5.12).
#[derive(Debug, Clone, PartialEq)]
pub struct Water {
    pub corner: [f64; 3],
    pub size: [f64; 3],
    pub texture: Option<String>,
    pub liquid: Option<i32>,
}

impl Water {
    pub fn surface(&self) -> f64 {
        self.corner[2] + self.size[2]
    }
}

/// One line split into its parts. `words` has no comment in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry<'a> {
    pub key: String,
    pub words: Vec<&'a str>,
}

/// The parts of a line: indent, the entry text, the comment (with its `#`).
fn split(line: &str) -> (&str, &str, &str) {
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);
    match rest.find('#') {
        Some(i) => {
            // Keep the space before the comment with the comment.
            let body = &rest[..i];
            let trimmed = body.trim_end();
            (indent, trimmed, &rest[trimmed.len()..])
        }
        None => (indent, rest.trim_end(), ""),
    }
}

/// The entry on a line, or `None` for a blank or comment line.
pub fn entry(line: &str) -> Option<Entry<'_>> {
    let (_, body, _) = split(line);
    let mut words = body.split_whitespace();
    let key = words.next()?.to_ascii_lowercase();
    Some(Entry {
        key,
        words: words.collect(),
    })
}

fn num(w: &str, what: &str) -> Result<f64, String> {
    w.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what}: `{w}` is not a number"))
}

fn nums<const N: usize>(w: &[&str], what: &str) -> Result<[f64; N], String> {
    if w.len() < N {
        return Err(format!("{what}: needs {N} numbers"));
    }
    let mut out = [0.0; N];
    for (o, s) in out.iter_mut().zip(w) {
        *o = num(s, what)?;
    }
    Ok(out)
}

/// A number as the world file writes it: up to four decimals, no trailing
/// zeros, no `-0`.
pub fn fmt_num(v: f64) -> String {
    let mut s = format!("{v:.4}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

impl Object {
    fn parse(key: &str, w: &[&str]) -> Result<Object, String> {
        let (only_for, w) = if key == "objectfor" {
            let (first, rest) = w
                .split_first()
                .ok_or("objectfor: needs the accounts, a name and a shape")?;
            (Some(first.to_string()), rest)
        } else {
            (None, w)
        };
        if w.len() < 5 {
            return Err(format!("{key}: needs a name, a shape and x y z"));
        }
        let pos = nums::<3>(&w[2..5], key)?;
        let turn = if w.len() >= 9 {
            let t = nums::<4>(&w[5..9], key)?;
            Some(Turn {
                axis: [t[0], t[1], t[2]],
                angle: t[3],
            })
        } else {
            None
        };
        Ok(Object {
            only_for,
            name: w[0].to_string(),
            shape: w[1].to_string(),
            pos,
            turn,
            rest: w
                .get(9..)
                .unwrap_or(&[])
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    fn line(&self) -> String {
        let mut out = match &self.only_for {
            Some(a) => format!("objectfor {a} "),
            None => "object ".to_string(),
        };
        out.push_str(&format!(
            "{} {} {} {} {}",
            self.name,
            self.shape,
            fmt_num(self.pos[0]),
            fmt_num(self.pos[1]),
            fmt_num(self.pos[2])
        ));
        if let Some(t) = self.turn {
            out.push_str(&format!(
                "  {} {} {} {}",
                fmt_num(t.axis[0]),
                fmt_num(t.axis[1]),
                fmt_num(t.axis[2]),
                fmt_num(t.angle)
            ));
            for r in &self.rest {
                out.push(' ');
                out.push_str(r);
            }
        }
        out
    }
}

/// A world file being edited.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldDoc {
    lines: Vec<String>,
    /// The file ended with a newline (kept on save).
    final_newline: bool,
}

impl WorldDoc {
    pub fn parse(text: &str) -> Self {
        let final_newline = text.is_empty() || text.ends_with('\n');
        let lines = text.lines().map(str::to_string).collect();
        WorldDoc {
            lines,
            final_newline,
        }
    }

    /// The text to save. A document nobody changed gives back the text it
    /// was parsed from (CRLF line ends become LF, as the server reads both).
    pub fn text(&self) -> String {
        let mut out = self.lines.join("\n");
        if self.final_newline && !self.lines.is_empty() {
            out.push('\n');
        }
        out
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Replace line `i`'s entry with `body`, keeping its indent and comment.
    fn set_line(&mut self, i: usize, body: String) {
        let (indent, _, comment) = split(&self.lines[i]);
        self.lines[i] = format!("{indent}{body}{comment}");
    }

    /// Indices of the lines whose key is `key`.
    fn find(&self, key: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| entry(l).is_some_and(|e| e.key == key))
            .map(|(i, _)| i)
            .collect()
    }

    /// Set the first `key` line (the server uses one of each) or append one.
    fn set_single(&mut self, key: &str, body: String) {
        match self.find(key).first() {
            Some(&i) => self.set_line(i, body),
            None => self.lines.push(body),
        }
    }

    fn remove_key(&mut self, key: &str) {
        self.lines.retain(|l| entry(l).is_none_or(|e| e.key != key));
    }

    /// The parsed value of the first `key` line, if it parses.
    fn single<T>(&self, key: &str, f: impl Fn(&[&str]) -> Result<T, String>) -> Option<T> {
        // The server keeps the last terrain line; one of the others is the
        // usual case, and for those the first is as good as any.
        let idx = if key == "terrain" {
            self.find(key).last().copied()
        } else {
            self.find(key).first().copied()
        }?;
        entry(&self.lines[idx]).and_then(|e| f(&e.words).ok())
    }

    // ---- objects ----

    /// Every `object` and `objectfor` line that parses, in file order.
    pub fn objects(&self) -> Vec<Object> {
        self.object_lines().into_iter().map(|(_, o)| o).collect()
    }

    fn object_lines(&self) -> Vec<(usize, Object)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let e = entry(l)?;
                if e.key == "object" || e.key == "objectfor" {
                    Object::parse(&e.key, &e.words).ok().map(|o| (i, o))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn object(&self, name: &str) -> Option<Object> {
        self.objects().into_iter().find(|o| o.name == name)
    }

    /// A name no object has yet: `base`, else `base2`, `base3`, ...
    pub fn free_name(&self, base: &str) -> String {
        let names: Vec<String> = self.objects().into_iter().map(|o| o.name).collect();
        if !names.iter().any(|n| n == base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base}{n}"))
            .find(|c| !names.contains(c))
            .expect("a free name")
    }

    /// Add an object after the last object line (or at the end).
    pub fn add_object(&mut self, o: &Object) -> Result<(), String> {
        check_name(&o.name)?;
        check_name(&o.shape)?;
        if self.object(&o.name).is_some() {
            return Err(format!("there is already an object named {}", o.name));
        }
        let at = self
            .object_lines()
            .last()
            .map_or(self.lines.len(), |(i, _)| i + 1);
        self.lines.insert(at, o.line());
        Ok(())
    }

    /// Replace the object named `name` with `o` (which may rename it).
    pub fn set_object(&mut self, name: &str, o: &Object) -> Result<(), String> {
        check_name(&o.name)?;
        check_name(&o.shape)?;
        if o.name != name && self.object(&o.name).is_some() {
            return Err(format!("there is already an object named {}", o.name));
        }
        let (i, _) = self
            .object_lines()
            .into_iter()
            .find(|(_, x)| x.name == name)
            .ok_or_else(|| format!("no object named {name}"))?;
        self.set_line(i, o.line());
        Ok(())
    }

    pub fn remove_object(&mut self, name: &str) -> Result<Object, String> {
        let (i, o) = self
            .object_lines()
            .into_iter()
            .find(|(_, x)| x.name == name)
            .ok_or_else(|| format!("no object named {name}"))?;
        self.lines.remove(i);
        Ok(o)
    }

    // ---- the one-of-each lines ----

    pub fn spawn(&self) -> Option<[f64; 3]> {
        self.single("spawn", |w| nums::<3>(w, "spawn"))
    }

    pub fn set_spawn(&mut self, p: [f64; 3]) {
        self.set_single(
            "spawn",
            format!(
                "spawn {} {} {}",
                fmt_num(p[0]),
                fmt_num(p[1]),
                fmt_num(p[2])
            ),
        );
    }

    /// The `camera` line, or `None` (the server then puts it 0 -5 2 from the
    /// spawn, facing it).
    pub fn camera(&self) -> Option<([f64; 3], Option<Turn>)> {
        self.single("camera", |w| {
            let p = nums::<3>(w, "camera")?;
            let t = if w.len() >= 7 {
                let t = nums::<4>(&w[3..7], "camera")?;
                Some(Turn {
                    axis: [t[0], t[1], t[2]],
                    angle: t[3],
                })
            } else {
                None
            };
            Ok((p, t))
        })
    }

    pub fn set_camera(&mut self, p: [f64; 3], turn: Option<Turn>) {
        let mut body = format!(
            "camera {} {} {}",
            fmt_num(p[0]),
            fmt_num(p[1]),
            fmt_num(p[2])
        );
        if let Some(t) = turn {
            body.push_str(&format!(
                "  {} {} {} {}",
                fmt_num(t.axis[0]),
                fmt_num(t.axis[1]),
                fmt_num(t.axis[2]),
                fmt_num(t.angle)
            ));
        }
        self.set_single("camera", body);
    }

    pub fn sun(&self) -> Option<Sun> {
        self.single("sun", |w| {
            let v = nums::<11>(w, "sun")?;
            Ok(Sun {
                dir: [v[0], v[1], v[2]],
                colour: [v[3], v[4], v[5], v[6]],
                ambient: [v[7], v[8], v[9], v[10]],
            })
        })
    }

    pub fn set_sun(&mut self, s: &Sun) {
        let f = |v: &[f64]| v.iter().map(|x| fmt_num(*x)).collect::<Vec<_>>().join(" ");
        self.set_single(
            "sun",
            format!("sun {}  {}  {}", f(&s.dir), f(&s.colour), f(&s.ambient)),
        );
    }

    pub fn sky(&self) -> Option<Sky> {
        self.single("sky", |w| {
            let textures = w
                .first()
                .ok_or("sky: needs its textures")?
                .split(',')
                .map(str::to_string)
                .collect();
            let visible = w.get(1).map(|v| num(v, "sky")).transpose()?;
            let fog = if w.len() >= 5 {
                Some(nums::<3>(&w[2..5], "sky")?)
            } else {
                None
            };
            Ok(Sky {
                textures,
                visible,
                fog,
            })
        })
    }

    pub fn set_sky(&mut self, s: Option<&Sky>) {
        let Some(s) = s else {
            self.remove_key("sky");
            return;
        };
        let mut body = format!("sky {}", s.textures.join(","));
        if let Some(v) = s.visible {
            body.push_str(&format!(" {}", fmt_num(v)));
            if let Some(f) = s.fog {
                body.push_str(&format!(
                    " {} {} {}",
                    fmt_num(f[0]),
                    fmt_num(f[1]),
                    fmt_num(f[2])
                ));
            }
        }
        self.set_single("sky", body);
    }

    /// The terrain line in effect (the server keeps the last one).
    pub fn terrain(&self) -> Option<TerrainLine> {
        self.single("terrain", |w| {
            let folder = w.first().ok_or("terrain: needs a folder")?.to_string();
            let mut square = 8;
            let mut repeat = false;
            for x in &w[1..] {
                if x.eq_ignore_ascii_case("repeat") {
                    repeat = true;
                } else {
                    square = x
                        .parse::<u32>()
                        .ok()
                        .filter(|s| (1..=64).contains(s))
                        .ok_or("terrain: the square size is 1 to 64")?;
                }
            }
            Ok(TerrainLine {
                folder,
                square,
                repeat,
            })
        })
    }

    pub fn waters(&self) -> Vec<Water> {
        self.find("water")
            .into_iter()
            .filter_map(|i| {
                let e = entry(&self.lines[i])?;
                let v = nums::<6>(&e.words, "water").ok()?;
                Some(Water {
                    corner: [v[0], v[1], v[2]],
                    size: [v[3], v[4], v[5]],
                    texture: e.words.get(6).map(|s| s.to_string()),
                    liquid: e.words.get(7).and_then(|s| s.parse().ok()),
                })
            })
            .collect()
    }

    /// What the server would complain about at start-up, and what the
    /// editor cannot place: unknown keys, lines that do not parse, two
    /// objects with one name. One message a problem, with its line number.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut names: HashMap<String, usize> = HashMap::new();
        for (i, l) in self.lines.iter().enumerate() {
            let Some(e) = entry(l) else { continue };
            let n = i + 1;
            if !KEYS.contains(&e.key.as_str()) {
                out.push(format!("line {n}: `{}` is not a world file entry", e.key));
                continue;
            }
            let parsed: Result<(), String> = match e.key.as_str() {
                "object" | "objectfor" => Object::parse(&e.key, &e.words).map(|o| {
                    if let Some(first) = names.insert(o.name.clone(), n) {
                        out.push(format!(
                            "line {n}: object {} is also on line {first}",
                            o.name
                        ));
                    }
                }),
                "spawn" | "camera" => nums::<3>(&e.words, &e.key).map(|_| ()),
                "sun" => nums::<11>(&e.words, "sun").map(|_| ()),
                "water" => nums::<6>(&e.words, "water").map(|_| ()),
                _ => Ok(()),
            };
            if let Err(m) = parsed {
                out.push(format!("line {n}: {m}"));
            }
        }
        out
    }
}

/// Names and shapes are single words in the file.
fn check_name(s: &str) -> Result<(), String> {
    if s.is_empty() || s.contains(char::is_whitespace) || s.contains('#') {
        Err(format!("`{s}` must be one word without `#`"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# A test world.
sun 0.57735 0.57735 -0.57735 0.8 0.8 0.8 1 0.3 0.3 0.35 1
sky sky_fr,sky_rt,sky_bk,sky_lf,sky_up,sky_dn 500
object house house.dif 4 4 0   # the inn
objectfor one,two door door.3ds 2 2 0  0 0 1 1.5 2 2 1
  object Tree1 tree.3ds 10.5 -3 0.25
playershape labcube.3ds
terrain world/island 4 repeat
water -1024 -1024 -19.5 2048 2048 20 water_surface 0
futurekey 1 2 3
spawn 0 0 1
";

    #[test]
    fn unchanged_round_trip() {
        assert_eq!(WorldDoc::parse(SAMPLE).text(), SAMPLE);
        let no_nl = "spawn 0 0 1";
        assert_eq!(WorldDoc::parse(no_nl).text(), no_nl);
    }

    #[test]
    fn reads_every_kind() {
        let d = WorldDoc::parse(SAMPLE);
        let objs = d.objects();
        assert_eq!(objs.len(), 3);
        assert_eq!(objs[1].only_for.as_deref(), Some("one,two"));
        assert_eq!(objs[1].turn, Some(Turn::about_z(1.5)));
        assert_eq!(objs[2].pos, [10.5, -3.0, 0.25]);
        assert_eq!(d.spawn(), Some([0.0, 0.0, 1.0]));
        assert_eq!(d.sun().unwrap().ambient[2], 0.35);
        assert_eq!(d.sky().unwrap().textures.len(), 6);
        assert_eq!(d.sky().unwrap().visible, Some(500.0));
        let t = d.terrain().unwrap();
        assert_eq!(
            (t.folder.as_str(), t.square, t.repeat),
            ("world/island", 4, true)
        );
        assert_eq!(d.waters()[0].surface(), 0.5);
        assert_eq!(
            d.problems(),
            vec!["line 10: `futurekey` is not a world file entry".to_string()]
        );
    }

    #[test]
    fn edits_touch_only_their_line() {
        let mut d = WorldDoc::parse(SAMPLE);
        let mut inn = d.object("house").unwrap();
        inn.pos = [5.0, 4.0, -0.0];
        inn.turn = Some(Turn::about_z(0.5));
        d.set_object("house", &inn).unwrap();
        let mut door = d.object("door").unwrap();
        assert_eq!(door.rest, vec!["2", "2", "1"]);
        door.pos[0] = 3.0;
        d.set_object("door", &door).unwrap();
        assert!(d
            .text()
            .contains("objectfor one,two door door.3ds 3 2 0  0 0 1 1.5 2 2 1\n"));
        let mut tree = d.object("Tree1").unwrap();
        tree.pos[2] = 1.0;
        d.set_object("Tree1", &tree).unwrap();
        let text = d.text();
        assert!(text.contains("object house house.dif 5 4 0  0 0 1 0.5   # the inn\n"));
        assert!(text.contains("\n  object Tree1 tree.3ds 10.5 -3 1\n"));
        let changed: Vec<_> = text
            .lines()
            .zip(SAMPLE.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(changed.len(), 3);
    }

    #[test]
    fn add_remove_and_singles() {
        let mut d = WorldDoc::parse(SAMPLE);
        let name = d.free_name("house");
        assert_eq!(name, "house2");
        d.add_object(&Object {
            only_for: None,
            name: name.clone(),
            shape: "house.dif".into(),
            pos: [1.0, 2.0, 3.0],
            turn: None,
            rest: Vec::new(),
        })
        .unwrap();
        // Right after the last object line.
        assert_eq!(d.lines()[6], "object house2 house.dif 1 2 3");
        assert!(d
            .add_object(&Object {
                only_for: None,
                name: "house".into(),
                shape: "x.3ds".into(),
                pos: [0.0; 3],
                turn: None,
                rest: Vec::new(),
            })
            .is_err());
        d.remove_object("door").unwrap();
        assert!(d.object("door").is_none());
        d.set_spawn([1.5, -2.0, 3.0]);
        assert_eq!(d.spawn(), Some([1.5, -2.0, 3.0]));
        assert!(d.text().ends_with("spawn 1.5 -2 3\n"));
        d.set_camera(
            [0.0, -5.0, 2.0],
            Some(Turn {
                axis: [1.0, 0.0, 0.0],
                angle: 0.38,
            }),
        );
        assert_eq!(d.lines().last().unwrap(), "camera 0 -5 2  1 0 0 0.38");
        d.set_sky(None);
        assert!(d.sky().is_none());
        let mut s = d.sun().unwrap();
        s.dir = [0.0, 0.0, -1.0];
        d.set_sun(&s);
        assert_eq!(d.sun().unwrap().dir, [0.0, 0.0, -1.0]);
        assert!(d.lines()[1].starts_with("sun 0 0 -1  0.8 0.8 0.8 1"));
    }

    #[test]
    fn problems_name_the_line() {
        let d = WorldDoc::parse(
            "object a a.3ds 0 0 x\nobject b b.3ds 0 0 0\nobject b c.3ds 1 1 1\nsun 1 2\n",
        );
        let p = d.problems();
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(p[0].starts_with("line 1: object"));
        assert!(p[1].contains("also on line 2"));
        assert!(p[2].starts_with("line 4: sun"));
    }

    #[test]
    fn numbers_are_short() {
        assert_eq!(fmt_num(1.0), "1");
        assert_eq!(fmt_num(-0.00001), "0");
        assert_eq!(fmt_num(2.50), "2.5");
        assert_eq!(fmt_num(-2.71239), "-2.7124");
    }
}
