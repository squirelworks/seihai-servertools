//! Reading glTF 2.0 (`.gltf` with its buffers, or a binary `.glb`): the
//! meshes of the default scene placed by their node transforms, with each
//! material's base colour and base colour texture. Skins, morph targets
//! and animations are dropped, so a rigged model comes as it rests.
//!
//! Written from the Khronos glTF 2.0 specification.

use super::{texture, Face, FileLoader, Material, Model, Up, Vertex};
use serde_json::Value;
use std::collections::HashMap;

type Mat4 = [[f32; 4]; 4];

const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// `a * b`, column-major as glTF writes matrices (`m[col][row]`).
fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0; 4]; 4];
    for (c, col) in r.iter_mut().enumerate() {
        for (row, out) in col.iter_mut().enumerate() {
            *out = (0..4).map(|k| a[k][row] * b[c][k]).sum();
        }
    }
    r
}

fn apply(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for (row, o) in out.iter_mut().enumerate() {
        *o = m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row];
    }
    out
}

fn nums(v: &Value) -> Vec<f32> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_f64())
                .map(|x| x as f32)
                .collect()
        })
        .unwrap_or_default()
}

/// A node's own transform.
fn local(node: &Value) -> Mat4 {
    let m = nums(&node["matrix"]);
    if m.len() == 16 {
        let mut r = [[0.0; 4]; 4];
        for (i, v) in m.iter().enumerate() {
            r[i / 4][i % 4] = *v;
        }
        return r;
    }
    let t = nums(&node["translation"]);
    let q = nums(&node["rotation"]);
    let s = nums(&node["scale"]);
    let t = if t.len() == 3 {
        [t[0], t[1], t[2]]
    } else {
        [0.0; 3]
    };
    let s = if s.len() == 3 {
        [s[0], s[1], s[2]]
    } else {
        [1.0; 3]
    };
    let [x, y, z, w] = if q.len() == 4 {
        [q[0], q[1], q[2], q[3]]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    // Rotation matrix of the unit quaternion, columns scaled.
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + z * w),
            2.0 * (x * z - y * w),
        ],
        [
            2.0 * (x * y - z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + x * w),
        ],
        [
            2.0 * (x * z + y * w),
            2.0 * (y * z - x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let mut out = IDENTITY;
    for c in 0..3 {
        for row in 0..3 {
            out[c][row] = r[c][row] * s[c];
        }
    }
    out[3] = [t[0], t[1], t[2], 1.0];
    out
}

fn base64(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("bad base64 in a data: URI".into()),
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

fn uri_bytes(uri: &str, files: &FileLoader) -> Result<Vec<u8>, String> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (_, data) = rest
            .split_once(";base64,")
            .ok_or("a data: URI that is not base64")?;
        return base64(data);
    }
    files.load(&percent_decode(uri))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

struct Doc<'a> {
    json: Value,
    buffers: Vec<Vec<u8>>,
    files: &'a FileLoader,
}

