//! Authored public-API package-output controls. No corpus or client inputs.
//! Candidate against servertools ea6cd0a6d20acdc638339680ac18cf3208454b0c.
use lat_core::gpak;
use std::fs;
use std::path::PathBuf;

fn existing_package() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<u8>) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("own.txt"), b"original authored contents").unwrap();
    // Output deliberately lies outside source, so collect never includes it.
    let output = root.path().join("owned.gpak");
    let packed = gpak::pack(&source, &output, None, None).unwrap();
    assert_eq!(packed.files, 1);
    let before = fs::read(&output).unwrap();
    let decoded = gpak::read(std::io::Cursor::new(&before), None).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].name, "own.txt");
    assert_eq!(decoded[0].data, b"original authored contents");
    (root, source, output, before)
}

#[test]
fn rejected_empty_repack_preserves_existing_package() {
    let (root, _source, output, before) = existing_package();
    let empty = root.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let error = gpak::pack(&empty, &output, None, None).unwrap_err();
    assert!(error.to_string().contains("at least one file"));
    assert_eq!(
        fs::read(&output).expect("old package must still exist"),
        before
    );
}

#[test]
fn rejected_duplicate_basenames_preserve_existing_package() {
    let (root, _source, output, before) = existing_package();
    let duplicate = root.path().join("duplicate");
    fs::create_dir_all(duplicate.join("a")).unwrap();
    fs::create_dir_all(duplicate.join("b")).unwrap();
    // Distinct directories make this authored case portable on Windows too.
    fs::write(duplicate.join("a/Shared.txt"), b"one").unwrap();
    fs::write(duplicate.join("b/shared.TXT"), b"two").unwrap();
    let error = gpak::pack(&duplicate, &output, None, None).unwrap_err();
    assert!(error.to_string().contains("one hides the other"));
    assert_eq!(
        fs::read(&output).expect("old package must still exist"),
        before
    );
}

#[test]
fn successful_repack_replaces_output_with_complete_readable_package() {
    let (_root, source, output, before) = existing_package();
    fs::write(source.join("own.txt"), b"replacement authored contents").unwrap();
    fs::write(source.join("second.txt"), b"second authored entry").unwrap();
    let packed = gpak::pack(&source, &output, None, None).unwrap();
    assert_eq!(packed.path, output);
    assert_eq!(packed.files, 2);
    assert_eq!(packed.key, None);
    assert_eq!(packed.key_file, None);
    let after = fs::read(&output).unwrap();
    assert_ne!(after, before);
    let decoded = gpak::read(std::io::Cursor::new(&after), None).unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].name, "own.txt");
    assert_eq!(decoded[0].data, b"replacement authored contents");
    assert_eq!(decoded[1].name, "second.txt");
    assert_eq!(decoded[1].data, b"second authored entry");
}

#[test]
fn refused_non_directory_input_preserves_existing_package() {
    let (root, _source, output, before) = existing_package();
    let file = root.path().join("not-a-directory.txt");
    fs::write(&file, b"authored regular file").unwrap();
    let error = gpak::pack(&file, &output, None, None).unwrap_err();
    assert!(matches!(error, gpak::Error::Io(_)));
    assert_eq!(fs::read(&output).unwrap(), before);
}
