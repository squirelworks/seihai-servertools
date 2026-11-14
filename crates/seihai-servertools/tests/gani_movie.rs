//! `gani movie show|render`, run as a program.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seihai-servertools"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
    out
}

fn read_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

const MOVIE: &str = "GANI0002
SPRITE 0 Sheet.PNG 0 0 2 2 red
FRAMES 5
MOVIE
  ACTOR SPRITE box
    FRAME 0 sprite=0,dx=0,dy=1
    FRAME 4 dx=8
  ACTOREND

  ACTOR TEXT title
    FRAME 0 text=Hi,font=Fancy,color=#00ff00,fontsize=6,dx=0,dy=4
  ACTOREND

  ACTOR CHAR hero
    FRAME 0 ani=stand,body=sheet.png,dx=10,dy=4
  ACTOREND
MOVIEEND
";

#[test]
fn shows_and_renders_a_movie() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("pics")).unwrap();
    fs::write(
        dir.path().join("pics/sheet.png"),
        png(2, 2, &[255, 0, 0, 255].repeat(4)),
    )
    .unwrap();
    fs::write(dir.path().join("intro.gani"), MOVIE).unwrap();
    // A one-step gani that draws one body cell, the same in every direction.
    fs::write(
        dir.path().join("pics/Stand.gani"),
        "GANI0001\nSPRITE 0 BODY 0 0 1 1 b\nSINGLEDIRECTION\nANI\n0 0 0\nANIEND\n",
    )
    .unwrap();

    let out = run(&["gani", "movie", "show", "intro.gani"], dir.path());
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("5 frames, 3 actors"), "{text}");
    assert!(text.contains("SPRITE box (2 keyframes)"), "{text}");
    assert!(text.contains("  4 dx=8"), "{text}");

    let out = run(
        &["gani", "movie", "show", "intro.gani", "--frame", "2"],
        dir.path(),
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("SPRITE box: sprite=0,dx=4,dy=1"), "{text}");

    let out = run(
        &[
            "gani",
            "movie",
            "render",
            "intro.gani",
            "f2.png",
            "--frame",
            "2",
            "--size",
            "16x8",
        ],
        dir.path(),
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{err}");
    assert!(!err.contains("title (TEXT)"), "{err}");
    assert!(err.contains("font Fancy not found"), "{err}");
    let (w, h, px) = read_png(&fs::read(dir.path().join("f2.png")).unwrap());
    assert_eq!((w, h), (16, 8));
    let at = |x: u32, y: u32| &px[((y * w + x) * 4) as usize..][..4];
    assert_eq!(at(4, 1), [255, 0, 0, 255]);
    assert_eq!(at(5, 2), [255, 0, 0, 255]);
    assert_eq!(at(3, 1)[3], 0);
    assert_eq!(at(4, 0)[3], 0);
    assert_eq!(at(10, 4), [255, 0, 0, 255], "the CHAR's body cell");
    let greenish = px
        .chunks(4)
        .filter(|p| p[1] > 60 && p[0] < 60 && p[3] > 0)
        .count();
    assert!(greenish > 0, "the TEXT actor");
    assert_eq!(at(11, 4)[3], 0);
    assert!(err.contains("head0.png not found"), "{err}");

    // Past the timeline, and a plain gani, are refused.
    let out = run(
        &[
            "gani",
            "movie",
            "render",
            "intro.gani",
            "x.png",
            "--frame",
            "5",
        ],
        dir.path(),
    );
    assert!(!out.status.success());
    fs::write(
        dir.path().join("idle.gani"),
        "GANI0001\nSPRITE 0 a.png 0 0 1 1 x\nANI\n0 0 0\nANIEND\n",
    )
    .unwrap();
    let out = run(&["gani", "movie", "show", "idle.gani"], dir.path());
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a movie"));
}