impl Doc<'_> {
    fn view(&self, i: usize) -> Result<(&[u8], usize), String> {
        let v = &self.json["bufferViews"][i];
        let buf = v["buffer"].as_u64().ok_or("a bufferView with no buffer")? as usize;
        let off = v["byteOffset"].as_u64().unwrap_or(0) as usize;
        let len = v["byteLength"]
            .as_u64()
            .ok_or("a bufferView with no length")? as usize;
        let stride = v["byteStride"].as_u64().unwrap_or(0) as usize;
        let data = self
            .buffers
            .get(buf)
            .and_then(|b| b.get(off..off + len))
            .ok_or("a bufferView past the end of its buffer")?;
        Ok((data, stride))
    }

    /// An accessor's values as floats, `n` components an element
    /// (normalised integers become 0..1 or -1..1).
    fn floats(&self, i: usize) -> Result<Vec<Vec<f32>>, String> {
        let a = &self.json["accessors"][i];
        let count = a["count"].as_u64().ok_or("an accessor with no count")? as usize;
        let n = match a["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            _ => return Err("an accessor of a matrix type".into()),
        };
        let ct = a["componentType"].as_u64().unwrap_or(5126);
        let size = match ct {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err(format!("an accessor of component type {ct}")),
        };
        let norm = a["normalized"].as_bool().unwrap_or(false);
        let Some(view) = a["bufferView"].as_u64() else {
            // No view: all zeros (sparse values are not read).
            return Ok(vec![vec![0.0; n]; count]);
        };
        let (data, stride) = self.view(view as usize)?;
        let off = a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let stride = if stride == 0 { n * size } else { stride };
        let mut out = Vec::with_capacity(count);
        for e in 0..count {
            let mut el = Vec::with_capacity(n);
            for k in 0..n {
                let at = off + e * stride + k * size;
                let b = data
                    .get(at..at + size)
                    .ok_or("an accessor past the end of its bufferView")?;
                let v = match ct {
                    5120 => {
                        let x = b[0] as i8 as f32;
                        if norm {
                            (x / 127.0).max(-1.0)
                        } else {
                            x
                        }
                    }
                    5121 => {
                        let x = b[0] as f32;
                        if norm {
                            x / 255.0
                        } else {
                            x
                        }
                    }
                    5122 => {
                        let x = i16::from_le_bytes([b[0], b[1]]) as f32;
                        if norm {
                            (x / 32767.0).max(-1.0)
                        } else {
                            x
                        }
                    }
                    5123 => {
                        let x = u16::from_le_bytes([b[0], b[1]]) as f32;
                        if norm {
                            x / 65535.0
                        } else {
                            x
                        }
                    }
                    5125 => u32::from_le_bytes(b.try_into().unwrap()) as f32,
                    _ => f32::from_le_bytes(b.try_into().unwrap()),
                };
                el.push(v);
            }
            out.push(el);
        }
        Ok(out)
    }

    fn indices(&self, i: usize) -> Result<Vec<u32>, String> {
        let a = &self.json["accessors"][i];
        let count = a["count"].as_u64().ok_or("an accessor with no count")? as usize;
        let ct = a["componentType"].as_u64().unwrap_or(5125);
        let size = match ct {
            5121 => 1,
            5123 => 2,
            5125 => 4,
            _ => return Err(format!("indices of component type {ct}")),
        };
        let view = a["bufferView"]
            .as_u64()
            .ok_or("indices with no bufferView")?;
        let (data, stride) = self.view(view as usize)?;
        let off = a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let stride = if stride == 0 { size } else { stride };
        (0..count)
            .map(|e| {
                let at = off + e * stride;
                let b = data.get(at..at + size).ok_or("indices past the end")?;
                Ok(match size {
                    1 => b[0] as u32,
                    2 => u16::from_le_bytes([b[0], b[1]]) as u32,
                    _ => u32::from_le_bytes(b.try_into().unwrap()),
                })
            })
            .collect()
    }

    fn image(&self, i: usize) -> Result<crate::preview::Rgba, String> {
        let img = &self.json["images"][i];
        let bytes = if let Some(uri) = img["uri"].as_str() {
            uri_bytes(uri, self.files)?
        } else if let Some(v) = img["bufferView"].as_u64() {
            self.view(v as usize)?.0.to_vec()
        } else {
            return Err(format!("image {i} has no data"));
        };
        texture::decode(&bytes)
    }
}

/// Read a `.gltf` (JSON); `files` finds its buffers and images.
pub fn read_gltf(bytes: &[u8], files: &FileLoader) -> Result<Model, String> {
    let json: Value = serde_json::from_slice(bytes).map_err(|e| format!("glTF JSON: {e}"))?;
    read_doc(json, None, files)
}

