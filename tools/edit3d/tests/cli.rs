//! seihai-edit3d run as a program, on a world folder made with servertools'
//! own 3D generator (no client content).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_seihai-edit3d"))
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().unwrap()
}

fn ok(args: &[&str]) -> String {
    let o = run(args);
    assert!(
        o.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

/// `gen 3d --stream --playerworld`'s layout: the world file in config/,
/// shapes and the terrain folder under world/.
fn world(dir: &Path) {
    let o = lat_core::generate::TerrainOptions {
        world: "isle".into(),
        width: 2,
        height: 2,
        seed: 7,
        base: 0,
        amplitude: 64.0,
        chaos: 0.5,
        even_borders: true,
    };
    let o3 = lat_core::terrain3d::Terrain3dOptions {
        stream: true,
        ..Default::default()
    };
    for (name, bytes) in lat_core::terrain3d::terrain3d(&o, &o3).unwrap() {
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
fn version_and_help() {
    assert!(ok(&["--version"]).starts_with("seihai-edit3d "));
    assert!(ok(&["--help"]).contains("seihai-edit3d terrain FOLDER"));
    assert!(!run(&[]).status.success());
}

#[test]
fn show_edit_and_render() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().to_str().unwrap();
    world(d.path());
    let original = fs::read_to_string(d.path().join("config/graal3dworld.txt")).unwrap();

    assert_eq!(ok(&["worlds", root]), "default\n");
    let shown = ok(&["show", root]);
    assert!(shown.contains("terrain world/isle_terrain"), "{shown}");
    assert!(shown.contains("0 chunks missing"), "{shown}");
    assert!(!shown.contains("problem:"), "{shown}");

    // Add an object standing on the ground, turned 90 degrees.
    let out = ok(&[
        "add",
        root,
        "Hut",
        "isle_rock.3ds",
        "10",
        "-10",
        "0",
        "--ground",
        "--turn",
        "90",
    ]);
    assert!(out.contains("backup "), "{out}");
    let text = fs::read_to_string(d.path().join("config/graal3dworld.txt")).unwrap();
    let hut = text.lines().find(|l| l.starts_with("object Hut ")).unwrap();
    assert!(hut.ends_with("0 0 1 1.5708"), "{hut}");
    // Everything else as it was, in order.
    let rest: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("object Hut "))
        .collect();
    assert_eq!(rest, original.lines().collect::<Vec<_>>());
    // A backup of the original sits beside it.
    let backups: Vec<_> = fs::read_dir(d.path().join("config"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("graal3dworld.txt.bak-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(backups[0].path()).unwrap(), original);

    ok(&["move", root, "Hut", "12", "-10", "3"]);
    assert!(fs::read_to_string(d.path().join("config/graal3dworld.txt"))
        .unwrap()
        .contains("object Hut isle_rock.3ds 12 -10 3  0 0 1 1.5708"));
    let dup = run(&["add", root, "Hut", "x.3ds", "0", "0", "0"]);
    assert!(!dup.status.success());
    ok(&["remove", root, "Hut"]);
    ok(&["spawn", root, "5", "-5", "1", "--ground"]);

    // Raise the terrain at the spawn; the chunks are rewritten and backed up.
    let said = ok(&["terrain", root, "raise", "5", "-5", "24", "6"]);
    assert!(
        said.contains("squares changed") && said.contains("terrain backup"),
        "{said}"
    );
    let shown = ok(&["show", root]);
    assert!(shown.contains("0 chunks missing"), "{shown}");
    // A brush that changes nothing saves nothing (no backups, no rewrite).
    let said = ok(&["terrain", root, "raise", "5", "-5", "24", "0"]);
    assert!(
        said.contains("0 squares changed") && said.contains("nothing to save"),
        "{said}"
    );
    assert!(!run(&["terrain", root, "paint", "5", "-5", "24", "99"])
        .status
        .success());

    // A new world in graal3dworlds/ is created on the first change.
    ok(&[
        "add",
        root,
        "Box",
        "isle_rock.3ds",
        "0",
        "0",
        "0",
        "--world",
        "cave",
    ]);
    assert_eq!(ok(&["worlds", root]), "default\ncave\n");

    let png = d.path().join("view.png");
    ok(&["render", root, png.to_str().unwrap(), "--size", "160x100"]);
    let bytes = fs::read(&png).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
}

#[cfg(unix)]
#[test]
fn another_renderer_is_run() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    world(d.path());
    // A stand-in renderer: copies a PNG the editor drew to the out path and
    // records its arguments.
    let first = d.path().join("first.png");
    ok(&[
        "render",
        d.path().to_str().unwrap(),
        first.to_str().unwrap(),
        "--size",
        "32x32",
    ]);
    let script = d.path().join("renderer.sh");
    let log = d.path().join("args.txt");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$@\" > {}\ncp {} \"$3\"\n",
            log.display(),
            first.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let out = d.path().join("ext.png");
    ok(&[
        "render",
        d.path().to_str().unwrap(),
        out.to_str().unwrap(),
        "--renderer",
        script.to_str().unwrap(),
        "--eye",
        "1,2,3",
        "--yaw",
        "45",
        "--size",
        "64x48",
    ]);
    assert_eq!(fs::read(&out).unwrap(), fs::read(&first).unwrap());
    let args = fs::read_to_string(&log).unwrap();
    assert!(
        args.trim_end().ends_with("1 2 3 45 34.3775 60 64 48"),
        "{args}"
    );
}
