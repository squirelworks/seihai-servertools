//! `seihai-regions`, run as a program on its own.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seihai-regions"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn ok(args: &[&str], dir: &Path) -> String {
    let out = run(args, dir);
    assert!(
        out.status.success(),
        "{args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn split_cuts_terrain_and_objects_into_regions() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path();
    fs::create_dir_all(root.join("world/big")).unwrap();
    // A 32 x 16 terrain at square size 8: x from -2048, y from -1024.
    for (x, y) in [(0, 0), (31, 15), (8, 3)] {
        fs::write(
            root.join(format!("world/big/heights{x}-{y}.dat")),
            [x as u8, y as u8],
        )
        .unwrap();
    }
    fs::write(root.join("world/big/heights.info"), "g3d_grass.png\n").unwrap();
    fs::write(root.join("world/big/g3d_grass.png"), "png").unwrap();
    fs::write(
        root.join("world.txt"),
        "sun 0 0 -1 1 1 1 1 0 0 0 1\n\
         terrain world/big 8 repeat 32x16\n\
         spawn 10 10 1\n\
         object hut hut.dts -2000 -1000 2 # by the corner\n\
         objectfor bob door door.dts 1030.5 5 0\n\
         water -100 -100 0 200 200 2 water 0\n",
    )
    .unwrap();
    let said = ok(
        &["split", "world.txt", "--root", ".", "--into", "world/land"],
        root,
    );
    assert!(said.contains("4 x 2 regions of 1024 units"), "{said}");
    let world = fs::read_to_string(root.join("world-regions.txt")).unwrap();
    assert!(world.contains("regions world/land 4x2 8 8\n"), "{world}");
    assert!(
        world.contains("sun 0 0")
            && world.contains("spawn 10 10 1")
            && world.contains("water -100")
    );
    assert!(!world.contains("object") && !world.contains("terrain"));
    let land = root.join("world/land");
    assert_eq!(fs::read(land.join("0-0/heights0-0.dat")).unwrap(), [0, 0]);
    assert_eq!(fs::read(land.join("3-1/heights7-7.dat")).unwrap(), [31, 15]);
    assert_eq!(fs::read(land.join("1-0/heights0-3.dat")).unwrap(), [8, 3]);
    assert!(land.join("heights.info").is_file() && land.join("g3d_grass.png").is_file());
    assert_eq!(
        fs::read_to_string(land.join("0-0/region.txt")).unwrap(),
        "object hut hut.dts 48 24 2 # by the corner\n"
    );
    assert_eq!(
        fs::read_to_string(land.join("3-1/region.txt")).unwrap(),
        "objectfor bob door door.dts 6.5 5 0\n"
    );
    // Again refuses without --force.
    assert!(!run(
        &["split", "world.txt", "--root", ".", "--into", "world/land"],
        root
    )
    .status
    .success());
    // Stats on either form count the same.
    let before = ok(&["stats", "world.txt", "--root", "."], root);
    let after = ok(&["stats", "world-regions.txt", "--root", "."], root);
    assert!(
        before.contains("4 x 2 regions of 1024 units (8 chunks each), 2 objects"),
        "{before}"
    );
    assert_eq!(before, after);
}

#[test]
fn a_size_that_does_not_divide_is_refused() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("w.txt"), "terrain t 8 repeat 48x48\n").unwrap();
    let out = run(
        &[
            "split", "w.txt", "--root", ".", "--into", "r", "--chunks", "32",
        ],
        t.path(),
    );
    assert!(!out.status.success());
    let out = run(
        &[
            "split", "w.txt", "--root", ".", "--into", "r", "--chunks", "16",
        ],
        t.path(),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn demo_builds_a_world_the_stats_find_and_that_fits() {
    let t = tempfile::tempdir().unwrap();
    let said = ok(
        &[
            "demo",
            "out",
            "--regions",
            "2x2",
            "--towns",
            "1",
            "--town-objects",
            "300",
            "--trees",
            "10",
        ],
        t.path(),
    );
    assert!(said.contains("1 towns, 300 houses,"), "{said}");
    let out = t.path().join("out");
    let world = fs::read_to_string(out.join("config/graal3dworld.txt")).unwrap();
    assert!(world.contains("regions world/3d/regions 2x2 8 8\n"));
    for f in [
        "world/3d/rg_house.3ds",
        "world/3d/regions/heights.info",
        "world/3d/regions/1-1/heights7-7.dat",
    ] {
        assert!(out.join(f).is_file(), "{f}");
    }
    let n = fs::read_dir(out.join("world/3d/regions/0-0"))
        .unwrap()
        .count();
    assert_eq!(n, 64 + 1); // its chunks and region.txt
    let stats = ok(
        &["stats", "out/config/graal3dworld.txt", "--root", "out"],
        t.path(),
    );
    assert!(
        stats.contains("2 x 2 regions of 1024 units") && stats.contains(": fits"),
        "{stats}"
    );
    assert!(!run(&["demo", "out", "--regions", "2x2"], t.path())
        .status
        .success());
}
