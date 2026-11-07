//! Client download caches: `GWEBL001` level files and `GRESCR01` script files.
//!
//! A level cache is DES-ECB with a fresh key on every 8-byte block. The key
//! stream is seeded from the level name, so the file needs no key beside it.
//! A script cache is one DES key (the two words a server sends with the
//! script). The CRC32 of the plain body is the integer a "use your cache"
//! header carries. DES key bytes are bit-reversed, the same way [`crate::gpak`]
//! encrypts a package.
//!
//! Nothing here talks to a server. Tests build the bytes themselves.

use crate::gpak::{self, Key};

/// A `GWEBL001` level cache, after decryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelCache {
    /// The host stored in the file. A test uses `127.0.0.1` or `<host>`.
    pub host: String,
    /// The server signature (the value packet 25 carries).
    pub signature: u16,
    /// The level's modification time, as the 35-bit integer in the file.
    pub mod_time: u64,
    /// Level file name, lower case, no folder.
    pub level: String,
    /// The level body. A read strips the `0x0A` block padding (at most 7
    /// bytes). A body that itself ends in `0x0A` loses those bytes on read.
    pub body: Vec<u8>,
}

/// A `GRESCR01` script cache, after decryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCache {
    /// zlib CRC-32 of `body`: the integer the 197 header carries.
    pub crc32: u32,
    pub body: Vec<u8>,
}

/// What [`open`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cache {
    Level(LevelCache),
    Script(ScriptCache),
}

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(text: impl Into<String>) -> Error {
    Error(text.into())
}

/// Level name as the cache keys it: the file name only, lower case.
pub fn level_key(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    base.to_ascii_lowercase()
}

/// Level name from a cache file name: `bank.nw-14802.code` is `bank.nw`.
/// A trailing `-<digits>` is the port and is dropped.
pub fn level_from_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = base.strip_suffix(".code").unwrap_or(base);
    if let Some((level, port)) = stem.rsplit_once('-') {
        if !level.is_empty() && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) {
            return level.to_ascii_lowercase();
        }
    }
    stem.to_ascii_lowercase()
}

/// zlib CRC-32. The ISO check value of `123456789` is `0xCBF43926`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn text_bytes(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

fn text_from(b: &[u8]) -> String {
    b.iter().map(|&c| char::from(c)).collect()
}

fn gchar(n: u8) -> u8 {
    n.wrapping_add(0x20)
}

fn gval(b: u8) -> u8 {
    b.wrapping_sub(0x20) & 0x7F
}

/// Delphi `Random(n)`: `seed = seed * 0x08088405 + 1`, then the high half
/// of the unsigned product `seed * n`.
fn delphi_random(seed: &mut u32, n: u32) -> u32 {
    *seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((*seed as u64).wrapping_mul(u64::from(n)) >> 32) as u32
}

fn seed_for(level: &str) -> u32 {
    let mut seed = 0x04A8_0B38u32;
    for b in level_key(level).bytes() {
        seed = seed.wrapping_mul(u32::from(b));
    }
    seed
}

fn next_block_key(seed: &mut u32) -> Key {
    let mut key = [0u8; 8];
    for b in &mut key {
        *b = delphi_random(seed, 256) as u8;
    }
    key
}

fn column_sum(plain: &[u8]) -> [u8; 8] {
    let mut sum = [0u8; 8];
    for block in plain.chunks(8) {
        for (i, b) in block.iter().enumerate() {
            sum[i] = sum[i].wrapping_add(*b);
        }
    }
    sum
}

fn crypt_blocks(data: &[u8], level: &str, encrypt: bool) -> Vec<u8> {
    let mut seed = seed_for(level);
    let mut out = Vec::with_capacity(data.len());
    for block in data.chunks(8) {
        let key = next_block_key(&mut seed);
        let mut piece = if encrypt {
            gpak::encrypt_entry(block, &key)
        } else {
            gpak::decrypt_entry(block, &key)
        };
        piece.truncate(block.len());
        out.append(&mut piece);
    }
    out
}

fn pad_0a(mut plain: Vec<u8>) -> Vec<u8> {
    while !plain.len().is_multiple_of(8) {
        plain.push(0x0A);
    }
    plain
}

