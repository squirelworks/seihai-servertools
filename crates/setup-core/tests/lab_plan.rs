//! The one-step lab start's plan, on a fake world folder. Nothing is started.

use std::fs;
use std::path::Path;

use setup_core::lab::{exe_name, plan, LabError, LabOptions, ListserverChoice};

fn server(world: &Path, name: &str, interface: &str, port: u16) {
    let dir = world.join("servers").join(name).join("config");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("serveroptions.txt"),
        format!("# lab\nname = {name}\nserverinterface = {interface}\nserverport = {port}\n"),
    )
    .unwrap();
}

fn world() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(exe_name("gs2emu")), b"").unwrap();
    server(dir.path(), "default", "127.0.0.1", 14802);
    server(dir.path(), "classic", "127.0.0.1", 14803);
    dir
}

fn opts(world: &Path) -> LabOptions {
    LabOptions {
        world: world.to_path_buf(),
        listserver: ListserverChoice::Off,
        ..Default::default()
    }
}

#[test]
fn starts_default_and_classic_when_none_named() {
    let w = world();
    let p = plan(&opts(w.path())).unwrap();
    let names: Vec<_> = p.gservers.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["default", "classic"]);
    assert_eq!(p.gservers[0].spec.args, ["--server", "default"]);
    assert_eq!(p.gservers[1].port.as_deref(), Some("14803"));
    assert!(p.notes.is_empty(), "{:?}", p.notes);
    assert!(p.listserver.is_none());
}

#[test]
fn only_existing_default_folders() {
    let w = world();
    fs::remove_dir_all(w.path().join("servers/classic")).unwrap();
    let p = plan(&opts(w.path())).unwrap();
    assert_eq!(p.gservers.len(), 1);
}

#[test]
fn named_folder_must_exist_and_be_plain() {
    let w = world();
    for bad in ["nope", "../default", ""] {
        let mut o = opts(w.path());
        o.servers = vec![bad.into()];
        assert_eq!(plan(&o), Err(LabError::NoServerFolder(bad.into())));
    }
}

#[test]
fn missing_gserver_is_an_error() {
    let w = world();
    fs::remove_file(w.path().join(exe_name("gs2emu"))).unwrap();
    assert!(matches!(plan(&opts(w.path())), Err(LabError::NoGserver(_))));
}

#[test]
fn public_folder_gets_a_note() {
    let w = world();
    server(w.path(), "default", "AUTO", 14802);
    let p = plan(&opts(w.path())).unwrap();
    assert!(p
        .notes
        .iter()
        .any(|n| n.contains("servers/default is not in lab mode")));
}

#[test]
fn listserver_in_the_world_folder_with_its_config() {
    let w = world();
    let ls = w.path().join(exe_name("seihai-listserver"));
    fs::write(&ls, b"").unwrap();
    fs::write(
        w.path().join("seihai-listserver.toml"),
        "bind = \"127.0.0.1\"\ngserver_port = 15900\n",
    )
    .unwrap();
    let mut o = opts(w.path());
    o.listserver = ListserverChoice::Auto;
    let p = plan(&o).unwrap();
    let l = p.listserver.expect("listserver found");
    assert_eq!(l.spec.program, ls);
    assert_eq!(l.spec.args[0], "--config");
    assert_eq!(l.settings.gserver_port, 15900);
}

#[test]
fn listserver_path_must_exist() {
    let w = world();
    let mut o = opts(w.path());
    o.listserver = ListserverChoice::Path(w.path().join("none"));
    assert!(matches!(plan(&o), Err(LabError::NoListserver(_))));
}