/// Read a binary `.glb`.
pub fn read_glb(bytes: &[u8], files: &FileLoader) -> Result<Model, String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("not a .glb file".into());
    }
    let u = |at: usize| -> Option<usize> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    let version = u(4).unwrap_or(0);
    if version != 2 {
        return Err(format!("a version {version} .glb (only glTF 2.0 is read)"));
    }
    let mut at = 12;
    let mut json = None;
    let mut bin = None;
    while let (Some(len), Some(kind)) = (u(at), u(at + 4)) {
        let body = bytes
            .get(at + 8..at + 8 + len)
            .ok_or("a .glb chunk past the end")?;
        match kind {
            0x4E4F_534A => json = Some(body),
            0x004E_4942 => bin = Some(body.to_vec()),
            _ => {}
        }
        at += 8 + len;
    }
    let json: Value = serde_json::from_slice(json.ok_or("a .glb with no JSON chunk")?)
        .map_err(|e| format!("glTF JSON: {e}"))?;
    read_doc(json, bin, files)
}

fn read_doc(json: Value, bin: Option<Vec<u8>>, files: &FileLoader) -> Result<Model, String> {
    let mut m = Model::new(Up::Y);
    let mut bin = bin;
    let mut buffers = Vec::new();
    for (i, b) in json["buffers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let data = match b["uri"].as_str() {
            Some(uri) => uri_bytes(uri, files).map_err(|e| format!("buffer {i}: {e}"))?,
            None => bin.take().ok_or(format!("buffer {i} has no data"))?,
        };
        buffers.push(data);
    }
    let required: Vec<String> = json["extensionsRequired"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_str().map(str::to_string))
        .collect();
    if let Some(e) = required.iter().find(|e| {
        matches!(
            e.as_str(),
            "KHR_draco_mesh_compression" | "EXT_meshopt_compression"
        )
    }) {
        return Err(format!(
            "the file is compressed with {e}, which is not read (export it again \
             without mesh compression)"
        ));
    }
    for e in required.iter().filter(|e| *e != "KHR_mesh_quantization") {
        m.notes.push(format!(
            "glTF extension {e} is not read; the file is read without it"
        ));
    }
    let doc = Doc {
        json,
        buffers,
        files,
    };
    if doc.json["skins"].as_array().is_some_and(|a| !a.is_empty()) {
        m.notes
            .push("skins dropped: the mesh is taken as it rests".into());
    }
    if doc.json["animations"]
        .as_array()
        .is_some_and(|a| !a.is_empty())
    {
        m.notes
            .push("animations dropped: shapes do not animate on the 5.12".into());
    }
    // Nodes to walk: the default scene's, else every root node.
    let nodes = doc.json["nodes"].as_array().cloned().unwrap_or_default();
    let scene = doc.json["scene"].as_u64().unwrap_or(0) as usize;
    let roots: Vec<usize> = match doc.json["scenes"].get(scene) {
        Some(s) => s["nodes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64())
                    .map(|v| v as usize)
                    .collect()
            })
            .unwrap_or_default(),
        None => {
            let mut child = vec![false; nodes.len()];
            for n in &nodes {
                for c in n["children"].as_array().into_iter().flatten() {
                    if let Some(c) = c.as_u64().and_then(|c| child.get_mut(c as usize)) {
                        *c = true;
                    }
                }
            }
            (0..nodes.len()).filter(|i| !child[*i]).collect()
        }
    };
    let mut materials: HashMap<Option<usize>, usize> = HashMap::new();
    let mut images: HashMap<usize, Option<usize>> = HashMap::new();
    let mut stack: Vec<(usize, Mat4, usize)> =
        roots.into_iter().map(|r| (r, IDENTITY, 0)).collect();
    while let Some((ni, parent, depth)) = stack.pop() {
        if depth > 64 {
            return Err("glTF nodes nest too deep (a loop?)".into());
        }
        let Some(node) = nodes.get(ni) else { continue };
        let world = mul(&parent, &local(node));
        if let Some(mesh) = node["mesh"].as_u64() {
            mesh_into(
                &doc,
                mesh as usize,
                &world,
                &mut m,
                &mut materials,
                &mut images,
            )?;
        }
        for c in node["children"].as_array().into_iter().flatten() {
            if let Some(c) = c.as_u64() {
                stack.push((c as usize, world, depth + 1));
            }
        }
    }
    if m.faces.is_empty() {
        return Err("the glTF has no triangles in its scene".into());
    }
    Ok(m)
}

