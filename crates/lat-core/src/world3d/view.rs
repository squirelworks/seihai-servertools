//! The editor's built-in preview: a small software renderer for a world
//! file, its `.3ds` shapes, its streamed terrain and its water.
//!
//! It follows what the Graal3D renderer work learned about the real 5.12
//! (Graal_analysis `torque/`, project renderer notes): one Torque unit is
//! one tile, rotations turn the opposite way from the right-hand rule, a
//! water surface is at z + the z size, the sky is the fog colour toward the
//! horizon. Lighting is flat per face: the sun's colour times the angle to
//! the sun, plus the ambient colour. It is a preview for placing things,
//! not a match of the client's pixels; an external renderer can take its
//! place (see the `seihai-edit3d` README).

use super::doc::Turn;
use super::shape::Tri;
use crate::preview::Rgba;

/// Where the preview looks from. Yaw 0 looks along +y (north), a positive
/// yaw turns right (toward +x); a positive pitch looks down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: [f64; 3],
    pub yaw: f64,
    pub pitch: f64,
    /// Horizontal field of view, degrees.
    pub fov: f64,
}

impl Camera {
    /// Looking at `target` from `distance` away, at that yaw and pitch.
    pub fn orbit(target: [f64; 3], distance: f64, yaw: f64, pitch: f64) -> Camera {
        let f = forward(yaw, pitch);
        Camera {
            eye: [
                target[0] - f[0] * distance,
                target[1] - f[1] * distance,
                target[2] - f[2] * distance,
            ],
            yaw,
            pitch,
            fov: 60.0,
        }
    }

    /// The unit ray through pixel (px, py) of a `w` x `h` view.
    pub fn ray(&self, px: f64, py: f64, w: u32, h: u32) -> [f64; 3] {
        let (f, r, u) = basis(self.yaw, self.pitch);
        let fpx = f64::from(w) / 2.0 / (self.fov.to_radians() / 2.0).tan();
        let x = (px - f64::from(w) / 2.0) / fpx;
        let y = (f64::from(h) / 2.0 - py) / fpx;
        norm([
            f[0] + r[0] * x + u[0] * y,
            f[1] + r[1] * x + u[1] * y,
            f[2] + r[2] * x + u[2] * y,
        ])
    }
}

fn forward(yaw: f64, pitch: f64) -> [f64; 3] {
    [
        yaw.sin() * pitch.cos(),
        yaw.cos() * pitch.cos(),
        -pitch.sin(),
    ]
}

/// Forward, right and up.
fn basis(yaw: f64, pitch: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let f = forward(yaw, pitch);
    let r = [yaw.cos(), -yaw.sin(), 0.0];
    let u = cross(r, f);
    (f, r, u)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt();
    if l == 0.0 {
        a
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

/// Rotate `p` by a world-file turn (the clients' sense: the right-hand rule
/// with the angle negated).
pub fn turn_point(p: [f64; 3], t: Turn) -> [f64; 3] {
    let k = norm(t.axis);
    if dot(k, k) == 0.0 {
        return p;
    }
    let (s, c) = (-t.angle).sin_cos();
    let kxp = cross(k, p);
    let kp = dot(k, p);
    [
        p[0] * c + kxp[0] * s + k[0] * kp * (1.0 - c),
        p[1] * c + kxp[1] * s + k[1] * kp * (1.0 - c),
        p[2] * c + kxp[2] * s + k[2] * kp * (1.0 - c),
    ]
}

/// One triangle in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub p: [[f64; 3]; 3],
    pub rgb: [u8; 3],
    /// Water: drawn see-through, after everything solid.
    pub water: bool,
    /// The object it belongs to (an index into the caller's list), for
    /// picking; `None` for terrain and water.
    pub object: Option<usize>,
}

/// Everything to draw, already in world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub faces: Vec<Face>,
    /// Toward the light is `-sun_dir`.
    pub sun_dir: [f64; 3],
    pub sun: [f64; 3],
    pub ambient: [f64; 3],
    /// The horizon colour (the sky line's fog, else a pale blue).
    pub sky: [u8; 3],
}

