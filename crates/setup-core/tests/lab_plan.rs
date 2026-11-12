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
    server(dir.path(), "modern", "127.0.0.1", 14805);
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
fn starts_default_classic_and_modern_when_none_named() {
    let w = world();
    let p = plan(&opts(w.path())).unwrap();
    let names: Vec<_> = p.gservers.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["default", "classic", "modern"]);
    assert_eq!(p.gservers[0].spec.args, ["--server", "default"]);
    assert_eq!(p.gservers[1].port.as_deref(), Some("14803"));
    assert_eq!(p.gservers[2].port.as_deref(), Some("14805"));
    assert!(p.notes.is_empty(), "{:?}", p.notes);
    assert!(p.listserver.is_none());
}

#[test]
fn only_existing_default_folders() {
    let w = world();
    fs::remove_dir_all(w.path().join("servers/classic")).unwrap();
    let p = plan(&opts(w.path())).unwrap();
    assert_eq!(p.gservers.len(), 2);
}

#[test]
fn world_without_modern_still_starts() {
    // Releases before seihai-gserver-v2's servers/modern have only default and classic.
    let w = world();
    fs::remove_dir_all(w.path().join("servers/modern")).unwrap();
    let p = plan(&opts(w.path())).unwrap();
    let names: Vec<_> = p.gservers.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["default", "classic"]);
    assert!(p.notes.is_empty(), "{:?}", p.notes);
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
fn listserver_without_silent_list_gets_a_note() {
    let w = world();
    fs::write(w.path().join(exe_name("seihai-listserver")), b"").unwrap();
    let mut o = opts(w.path());
    o.listserver = ListserverChoice::Auto;
    let has_note = |o: &LabOptions| {
        plan(o)
            .unwrap()
            .notes
            .iter()
            .any(|n| n.contains("real 1.41 rev 1 gets no server list"))
    };
    assert!(has_note(&o), "no config: the listserver default is 0");
    fs::write(
        w.path().join("seihai-listserver.toml"),
        "silent_list_after_ms = 500\n",
    )
    .unwrap();
    assert!(!has_note(&o));
}

#[test]
fn listserver_path_must_exist() {
    let w = world();
    let mut o = opts(w.path());
    o.listserver = ListserverChoice::Path(w.path().join("none"));
    assert!(matches!(plan(&o), Err(LabError::NoListserver(_))));
}

/// A seihai-gserver world folder with `config/server.txt`.
fn rust_world(dir: &Path, text: &str) {
    fs::create_dir_all(dir.join("config")).unwrap();
    fs::write(dir.join("config/server.txt"), text).unwrap();
}

fn with_rust(world: &Path) -> LabOptions {
    let program = world.join(exe_name("seihai-gserver"));
    fs::write(&program, b"").unwrap();
    LabOptions {
        rust_worlds: vec!["rust".into()],
        rust_gserver: Some(program),
        ..opts(world)
    }
}

#[test]
fn rust_world_starts_next_to_the_fork() {
    use setup_core::lab::ServerKind;
    let w = world();
    rust_world(
        &w.path().join("rust"),
        "name = Rust Lab\nport = 14806 # its own port\nbind = 127.0.0.1\n",
    );
    let p = plan(&with_rust(w.path())).unwrap();
    let names: Vec<_> = p.gservers.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["default", "classic", "modern", "rust"]);
    let r = &p.gservers[3];
    assert_eq!(r.kind, ServerKind::Rust);
    assert_eq!(p.gservers[0].kind, ServerKind::Fork);
    assert_eq!(
        r.spec.args,
        [
            "--world".to_string(),
            w.path().join("rust").display().to_string()
        ]
    );
    assert_eq!(r.spec.console, w.path().join("logs/rust-console.txt"));
    assert_eq!(
        (r.interface.as_deref(), r.port.as_deref()),
        (Some("127.0.0.1"), Some("14806"))
    );
    assert!(p.notes.is_empty(), "{:?}", p.notes);
    assert!(
        p.to_string()
            .contains("rust        rust on 127.0.0.1:14806"),
        "{p}"
    );
}