fn material_into(
    doc: &Doc,
    mi: Option<usize>,
    m: &mut Model,
    images: &mut HashMap<usize, Option<usize>>,
) -> usize {
    let Some(i) = mi else {
        return m.colour_material([200, 200, 200]);
    };
    let mat = &doc.json["materials"][i];
    let pbr = &mat["pbrMetallicRoughness"];
    let f = nums(&pbr["baseColorFactor"]);
    let factor = if f.len() >= 3 {
        [f[0], f[1], f[2]]
    } else {
        [1.0; 3]
    };
    let mut colour = factor.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    let mut texture = None;
    if let Some(t) = pbr["baseColorTexture"]["index"].as_u64() {
        if pbr["baseColorTexture"]["texCoord"].as_u64().unwrap_or(0) != 0 {
            m.notes
                .push("a texture on a second UV set is placed with the first".into());
        }
        if let Some(src) = doc.json["textures"][t as usize]["source"].as_u64() {
            texture = *images.entry(src as usize).or_insert_with(|| {
                match doc.image(src as usize) {
                    Ok(mut img) => {
                        // The base colour factor tints the texture.
                        if factor != [1.0; 3] {
                            for p in img.pixels.as_chunks_mut::<4>().0.iter_mut() {
                                for k in 0..3 {
                                    p[k] = (p[k] as f32 * factor[k].clamp(0.0, 1.0)) as u8;
                                }
                            }
                        }
                        m.textures.push(img);
                        Some(m.textures.len() - 1)
                    }
                    Err(e) => {
                        m.notes
                            .push(format!("texture {e}; the base colour is used"));
                        None
                    }
                }
            });
        } else {
            m.notes.push(
                "a texture in a format this reader does not know (e.g. KTX2 or WebP) was \
                 dropped; the base colour is used"
                    .into(),
            );
        }
    }
    if let Some(t) = texture {
        colour = texture::average(&m.textures[t]);
    }
    let name = mat["name"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("material{i}"));
    m.materials.push(Material {
        name,
        colour,
        texture,
    });
    m.materials.len() - 1
}