/// Drop a trailing `0x0A` pad of at most 7 bytes.
fn strip_pad(plain: &[u8]) -> &[u8] {
    let mut end = plain.len();
    let mut n = 0;
    while n < 7 && end > 0 && plain[end - 1] == 0x0A {
        end -= 1;
        n += 1;
    }
    &plain[..end]
}

/// The unpadded `GWEBL001` plaintext (`code dump` writes this).
pub fn level_plaintext(level: &LevelCache) -> Result<Vec<u8>, Error> {
    let name = level_key(&level.level);
    if name.is_empty() || name.len() > 223 {
        return Err(err("level name must be 1 to 223 bytes"));
    }
    if name.as_bytes().contains(&0x0A) {
        return Err(err("level name cannot contain a line feed"));
    }
    let host = text_bytes(&level.host);
    if host.len() > 223 {
        return Err(err("host must be at most 223 bytes"));
    }
    if level.mod_time >= 1 << 35 {
        return Err(err("mod time does not fit in 35 bits"));
    }
    let mut plain = b"GWEBL001".to_vec();
    plain.push(gchar(host.len() as u8));
    plain.extend_from_slice(&host);
    plain.push(gchar((level.signature >> 7) as u8));
    plain.push(gchar((level.signature & 0x7F) as u8));
    for shift in [28u32, 21, 14, 7, 0] {
        plain.push(gchar(((level.mod_time >> shift) & 0x7F) as u8));
    }
    let level_bytes = text_bytes(&name);
    plain.push(gchar(level_bytes.len() as u8));
    plain.extend_from_slice(&level_bytes);
    plain.extend_from_slice(&level.body);
    Ok(plain)
}

/// Read a dumped level (`code pack` of a `GWEBL001` plaintext).
pub fn parse_level_plaintext(plain: &[u8]) -> Result<LevelCache, Error> {
    parse_level(plain)
}

/// Encrypt a level cache. `level.level` is the keying name.
pub fn write_level(level: &LevelCache) -> Result<Vec<u8>, Error> {
    let plain = level_plaintext(level)?;
    let name = level_key(&level.level);
    let padded = pad_0a(plain);
    let sum = column_sum(&padded);
    let cipher = crypt_blocks(&padded, &name, true);
    let n = u32::try_from(padded.len() + 8).map_err(|_| err("level cache is too large"))?;
    let mut out = Vec::with_capacity(4 + cipher.len() + 8);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&cipher);
    out.extend_from_slice(&sum);
    Ok(out)
}

/// Decrypt a level cache. `level` is the name the file was keyed with
/// (usually [`level_from_file_name`]).
pub fn read_level(data: &[u8], level: &str) -> Result<LevelCache, Error> {
    if data.len() < 12 {
        return Err(err("level cache is too short"));
    }
    let n = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    if n < 8 || data.len() != 4 + (n - 8) + 8 {
        return Err(err("level cache length does not match the file"));
    }
    let cipher_len = n - 8;
    if !cipher_len.is_multiple_of(8) {
        return Err(err("level cache body is not a whole number of blocks"));
    }
    let cipher = &data[4..4 + cipher_len];
    let stored = &data[4 + cipher_len..];
    let name = level_key(level);
    let plain = crypt_blocks(cipher, &name, false);
    if column_sum(&plain).as_slice() != stored {
        return Err(err(
            "level cache checksum failed (the level name is probably wrong)",
        ));
    }
    let parsed = parse_level(strip_pad(&plain))?;
    if level_key(&parsed.level) != name {
        return Err(err(
            "level name inside the file does not match the name it was keyed with",
        ));
    }
    Ok(parsed)
}

fn parse_level(plain: &[u8]) -> Result<LevelCache, Error> {
    if plain.len() < 8 || &plain[..8] != b"GWEBL001" {
        return Err(err("level cache is not GWEBL001"));
    }
    let mut i = 8;
    let take = |i: &mut usize| -> Result<u8, Error> {
        let b = *plain
            .get(*i)
            .ok_or_else(|| err("level cache ended early"))?;
        *i += 1;
        Ok(b)
    };
    let host_len = gval(take(&mut i)?) as usize;
    let host = plain
        .get(i..i + host_len)
        .ok_or_else(|| err("level cache host is cut off"))?
        .to_vec();
    i += host_len;
    let hi = u16::from(gval(take(&mut i)?));
    let lo = u16::from(gval(take(&mut i)?));
    let mut mod_time = 0u64;
    for _ in 0..5 {
        mod_time = (mod_time << 7) | u64::from(gval(take(&mut i)?));
    }
    let name_len = gval(take(&mut i)?) as usize;
    let name = plain
        .get(i..i + name_len)
        .ok_or_else(|| err("level cache name is cut off"))?
        .to_vec();
    i += name_len;
    let level = text_from(&name);
    if i > plain.len() {
        return Err(err("level cache ended early"));
    }
    Ok(LevelCache {
        host: text_from(&host),
        signature: hi * 128 + lo,
        mod_time,
        level,
        body: plain[i..].to_vec(),
    })
}

