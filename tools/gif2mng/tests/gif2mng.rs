//! `seihai-gif2mng`, run as a program on its own.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gif2mng"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// A 2 x 1 GIF89a: one image of colour index `c` per entry of `frames`.
fn gif(frames: &[u8]) -> Vec<u8> {
    let mut g = b"GIF89a".to_vec();
    g.extend([2, 0, 1, 0, 0x81, 0, 0]);
    g.extend([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
    for &c in frames {
        g.extend([0x2C, 0, 0, 0, 0, 2, 0, 1, 0, 0]);
        // LZW, min code 2, 3-bit codes: clear c clear c end.
        let (mut acc, mut n, mut bytes) = (0u32, 0u32, Vec::new());
        for code in [4u32, u32::from(c), 4, u32::from(c), 5] {
            acc |= code << n;
            n += 3;
            while n >= 8 {
                bytes.push(acc as u8);
                acc >>= 8;
                n -= 8;
            }
        }
        if n > 0 {
            bytes.push(acc as u8);
        }
        g.extend([2, bytes.len() as u8]);
        g.extend(bytes);
        g.push(0);
    }
    g.push(0x3B);
    g
}

#[test]
fn gif2mng_writes_a_png_or_an_mng() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("still.gif"), gif(&[1])).unwrap();
    fs::write(d.path().join("walk.gif"), gif(&[1, 2, 3])).unwrap();
    let out = run(&["still.gif", "walk.gif", "--out", "o"], d.path());
    assert!(String::from_utf8_lossy(&out.stdout).contains("3 frames"));
    assert!(fs::read(d.path().join("o/still.png"))
        .unwrap()
        .starts_with(b"\x89PNG"));
    assert!(fs::read(d.path().join("o/walk.mng"))
        .unwrap()
        .starts_with(b"\x8aMNG"));
    let again = Command::new(env!("CARGO_BIN_EXE_seihai-gif2mng"))
        .args(["walk.gif", "--out", "o"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );
    run(&["-b", "--force", "walk.gif", "--out", "o"], d.path());
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gif2mng"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-gif2mng "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-gif2mng"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("GIF"), "{text}");
}