fn mesh_into(
    doc: &Doc,
    mesh: usize,
    world: &Mat4,
    m: &mut Model,
    materials: &mut HashMap<Option<usize>, usize>,
    images: &mut HashMap<usize, Option<usize>>,
) -> Result<(), String> {
    let prims = doc.json["meshes"][mesh]["primitives"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for p in prims {
        let mode = p["mode"].as_u64().unwrap_or(4);
        if mode < 4 {
            m.notes.push("points and lines skipped".into());
            continue;
        }
        let Some(pos) = p["attributes"]["POSITION"].as_u64() else {
            continue;
        };
        let positions = doc.floats(pos as usize)?;
        let uvs = match p["attributes"]["TEXCOORD_0"].as_u64() {
            Some(t) => doc.floats(t as usize)?,
            None => Vec::new(),
        };
        let key = p["material"].as_u64().map(|v| v as usize);
        let mi = match materials.get(&key) {
            Some(i) => *i,
            None => {
                let i = material_into(doc, key, m, images);
                materials.insert(key, i);
                i
            }
        };
        let base = m.vertices.len() as u32;
        for (k, q) in positions.iter().enumerate() {
            let uv = uvs.get(k).map_or([0.0, 0.0], |t| [t[0], 1.0 - t[1]]);
            m.vertices.push(Vertex {
                p: apply(world, [q[0], q[1], q[2]]),
                uv,
            });
        }
        let idx: Vec<u32> = match p["indices"].as_u64() {
            Some(i) => doc.indices(i as usize)?,
            None => (0..positions.len() as u32).collect(),
        };
        let tris: Vec<[u32; 3]> = match mode {
            4 => idx.as_chunks::<3>().0.to_vec(),
            5 => (2..idx.len())
                .map(|k| {
                    if k % 2 == 0 {
                        [idx[k - 2], idx[k - 1], idx[k]]
                    } else {
                        [idx[k - 1], idx[k - 2], idx[k]]
                    }
                })
                .collect(),
            _ => (2..idx.len())
                .map(|k| [idx[0], idx[k - 1], idx[k]])
                .collect(),
        };
        for t in tris {
            if t.iter().any(|&v| v as usize >= positions.len()) {
                return Err("a glTF index past the end of its points".into());
            }
            m.faces.push(Face {
                v: t.map(|v| v + base),
                m: mi,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-triangle glTF with its buffer as a data: URI, under a node
    /// moved by 10 on x and a material with a red factor.
    pub fn triangle_gltf() -> Vec<u8> {
        let mut buf = Vec::new();
        for v in [0f32, 0., 0., 1., 0., 0., 0., 1., 0.] {
            buf.extend(v.to_le_bytes());
        }
        for v in [0f32, 0., 1., 0., 0., 1.] {
            buf.extend(v.to_le_bytes());
        }
        let b64 = {
            const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut s = String::new();
            for c in buf.chunks(3) {
                let n = (c[0] as u32) << 16
                    | (*c.get(1).unwrap_or(&0) as u32) << 8
                    | *c.get(2).unwrap_or(&0) as u32;
                for k in 0..4 {
                    if k <= c.len() {
                        s.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
                    } else {
                        s.push('=');
                    }
                }
            }
            s
        };
        format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"mesh":0,"translation":[10,0,0]}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"material":0}}]}}],
            "materials":[{{"name":"red","pbrMetallicRoughness":{{"baseColorFactor":[1,0,0,1]}}}}],
            "buffers":[{{"byteLength":{len},"uri":"data:application/octet-stream;base64,{b64}"}}],
            "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":36}},{{"buffer":0,"byteOffset":36,"byteLength":24}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}},
                         {{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}}]}}"#,
            len = buf.len()
        )
        .into_bytes()
    }

    fn loader() -> FileLoader {
        FileLoader {
            dir: std::env::temp_dir(),
        }
    }

    #[test]
    fn reads_a_node_moved_triangle_with_its_colour() {
        let m = read_gltf(&triangle_gltf(), &loader()).unwrap();
        assert_eq!(m.faces.len(), 1);
        assert_eq!(m.vertices[1].p, [11.0, 0.0, 0.0]);
        // v is flipped to count from the bottom.
        assert_eq!(m.vertices[0].uv, [0.0, 1.0]);
        assert_eq!(m.vertices[1].uv, [1.0, 1.0]);
        assert_eq!(m.materials[0].colour, [255, 0, 0]);
    }

    #[test]
    fn reads_the_same_as_a_glb() {
        let json = triangle_gltf();
        let mut json = json;
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut glb = b"glTF".to_vec();
        glb.extend(2u32.to_le_bytes());
        glb.extend(((12 + 8 + json.len()) as u32).to_le_bytes());
        glb.extend((json.len() as u32).to_le_bytes());
        glb.extend(0x4E4F_534Au32.to_le_bytes());
        glb.extend(&json);
        let m = read_glb(&glb, &loader()).unwrap();
        assert_eq!(m.faces.len(), 1);
        assert!(read_glb(b"nope", &loader()).is_err());
    }

    #[test]
    fn rotations_and_matrices_compose() {
        // A quarter turn about +y takes +x to -z.
        let node: Value =
            serde_json::from_str(r#"{"rotation":[0,0.70710677,0,0.70710677]}"#).unwrap();
        let p = apply(&local(&node), [1.0, 0.0, 0.0]);
        assert!((p[0]).abs() < 1e-5 && (p[2] + 1.0).abs() < 1e-5, "{p:?}");
        let t: Value = serde_json::from_str(r#"{"translation":[0,5,0]}"#).unwrap();
        let p = apply(&mul(&local(&t), &local(&node)), [1.0, 0.0, 0.0]);
        assert!(
            (p[1] - 5.0).abs() < 1e-5 && (p[2] + 1.0).abs() < 1e-5,
            "{p:?}"
        );
    }
}