impl Scene {
    pub fn new(sun: &super::doc::Sun, fog: Option<[f64; 3]>) -> Scene {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let fog = fog.unwrap_or([0.6, 0.7, 0.9]);
        Scene {
            faces: Vec::new(),
            sun_dir: norm(sun.dir),
            sun: [sun.colour[0], sun.colour[1], sun.colour[2]],
            ambient: [sun.ambient[0], sun.ambient[1], sun.ambient[2]],
            sky: [c(fog[0]), c(fog[1]), c(fog[2])],
        }
    }

    /// Add a shape's triangles placed at `pos`, turned by `turn`.
    pub fn add_shape(&mut self, tris: &[Tri], pos: [f64; 3], turn: Option<Turn>, object: usize) {
        for t in tris {
            let p = t.p.map(|v| {
                let v = [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])];
                let v = turn.map_or(v, |tn| turn_point(v, tn));
                [v[0] + pos[0], v[1] + pos[1], v[2] + pos[2]]
            });
            self.faces.push(Face {
                p,
                rgb: t.rgb,
                water: false,
                object: Some(object),
            });
        }
    }

    /// A box standing on `pos`, for shapes the preview cannot read.
    pub fn add_marker(&mut self, pos: [f64; 3], rgb: [u8; 3], object: usize) {
        let (h, s) = (1.0, 0.5);
        let c = |x: f64, y: f64, z: f64| [pos[0] + x, pos[1] + y, pos[2] + z];
        let v = [
            c(-s, -s, 0.0),
            c(s, -s, 0.0),
            c(s, s, 0.0),
            c(-s, s, 0.0),
            c(-s, -s, h),
            c(s, -s, h),
            c(s, s, h),
            c(-s, s, h),
        ];
        for q in [
            [0, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ] {
            for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
                self.faces.push(Face {
                    p: t.map(|i| v[i]),
                    rgb,
                    water: false,
                    object: Some(object),
                });
            }
        }
    }

    /// The terrain's squares, every `step`th one (1 is all 65,025 quads).
    pub fn add_terrain(&mut self, f: &super::terrain::Field, colours: &[[u8; 3]], step: usize) {
        let step = step.clamp(1, 16);
        let p = |i: usize, j: usize| {
            let (x, y) = f.square_pos(i, j);
            [x, y, f.height(i, j)]
        };
        let mut j = 0;
        while j + step < super::terrain::SIZE_SQUARES {
            let mut i = 0;
            while i + step < super::terrain::SIZE_SQUARES {
                let m = f.materials[j][i] as usize;
                let rgb = colours.get(m).copied().unwrap_or([120, 150, 90]);
                let (a, b, c, d) = (
                    p(i, j),
                    p(i + step, j),
                    p(i + step, j + step),
                    p(i, j + step),
                );
                for t in [[a, b, c], [a, c, d]] {
                    self.faces.push(Face {
                        p: t,
                        rgb,
                        water: false,
                        object: None,
                    });
                }
                i += step;
            }
            j += step;
        }
    }

    /// A water block's surface.
    pub fn add_water(&mut self, w: &super::doc::Water) {
        let z = w.surface();
        let (x0, y0) = (w.corner[0], w.corner[1]);
        let (x1, y1) = (x0 + w.size[0], y0 + w.size[1]);
        let rgb = if w.liquid == Some(4) {
            [220, 80, 20]
        } else {
            [40, 90, 200]
        };
        for t in [
            [[x0, y0, z], [x1, y0, z], [x1, y1, z]],
            [[x0, y0, z], [x1, y1, z], [x0, y1, z]],
        ] {
            self.faces.push(Face {
                p: t,
                rgb,
                water: true,
                object: None,
            });
        }
    }
}

