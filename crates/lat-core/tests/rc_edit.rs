//! List, get, edit, put and get again, against a fake RC file server.
//!
//! The server speaks only what `seihai-rc`'s file link already sends and
//! reads. The editor side is [`lat_core::remote::RcFiles`]: `list_dir`,
//! `get_file` and `put_file`. Nothing here builds a second client.
//!
//! Loopback only, ephemeral port, no real account.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use lat_core::gani::GaniDoc;
use lat_core::level::LevelDoc;
use lat_core::remote::{Entry, LocalFolder, RcFiles, RemoteFiles};
use seihai_core::protocol::{read_gint3, split_messages, zlib_decompress};
use seihai_rc::files::{self, parse_file_body};
use seihai_rc::link::Codec;
use seihai_rc::rc::PLO_SIGNATURE;
use seihai_rc::wire::{self, message, text};

const LEVEL: &[u8] = b"GLEVNW01\nBOARD 0 0 1 0 AA\n";
const GANI: &[u8] = b"\
SPRITE 0 BODY 0 0 1 1
SINGLEDIRECTION
ANI
0 0 0
ANIEND
";

fn names(entries: &[Entry]) -> Vec<&str> {
    entries.iter().map(|e| e.name.as_str()).collect()
}

fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 2];
    stream.read_exact(&mut len)?;
    let mut frame = vec![0u8; usize::from(u16::from_be_bytes(len))];
    stream.read_exact(&mut frame)?;
    Ok(frame)
}

fn write_frame(stream: &mut TcpStream, codec: &mut Codec, body: &[u8]) {
    let frame = codec.encode(body).expect("encode reply");
    stream.write_all(&frame).expect("write reply");
    stream.flush().expect("flush reply");
}

fn with_nl(mut msg: Vec<u8>) -> Vec<u8> {
    msg.push(b'\n');
    msg
}

fn dir_list(names: &[String]) -> Vec<u8> {
    let mut payload = Vec::new();
    wire::push_gshort(&mut payload, names.len() as u16);
    for name in names {
        wire::push_gstring(&mut payload, name.as_bytes());
    }
    with_nl(message(files::plo::RC_FILEBROWSER_DIRLIST, &payload))
}

fn file_reply(name: &str, data: &[u8]) -> Vec<u8> {
    let mut inner = vec![wire::gchar(files::plo::RC_FILEBROWSER_DIR)];
    wire::push_gstring(&mut inner, name.as_bytes());
    wire::push_gint3(&mut inner, data.len() as u32);
    inner.extend_from_slice(data);
    let mut body = vec![wire::gchar(files::PLO_RAWDATA)];
    wire::push_gint3(&mut body, inner.len() as u32);
    body.push(b'\n');
    body.extend_from_slice(&inner);
    body
}

fn refusal(words: &str) -> Vec<u8> {
    with_nl(message(
        files::plo::RC_FILEBROWSER_MESSAGE,
        words.as_bytes(),
    ))
}

fn gstring_text(payload: &[u8]) -> String {
    let mut i = 0;
    text(&wire::read_gstring(payload, &mut i).expect("gstring"))
}

/// A put is an outbound raw block (id 50). `split_messages` does not treat
/// that id as raw, so the file bytes are taken from the block directly,
/// with `seihai-rc`'s `parse_file_body`.
fn take_put(body: &[u8]) -> Option<(String, Vec<u8>)> {
    if body.first().map(|b| b.wrapping_sub(0x20)) != Some(files::PLI_RAWDATA) {
        return None;
    }
    let n = read_gint3(body.get(1..4)?)?;
    if body.get(4) != Some(&b'\n') || body.len() < 5 + n {
        return None;
    }
    let inner = &body[5..5 + n];
    if inner.first().map(|b| b.wrapping_sub(0x20)) != Some(files::pli::RC_FILEBROWSER_UP) {
        return None;
    }
    let (name, bytes) = parse_file_body(&inner[1..]).ok()?;
    Some((name, bytes))
}

fn list_names(files: &BTreeMap<String, Vec<u8>>, dir: &str) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for path in files.keys() {
        let rest = if dir.is_empty() {
            path.as_str()
        } else {
            let Some(rest) = path.strip_prefix(dir) else {
                continue;
            };
            let Some(rest) = rest.strip_prefix('/') else {
                continue;
            };
            rest
        };
        if let Some(name) = rest.split('/').next() {
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
    }
    names.into_iter().collect()
}