/// Encrypt a script cache. A short tail (shorter than 8 bytes) stays plain,
/// as a package entry does.
pub fn write_script(body: &[u8], key: &Key) -> Vec<u8> {
    let mut out = b"GRESCR01".to_vec();
    out.extend(gpak::encrypt_entry(body, key));
    out
}

/// Decrypt a script cache.
pub fn read_script(data: &[u8], key: &Key) -> Result<ScriptCache, Error> {
    let body = data
        .strip_prefix(b"GRESCR01")
        .ok_or_else(|| err("script cache is not GRESCR01"))?;
    let body = gpak::decrypt_entry(body, key);
    Ok(ScriptCache {
        crc32: crc32(&body),
        body,
    })
}

/// Open a cache file. A `GRESCR01` file needs `key`. Anything else is read
/// as a `GWEBL001` level cache keyed by `level`.
pub fn open(data: &[u8], level: Option<&str>, key: Option<&Key>) -> Result<Cache, Error> {
    if data.starts_with(b"GRESCR01") {
        let key = key.ok_or_else(|| err("GRESCR01 needs --key"))?;
        return Ok(Cache::Script(read_script(data, key)?));
    }
    let level = level.ok_or_else(|| err("GWEBL001 needs the level name"))?;
    Ok(Cache::Level(read_level(data, level)?))
}

/// Text for `code show`.
pub fn show_text(cache: &Cache) -> String {
    match cache {
        Cache::Level(level) => format!(
            "GWEBL001\nhost: {}\nsignature: {}\nmod_time: {}\nlevel: {}\nbody_bytes: {}\n",
            level.host,
            level.signature,
            level.mod_time,
            level.level,
            level.body.len()
        ),
        Cache::Script(script) => format!(
            "GRESCR01\ncrc32: {}\nbody_bytes: {}\n",
            script.crc32,
            script.body.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_level() -> LevelCache {
        LevelCache {
            host: "127.0.0.1".into(),
            signature: 73,
            mod_time: 1_700_000_000,
            level: "start.nw".into(),
            body: b"GR-V1.03 level body".to_vec(),
        }
    }

    #[test]
    fn crc32_matches_the_iso_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn random_256_is_the_high_byte() {
        let mut seed = 1u32;
        let got = delphi_random(&mut seed, 256);
        assert_eq!(got, seed >> 24);
    }

    #[test]
    fn level_file_name_drops_the_port() {
        assert_eq!(level_from_file_name("start.nw-14802.code"), "start.nw");
        assert_eq!(
            level_from_file_name("weblevels/<host>/Start.NW.code"),
            "start.nw"
        );
    }

    #[test]
    fn gwebl_round_trips_byte_for_byte() {
        let level = sample_level();
        let bytes = write_level(&level).unwrap();
        let back = read_level(&bytes, "start.nw").unwrap();
        assert_eq!(back, level);
        assert_eq!(write_level(&back).unwrap(), bytes);
        assert!(read_level(&bytes, "other.nw").is_err());
    }

    #[test]
    fn grescr_round_trips_and_prints_the_crc() {
        let key = gpak::parse_key("0123456789ABCDEF").unwrap();
        let body = b"if (playerenters) { }";
        let bytes = write_script(body, &key);
        assert!(bytes.starts_with(b"GRESCR01"));
        let back = read_script(&bytes, &key).unwrap();
        assert_eq!(back.body, body);
        assert_eq!(back.crc32, crc32(body));
        let shown = show_text(&Cache::Script(back));
        assert!(shown.starts_with("GRESCR01\ncrc32: "));
        assert!(!shown.contains('\n') || shown.lines().nth(1).unwrap().starts_with("crc32: "));
    }
}