/// What the renderer gives back: the picture and, per pixel, the object
/// drawn there (for picking).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub image: Rgba,
    /// `objects[y * width + x]`: the object index + 1, 0 for none.
    pub objects: Vec<u32>,
}

const NEAR: f64 = 0.05;

/// Draw `scene` from `cam` into a `w` x `h` frame.
pub fn render(scene: &Scene, cam: &Camera, w: u32, h: u32) -> Frame {
    let (wu, hu) = (w as usize, h as usize);
    let mut px = vec![0u8; wu * hu * 4];
    // Sky: the horizon colour, lighter upward, darker below the horizon.
    for y in 0..hu {
        let ray = cam.ray(wu as f64 / 2.0, y as f64 + 0.5, w, h);
        let up = ray[2];
        let rgb = if up >= 0.0 {
            scene
                .sky
                .map(|c| (f64::from(c) + (255.0 - f64::from(c)) * up * 0.6) as u8)
        } else {
            scene.sky.map(|c| (f64::from(c) * (1.0 + up * 0.7)) as u8)
        };
        for x in 0..wu {
            let i = (y * wu + x) * 4;
            px[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    let mut depth = vec![f64::INFINITY; wu * hu];
    let mut objects = vec![0u32; wu * hu];
    let (f, r, u) = basis(cam.yaw, cam.pitch);
    let fpx = f64::from(w) / 2.0 / (cam.fov.to_radians() / 2.0).tan();
    let view = |p: [f64; 3]| {
        let d = sub(p, cam.eye);
        [dot(d, r), dot(d, u), dot(d, f)]
    };
    let light = norm([-scene.sun_dir[0], -scene.sun_dir[1], -scene.sun_dir[2]]);
    for pass_water in [false, true] {
        for face in scene.faces.iter().filter(|fc| fc.water == pass_water) {
            let n = norm(cross(sub(face.p[1], face.p[0]), sub(face.p[2], face.p[0])));
            // Two-sided: the side facing the camera is lit.
            let to_eye = sub(cam.eye, face.p[0]);
            let n = if dot(n, to_eye) < 0.0 {
                n.map(|v| -v)
            } else {
                n
            };
            let lambert = dot(n, light).max(0.0);
            let shade = |k: usize| {
                let v = f64::from(face.rgb[k]) * (scene.ambient[k] + scene.sun[k] * lambert);
                v.clamp(0.0, 255.0)
            };
            let rgb = [shade(0), shade(1), shade(2)];
            let vs: Vec<[f64; 3]> = face.p.iter().map(|&p| view(p)).collect();
            // Clip against the near plane.
            let mut poly: Vec<[f64; 3]> = Vec::with_capacity(4);
            for k in 0..3 {
                let (a, b) = (vs[k], vs[(k + 1) % 3]);
                if a[2] >= NEAR {
                    poly.push(a);
                }
                if (a[2] >= NEAR) != (b[2] >= NEAR) {
                    let t = (NEAR - a[2]) / (b[2] - a[2]);
                    poly.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, NEAR]);
                }
            }
            if poly.len() < 3 {
                continue;
            }
            let scr: Vec<[f64; 3]> = poly
                .iter()
                .map(|v| {
                    [
                        f64::from(w) / 2.0 + v[0] / v[2] * fpx,
                        f64::from(h) / 2.0 - v[1] / v[2] * fpx,
                        1.0 / v[2],
                    ]
                })
                .collect();
            for k in 1..scr.len() - 1 {
                fill([scr[0], scr[k], scr[k + 1]], (wu, hu), |x, y, inv_z| {
                    let i = y * wu + x;
                    let z = 1.0 / inv_z;
                    if z >= depth[i] {
                        return;
                    }
                    let o = i * 4;
                    if pass_water {
                        for c in 0..3 {
                            px[o + c] = (f64::from(px[o + c]) * 0.4 + rgb[c] * 0.6) as u8;
                        }
                    } else {
                        depth[i] = z;
                        objects[i] = face.object.map_or(0, |n| n as u32 + 1);
                        for c in 0..3 {
                            px[o + c] = rgb[c] as u8;
                        }
                    }
                });
            }
        }
    }
    Frame {
        image: Rgba {
            width: w,
            height: h,
            pixels: px,
        },
        objects,
    }
}

/// Fill a screen triangle (x, y, 1/z), calling `put` per covered pixel with
/// the interpolated 1/z.
fn fill(t: [[f64; 3]; 3], (w, h): (usize, usize), mut put: impl FnMut(usize, usize, f64)) {
    let area =
        (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]);
    if area.abs() < 1e-9 {
        return;
    }
    let min_x = t
        .iter()
        .map(|p| p[0])
        .fold(f64::MAX, f64::min)
        .floor()
        .max(0.0) as usize;
    let max_x = t
        .iter()
        .map(|p| p[0])
        .fold(f64::MIN, f64::max)
        .ceil()
        .min(w as f64 - 1.0);
    let min_y = t
        .iter()
        .map(|p| p[1])
        .fold(f64::MAX, f64::min)
        .floor()
        .max(0.0) as usize;
    let max_y = t
        .iter()
        .map(|p| p[1])
        .fold(f64::MIN, f64::max)
        .ceil()
        .min(h as f64 - 1.0);
    if max_x < 0.0 || max_y < 0.0 {
        return;
    }
    let (max_x, max_y) = (max_x as usize, max_y as usize);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (sx, sy) = (x as f64 + 0.5, y as f64 + 0.5);
            let e = |a: [f64; 3], b: [f64; 3]| {
                (b[0] - a[0]) * (sy - a[1]) - (b[1] - a[1]) * (sx - a[0])
            };
            let w0 = e(t[1], t[2]) / area;
            let w1 = e(t[2], t[0]) / area;
            let w2 = e(t[0], t[1]) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            put(x, y, w0 * t[0][2] + w1 * t[1][2] + w2 * t[2][2]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world3d::doc::Sun;

    #[test]
    fn camera_conventions() {
        let (f, r, u) = basis(0.0, 0.0);
        assert_eq!((f, r), ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]));
        assert!((u[2] - 1.0).abs() < 1e-12);
        // A positive yaw turns right: forward swings toward +x.
        assert!(forward(0.3, 0.0)[0] > 0.0);
        // A positive pitch looks down.
        assert!(forward(0.0, 0.3)[2] < 0.0);
        // The clients' turn: positive about +z swings +y toward +x.
        let p = turn_point([0.0, 1.0, 0.0], Turn::about_z(std::f64::consts::FRAC_PI_2));
        assert!((p[0] - 1.0).abs() < 1e-9 && p[1].abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn draws_a_marker_and_picks_it() {
        let mut s = Scene::new(&Sun::default(), None);
        s.add_marker([0.0, 5.0, 0.0], [200, 0, 0], 3);
        let cam = Camera {
            eye: [0.0, 0.0, 0.5],
            yaw: 0.0,
            pitch: 0.0,
            fov: 60.0,
        };
        let fr = render(&s, &cam, 64, 48);
        let mid = (24 * 64 + 32) as usize;
        assert_eq!(fr.objects[mid], 4);
        let p = fr.image.pixel(32, 24).unwrap();
        assert!(p[0] > p[1] && p[0] > p[2], "{p:?}");
        // The corner is sky.
        assert_eq!(fr.objects[0], 0);
    }

    #[test]
    fn behind_the_camera_is_clipped() {
        let mut s = Scene::new(&Sun::default(), None);
        s.add_marker([0.0, -5.0, 0.0], [200, 0, 0], 0);
        let cam = Camera {
            eye: [0.0, 0.0, 0.5],
            yaw: 0.0,
            pitch: 0.0,
            fov: 60.0,
        };
        assert!(render(&s, &cam, 32, 32).objects.iter().all(|&o| o == 0));
    }
}
