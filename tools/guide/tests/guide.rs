//! `seihai-guide`, run as a program on its own.

use std::process::Command;

fn guide(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_seihai-guide"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn version_and_help() {
    let (code, text) = guide(&["--version"]);
    assert_eq!(code, Some(0));
    assert!(text.starts_with("seihai-guide "));
    let (code, text) = guide(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(text.contains("troubleshooting"));
}

#[test]
fn topics_print() {
    let (code, text) = guide(&[]);
    assert_eq!(code, Some(0));
    assert!(text.contains("HOW A SEIHAI SERVER FITS TOGETHER"));
    for (topic, heading) in [
        ("rc", "REMOTE CONTROL"),
        ("setup", "setup new"),
        ("troubleshooting", "TROUBLESHOOTING"),
        ("tools", "EVERY TOOL"),
    ] {
        let (code, text) = guide(&[topic]);
        assert_eq!(code, Some(0), "{topic}");
        assert!(text.contains(heading), "{topic}");
    }
    let (_, all) = guide(&["all"]);
    assert!(all.contains("REMOTE CONTROL") && all.contains("THE NPC-SERVER"));
    let (code, _) = guide(&["nope"]);
    assert_eq!(code, Some(2));
}
