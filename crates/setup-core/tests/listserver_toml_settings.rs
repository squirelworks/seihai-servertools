use setup_core::config::listserver_toml::{ListserverLab, ListserverToml};
use setup_core::folder::ListserverConfig;
use std::path::PathBuf;

#[test]
fn owner_accounts_path_keeps_hash_after_edit_and_reload() {
    let original = "# owner comment\nunknown = 'keep me'\n";
    let mut doc = ListserverToml::parse(original).unwrap();
    assert!(doc.set_str("accounts", "owners#one.txt", "2026-11-22"));
    assert_eq!(doc.get("accounts").as_deref(), Some("owners#one.txt"));
    let rendered = doc.render();
    assert!(rendered.contains(original));
    let again = ListserverToml::parse(&rendered).unwrap();
    assert_eq!(again.render(), rendered);
    assert_eq!(again.settings().accounts, "owners#one.txt");
    // No filesystem calls: this is the production path supplied to first-account creation.
    let config = ListserverConfig {
        path: PathBuf::from("authored-world").join("seihai-listserver.toml"),
        toml: again,
        exists: true,
    };
    assert_eq!(
        config.accounts_path(&config.toml),
        Some(PathBuf::from("authored-world").join("owners#one.txt"))
    );
}

#[test]
fn valid_literal_and_escaped_toml_strings_use_decoded_values() {
    for text in [
        "bind = 'localhost'\naccounts = 'owners#one.txt' # outside comment\n",
        "bind = \"localhost\"\naccounts = \"owners\\u0023one.txt\" # outside comment\n",
    ] {
        let doc = ListserverToml::parse(text).unwrap();
        assert_eq!(doc.render(), text);
        assert_eq!(doc.get("accounts").as_deref(), Some("owners#one.txt"));
        let settings = doc.settings();
        assert_eq!(settings.accounts, "owners#one.txt");
        assert_eq!(settings.bind, "localhost");
        assert!(settings.is_loopback());
    }
}

#[test]
fn valid_toml_numeric_separators_are_not_replaced_by_defaults() {
    let text = "gserver_port = 14_802\nmax_connections = 1_200\nsilent_list_after_ms = 5_000\n";
    let doc = ListserverToml::parse(text).unwrap();
    let settings = doc.settings();
    assert_eq!(settings.gserver_port, 14802);
    assert_eq!(settings.max_connections, 1200);
    assert_eq!(settings.silent_list_after_ms, 5000);
    assert_eq!(ListserverLab::scan(text), settings);
    assert_eq!(doc.render(), text);
}

#[test]
fn malformed_lab_file_keeps_existing_best_effort_line_scan() {
    let text = "bind = \"localhost\"\ngserver_port = 14802\nbroken = [\n";
    assert!(ListserverToml::parse(text).is_err());
    let settings = ListserverLab::scan(text);
    assert_eq!(settings.bind, "localhost");
    assert_eq!(settings.gserver_port, 14802);
    assert!(settings.gservers.is_empty());
}