fn serve(listener: TcpListener, mut files: BTreeMap<String, Vec<u8>>) {
    let Ok((mut stream, _)) = listener.accept() else {
        return;
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let Ok(frame) = read_frame(&mut stream) else {
        return;
    };
    let Ok(login) = zlib_decompress(&frame) else {
        return;
    };
    let Some(&kind) = login.first() else {
        return;
    };
    let mut codec = match kind.wrapping_sub(0x20) {
        6 => {
            let key = login.get(1).copied().unwrap_or(0x20).wrapping_sub(0x20);
            Codec::gen5(key)
        }
        1 => Codec::Plain,
        _ => return,
    };
    write_frame(
        &mut stream,
        &mut codec,
        &with_nl(message(PLO_SIGNATURE, &[wire::gchar(73)])),
    );
    while let Ok(frame) = read_frame(&mut stream) {
        let Ok(body) = codec.decode(&frame) else {
            break;
        };
        if let Some((name, bytes)) = take_put(&body) {
            files.insert(name, bytes);
            continue;
        }
        for msg in split_messages(&body) {
            let reply = match msg.id {
                files::pli::RC_FILEBROWSER_CD => {
                    let dir = gstring_text(&msg.payload);
                    Some(dir_list(&list_names(&files, &dir)))
                }
                files::pli::RC_FILEBROWSER_DOWN => {
                    let path = gstring_text(&msg.payload);
                    Some(match files.get(&path) {
                        Some(data) => file_reply(&path, data),
                        None => refusal("no such file"),
                    })
                }
                _ => None,
            };
            if let Some(reply) = reply {
                write_frame(&mut stream, &mut codec, &reply);
            }
        }
    }
}

fn fake_server(files: BTreeMap<String, Vec<u8>>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    let handle = thread::spawn(move || serve(listener, files));
    (addr, handle)
}

#[test]
fn list_get_edit_put_and_get_again_on_loopback() {
    let mut seed = BTreeMap::new();
    seed.insert("levels/room.nw".to_string(), LEVEL.to_vec());
    seed.insert("levels/idle.gani".to_string(), GANI.to_vec());
    let (addr, handle) = fake_server(seed);

    let mut remote = RcFiles::connect(&addr, "labrc", "labpw").expect("login");
    assert!(
        remote.list("../x").is_err(),
        "a path must not leave the world"
    );
    assert_eq!(names(&remote.list("").unwrap()), ["levels"]);
    assert_eq!(
        names(&remote.list("levels").unwrap()),
        ["idle.gani", "room.nw"]
    );

    let got = remote.get("levels/room.nw").unwrap();
    assert_eq!(got, LEVEL);
    let mut level = LevelDoc::open(&got).unwrap();
    level.set_tile(0, 0, 1).unwrap();
    let encoded = level.encode();
    assert_ne!(encoded, LEVEL);
    remote.put("levels/room.nw", &encoded).unwrap();
    let again = remote.get("levels/room.nw").unwrap();
    assert_eq!(again, encoded);
    assert_eq!(LevelDoc::open(&again).unwrap().tile(0, 0), Some(1));

    let got = remote.get("levels/idle.gani").unwrap();
    assert_eq!(got, GANI);
    let mut gani = GaniDoc::open(&got).unwrap();
    gani.set_sprite_frame(0, 1, 0, 1, 1).unwrap();
    let encoded = gani.encode();
    assert_ne!(encoded, GANI);
    remote.put("levels/idle.gani", &encoded).unwrap();
    let again = remote.get("levels/idle.gani").unwrap();
    assert_eq!(again, encoded);
    assert_eq!(
        GaniDoc::open(&again).unwrap().sprite_frame(0),
        Some((1, 0, 1, 1))
    );

    drop(remote);
    handle.join().expect("fake server");
}

#[test]
fn the_same_edit_through_a_local_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = LocalFolder::new(dir.path());
    files.put("levels/room.nw", LEVEL).unwrap();
    let got = files.get("levels/room.nw").unwrap();
    let mut level = LevelDoc::open(&got).unwrap();
    level.set_tile(0, 0, 1).unwrap();
    let encoded = level.encode();
    files.put("levels/room.nw", &encoded).unwrap();
    assert_eq!(files.get("levels/room.nw").unwrap(), encoded);
}
