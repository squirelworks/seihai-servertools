//! Runs `seihai-asset3d` on models made here and checks what it writes.

use std::path::Path;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-asset3d"))
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = bin().args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// An 8-bit RGBA PNG, `w` x `h`, one colour.
fn png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w8 = e.write_header().unwrap();
        w8.write_image_data(&rgba.repeat((w * h) as usize)).unwrap();
    }
    out
}

/// A textured cube as OBJ + MTL, with a texture bigger than the 5.12 draws
/// and in upper case, as exporters write it.
fn cube(dir: &Path) {
    std::fs::write(
        dir.join("crate.mtl"),
        "newmtl wood\nKd 0.8 0.6 0.3\nmap_Kd textures\\Crate.PNG\n",
    )
    .unwrap();
    std::fs::write(dir.join("crate.png"), png(2048, 2048, [150, 100, 50, 255])).unwrap();
    let mut obj = String::from("mtllib crate.mtl\n");
    for (x, y, z) in [
        (0, 0, 0),
        (1, 0, 0),
        (1, 1, 0),
        (0, 1, 0),
        (0, 0, 1),
        (1, 0, 1),
        (1, 1, 1),
        (0, 1, 1),
    ] {
        obj.push_str(&format!("v {x} {y} {z}\n"));
    }
    obj.push_str("vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nusemtl wood\n");
    for f in [
        [1, 2, 3, 4],
        [5, 8, 7, 6],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [3, 7, 8, 4],
        [4, 8, 5, 1],
    ] {
        obj.push_str(&format!("f {}/1 {}/2 {}/3 {}/4\n", f[0], f[1], f[2], f[3]));
    }
    std::fs::write(dir.join("Crate.obj"), obj).unwrap();
}

#[test]
fn add_list_check_and_remove_an_asset() {
    let src = tempfile::tempdir().unwrap();
    cube(src.path());
    let world = tempfile::tempdir().unwrap();
    let w = world.path().to_str().unwrap();
    let obj = src.path().join("Crate.obj");
    let obj = obj.to_str().unwrap();

    // Not a world folder yet.
    let (ok, _, err) = run(&["add", w, obj]);
    assert!(!ok);
    assert!(err.contains("config/"), "{err}");
    std::fs::create_dir(world.path().join("config")).unwrap();

    let (ok, out, err) = run(&["add", w, obj, "--height", "2"]);
    assert!(ok, "{err}");
    assert!(out.contains("crate: crate.3ds (2 x 2 x 2 tiles"), "{out}");
    let files = world.path().join("files");
    let shape = std::fs::read(files.join("crate.3ds")).unwrap();
    assert!(shape.windows(11).any(|w| w == b"crate_0.png"));
    let tex = std::fs::read(files.join("crate_0.png")).unwrap();
    // 8-bit, opaque so RGB, and made 1024 a side.
    assert_eq!(&tex[16..24], &[0, 0, 4, 0, 0, 0, 4, 0]);
    assert_eq!(tex[24], 8);
    let man = std::fs::read_to_string(world.path().join("config/assets3d.txt")).unwrap();
    assert!(
        man.contains("asset crate crate.3ds 2 2 2 crate_0 Crate.obj"),
        "{man}"
    );

    let (ok, out, _) = run(&["list", w]);
    assert!(ok);
    assert!(out.starts_with("crate crate.3ds 2 x 2 x 2 tiles"), "{out}");

    let (ok, out, _) = run(&["check", files.join("crate.3ds").to_str().unwrap()]);
    assert!(ok, "{out}");
    assert!(
        out.contains("texture crate_0.png (asked for as `crate_0`)"),
        "{out}"
    );

    // The source texture is too big for a 5.12 as it is.
    let (ok, out, _) = run(&["check", src.path().join("crate.png").to_str().unwrap()]);
    assert!(!ok);
    assert!(out.contains("2048 x 2048"), "{out}");

    // Adding again replaces it; a file that is not the asset's is kept.
    std::fs::write(files.join("other.3ds"), b"x").unwrap();
    let (ok, _, err) = run(&["add", w, obj, "--name", "other"]);
    assert!(!ok);
    assert!(err.contains("files/other.3ds is already there"), "{err}");
    let (ok, _, err) = run(&["add", w, obj, "--colour-only"]);
    assert!(ok, "{err}");
    assert!(!files.join("crate_0.png").exists());

    let (ok, _, _) = run(&["remove", w, "crate"]);
    assert!(ok);
    assert!(!files.join("crate.3ds").exists());
    let (_, out, _) = run(&["list", w]);
    assert!(out.starts_with("No assets"), "{out}");
}

#[test]
fn pictures_become_cards_and_other_formats_say_why_not() {
    let src = tempfile::tempdir().unwrap();
    let pic = src.path().join("Lamp Post.png");
    std::fs::write(&pic, png(32, 48, [200, 200, 50, 255])).unwrap();
    let out = tempfile::tempdir().unwrap();
    let o = out.path().to_str().unwrap();
    let (ok, stdout, err) = run(&["convert", pic.to_str().unwrap(), o]);
    assert!(ok, "{err}");
    // 16 pixels a tile: 2 wide, 3 tall, flat.
    assert!(
        stdout.contains("lamppost: lamppost.3ds (2 x 0 x 3 tiles"),
        "{stdout}"
    );
    assert!(out.path().join("lamppost_0.png").exists());
    let (ok, _, err) = run(&["convert", pic.to_str().unwrap(), o]);
    assert!(!ok);
    assert!(err.contains("--force"), "{err}");

    let fbx = src.path().join("hero.fbx");
    std::fs::write(&fbx, b"Kaydara FBX Binary").unwrap();
    let (ok, _, err) = run(&["convert", fbx.to_str().unwrap(), o]);
    assert!(!ok);
    assert!(err.contains("FBX is a closed format"), "{err}");
    assert!(err.contains("glTF"), "{err}");

    let dts = src.path().join("old.dts");
    std::fs::write(&dts, 15u32.to_le_bytes()).unwrap();
    let (ok, _, err) = run(&["convert", dts.to_str().unwrap(), o]);
    assert!(!ok);
    assert!(err.contains("versions 17 to 24"), "{err}");

    let (ok, out, _) = run(&["formats"]);
    assert!(ok);
    assert!(out.contains(".glb .gltf"));
    let (ok, out, _) = run(&["--help"]);
    assert!(ok);
    assert!(out.contains("seihai-asset3d add FOLDER"));
}
