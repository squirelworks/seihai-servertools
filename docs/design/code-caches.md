# Client cache tooling and acceptance audit

Issue #37 requested local show, CRC, dump and pack commands for GWEBL001 and
GRESCR01. Merged PR #39 supplies those commands and reuses `gpak`'s DES
helper. This audit checks the implementation against the current public
format notes and synthetic fixtures; it is not an original-client lab run.

## CRC domains

`code crc FILE` computes CRC32 of the stored bytes after the eight-byte
GRESCR01 magic, before decryption, and needs no key. `--domain stored` says
the same thing explicitly. `code crc FILE --domain plain --key HEX16`
computes CRC32 of the decrypted body. `code show` prints both, labelled.

The default was `plain` until the real-client check below showed that the
197 CRC is the stored-body CRC. The exact unpacked Windows 6.03 weapon path described in the
[success-output follow-up](https://github.com/squirelworks/Graal_analysis/blob/main/formats/windows-603-weapon-cache.md)
already pointed the same way: it compares the stored-body CRC before
transformation. The older plaintext-CRC account in the
[cache format note](https://github.com/squirelworks/Graal_analysis/blob/main/formats/gwebl001-cache.md)
and [5.x delivery note](https://github.com/squirelworks/Graal_analysis/blob/main/protocol/v5x-clients.md)
is superseded for the builds checked here. This tool exposes checksums, not a
client's acceptance decision.

## Real-client check (Windows 6.113, 2026-11-12)

A private lab on the owner's own machine read decrypted packet 197 frames
from a running, unmodified 6.113 client's receive buffer (memory reads only:
no debugger, injection or packets sent). No keys, script bodies or hosts are
kept in this repo.

- 197 body: `weapon,<name>,1,<key: two GINT5>,<crc: GINT5>`.
- Key: the two GINT5 values as little-endian 32-bit words, first word
  first, with every DES key byte bit-reversed. That is exactly what
  `--key HEX16` plus `gpak`'s bit reversal does.
- Cipher: DES-ECB over whole 8-byte blocks. The last `len % 8` bytes are
  stored unencrypted, as the tool already assumed.
- CRC: the 197 CRC equals CRC32 of the stored body for 7 of 7 cached
  scripts from the login server; it does not equal the plaintext CRC.
- Body: compiled GS2 bytecode, not source. Every decrypted body starts with
  the segment header `00000001 00000004`. The function segment names
  ordinary event and public functions, and the clear tails end in the
  compiler's usual `OP_TYPE_NUMBER 0, OP_RET` (`14 f3 00 07`).
- Not yet seen: packet 140 (every script was a cache hit) and class caches
  in that session. Game-server scripts were not captured.

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
- CRC: explicit decrypted/stored domains; default is stored (real-client check).
- Dump/pack: CLI synthetic round trips implemented; no original payload used.
- Fixtures: existing synthetic GWEBL001 byte-for-byte and GRESCR01 known-key
  tests, plus direct long-length headers and a fixed textbook-DES block with
  an unchanged short tail. Stored and decrypted CRC expectations are fixed
  independently, so the check does not rely only on this tool's writer.
- Real file: done for weapon scripts on Windows 6.113, see above. Class
  caches, packet 140 and other builds remain unchecked.

Gani cache contents, native cache acceptance, decoded-container validation
and GS2 execution remain outside this command's guarantees. Required CI is
actual Linux/Windows/macOS fmt, strict Clippy, tests and headless build.
