# Client cache tooling and acceptance audit

Issue #37 requested local show, CRC, dump and pack commands for GWEBL001 and
GRESCR01. Merged PR #39 supplies those commands and reuses `gpak`'s DES
helper. This audit checks the implementation against the current public
format notes and synthetic fixtures; it is not an original-client lab run.

## CRC domains

`code crc FILE --domain plain --key HEX16` computes CRC32 of the extracted,
decrypted body. Omitting `--domain` preserves this existing default.
`code crc FILE --domain stored` computes CRC32 of the stored bytes after
the eight-byte GRESCR01 magic and needs no key. `code show` retains its
`crc32` field for the decrypted body and adds `stored_crc32` plus domain labels.

The exact unpacked Windows 6.03 weapon path described in the
[success-output follow-up](https://github.com/squirelworks/Graal_analysis/blob/main/formats/windows-603-weapon-cache.md)
compares the stored-body CRC **before** transformation. Expected CRC zero
skips comparison and leaves success false; returned bytes are separate from
success, and the weapon caller requests a script on false. Header-only data
therefore does not establish acceptance. That static-derived result is not
a new runtime observation or a contract for all classes, builds or callers.
This tool exposes checksums, not that caller's acceptance decision.

The older plaintext-CRC account in the
[cache format note](https://github.com/squirelworks/Graal_analysis/blob/main/formats/gwebl001-cache.md)
and [5.x delivery note](https://github.com/squirelworks/Graal_analysis/blob/main/protocol/v5x-clients.md)
must be read with this exact-build qualification. Do not choose a checksum
domain merely because decryption returned bytes. The encryption key does not
authenticate the contents, and this tool does not validate decoded GS2.

## Level header lengths

Host/name lengths are GCHAR byte counts, unlike the seven-bit groups in
GSHORT and the five-byte modification time. The existing writer allows up
to 223 bytes but the reader masked those counts to seven bits. It now reads
the full count. Independently assembled plaintext headers test 127/128 and
223-byte boundaries, then exercise encrypted round trips. This fixes a
tool writer/reader inconsistency; it does not prove every historical client
accepts those lengths. The writer also rejects signatures above the 14-bit
two-group field instead of silently discarding high bits. Existing
newline-padding ambiguity remains: an
original level body ending in padding-like line feeds can lose those bytes
on extraction. Do not claim arbitrary-body lossless round trips.

## Acceptance accounting

- Shared DES helper: used by both cache formats; no duplicate cipher.
- Show: level metadata/body size and script-body checksum fields implemented.
- CRC: explicit decrypted/stored domains and the compatible default tested.
- Dump/pack: CLI synthetic round trips implemented; no original payload used.
- Fixtures: existing synthetic GWEBL001 byte-for-byte and GRESCR01 known-key
  tests, plus direct long-length headers and a fixed textbook-DES block with
  an unchanged short tail. Stored and decrypted CRC expectations are fixed
  independently, so the check does not rely only on this tool's writer.
- Still open: decrypt a real GRESCR01 file using its matching captured key,
  identify the body without assuming source or bytecode, and publish the
  appropriately scoped research result. No suitable original/key pair was
  verified in this audit. Do not close #37 on synthetic tests alone.

Gani cache contents, native cache acceptance, decoded-container validation
and GS2 execution remain outside this command's guarantees. Required CI is
actual Linux/Windows/macOS fmt, strict Clippy, tests and headless build.
