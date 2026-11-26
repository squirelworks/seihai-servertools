//! `seihai-theme`, run as a program on its own.

use std::fs;
use std::process::Command;

#[test]
fn theme_make_writes_a_patch_theme_and_keeps_an_existing_one() {
    let d = tempfile::tempdir().unwrap();
    let mut tm = String::from("[Tiles]\n");
    for i in 0..16 {
        tm.push_str(&format!("Tile{i:02}=B{}\n", (b'A' + i) as char));
    }
    tm.push_str("[Objects]\nWidthMin=3\nWidthMax=5\nHeightMin=3\nHeightMax=5\nDensityMin=40\nDensityMax=80\nBaseObjectName=patch\n[Theme]\n#ofObjects=2\nQtyPerObject=c(50%,2)\nDescription=Patches\n");
    fs::write(d.path().join("p.tm"), tm).unwrap();
    let theme = || {
        Command::new(env!("CARGO_BIN_EXE_seihai-theme"))
            .args(["make", "p.tm", "p.nwt"])
            .current_dir(d.path())
            .output()
            .unwrap()
    };
    let out = theme();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let nwt = fs::read_to_string(d.path().join("p.nwt")).unwrap();
    assert!(nwt.contains("patch1=c(50%,2)|BA"), "{nwt}");
    let again = theme();
    assert!(
        !again.status.success(),
        "refuses to replace without --force"
    );
}

/// Every tool answers --version and --help on its own, with no other tool
/// built (tools/README.md).
#[test]
fn version_and_help_on_its_own() {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-theme"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("seihai-theme "));
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-theme"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("make"), "{text}");
}