#[test]
fn rust_world_alone_and_port_clash() {
    let w = tempfile::tempdir().unwrap();
    // seihai-gserver's defaults: 127.0.0.1:14802, like servers/default.
    rust_world(&w.path().join("rust"), "# defaults\n");
    let p = plan(&with_rust(w.path())).unwrap();
    assert_eq!(p.gservers.len(), 1);
    assert_eq!(p.gservers[0].port.as_deref(), Some("14802"));

    // With fork folders but no gs2emu, the Rust world starts alone and says so.
    server(w.path(), "default", "127.0.0.1", 14802);
    let p = plan(&with_rust(w.path())).unwrap();
    assert_eq!(p.gservers.len(), 1);
    assert!(
        p.notes
            .iter()
            .any(|n| n.contains("worlds only, not default")),
        "{:?}",
        p.notes
    );

    // Naming a fork folder still needs gs2emu.
    let mut o = with_rust(w.path());
    o.servers = vec!["default".into()];
    assert!(matches!(plan(&o), Err(LabError::NoGserver(_))));

    // With gs2emu both start, and the shared port is pointed out.
    fs::write(w.path().join(exe_name("gs2emu")), b"").unwrap();
    let p = plan(&with_rust(w.path())).unwrap();
    assert_eq!(p.gservers.len(), 2);
    assert!(
        p.notes
            .iter()
            .any(|n| n.contains("default and rust both listen on 127.0.0.1:14802")),
        "{:?}",
        p.notes
    );
}

#[test]
fn rust_world_errors_and_bind_note() {
    let w = world();
    let mut o = with_rust(w.path());
    assert!(matches!(plan(&o), Err(LabError::NoRustWorld(_))));
    rust_world(&w.path().join("rust"), "bind = 0.0.0.0\nport = 14806\n");
    let p = plan(&o).unwrap();
    assert!(
        p.notes.iter().any(|n| n.contains("binds 0.0.0.0")),
        "{:?}",
        p.notes
    );
    o.rust_gserver = Some(w.path().join("missing"));
    assert!(matches!(plan(&o), Err(LabError::NoRustGserver(Some(_)))));
}

#[test]
fn rust_listserver_line_is_shown_not_edited() {
    let w = world();
    let ls = w.path().join(exe_name("seihai-listserver"));
    fs::write(&ls, b"").unwrap();
    let text = "port = 14806\nlistserver = 127.0.0.1:14901\n";
    rust_world(&w.path().join("rust"), text);
    let mut o = with_rust(w.path());
    o.listserver = ListserverChoice::Path(ls);
    let p = plan(&o).unwrap();
    let note = p
        .notes
        .iter()
        .find(|n| n.contains("will not show in the lab list"))
        .expect("a note");
    assert!(note.contains("`listserver = 127.0.0.1:14901`"), "{note}");
    assert!(note.contains("`listserver = 127.0.0.1:14900`"), "{note}");
    assert_eq!(
        fs::read_to_string(w.path().join("rust/config/server.txt")).unwrap(),
        text,
        "the file is not changed"
    );
    // Pointing at the lab listserver: no note.
    rust_world(
        &w.path().join("rust"),
        "port = 14806\nlistserver = 127.0.0.1:14900\n",
    );
    let p = plan(&o).unwrap();
    assert!(
        p.notes.iter().all(|n| !n.contains("lab list")),
        "{:?}",
        p.notes
    );
    // No listserver line at all.
    rust_world(&w.path().join("rust"), "port = 14806\n");
    let p = plan(&o).unwrap();
    assert!(
        p.notes.iter().any(|n| n.contains("no listserver line")),
        "{:?}",
        p.notes
    );
}
