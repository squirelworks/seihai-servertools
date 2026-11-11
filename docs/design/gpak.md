# Resource packages (.gpak, .gpake)

A package is one ZIP that holds many game files (images, ganis, sounds,
scripts). The 4.20 and 6.x clients save a package the server sends under
`levels/gpaks/` and then find every file in it by its bare, lower-case name,
as if it lay loose in the levels folder. A `.gpake` is the same ZIP with
every entry encrypted; the client reads it only after the server has sent
that package's key.

`lat_core::gpak` builds and reads both; `seihai-servertools gpak` is the
command line. `seihai-gserver-v2` serves the result (its `gpaks` server
option and `config/gpakkeys.txt`).

## Build one for a gserver folder

```
seihai-servertools gpak pack art/ art.gpake --server path/to/servers/modern
```

writes `servers/modern/world/gpaks/art.gpake` and adds `art.gpake <KEY>`
to `servers/modern/config/gpakkeys.txt` (a random key unless `--key
HEX16` is given). Then set `gpaks = art.gpake` in `serveroptions.txt` to
send it at login; the gserver sends the key first. Keep `gpakkeys.txt` out
of `world/`: everything there can be downloaded.

In the stock lab world use `servers/modern` (127.0.0.1:14805): it is the
folder whose generation and `allowedversions.txt` let in the clients that
load packages (4.20r8 to 6.037). `servers/default` (14802) only lets in 2.00
to 3.153, which load none.

`gpak unpack art.gpake out/ --server path/to/servers/modern` gets the
files back.

## Which name to use

| Client | `.gpake` | `.gpak` | `.zip` |
|---|---|---|---|
| 6.03r7 | loads | loads | loads |
| 4.20r8 (and Mac 4.20r6, same code) | loads | saved, never loaded | loads |
| 4.04r2 | no packages | no packages | no packages |
| before 4.x | no packages | no packages | no packages |

So use `.gpake` when 4.20 players matter. 5.x was not run yet.

## Lab runs

2026-11-06, `seihai-gserver-v2` with the `gpaks` option, Wine, loopback
only. Real 4.20r8: a `.gpake` built with `gpak pack` was sent at login, key
first, and its image drawn; with a wrong key in `gpakkeys.txt` it was not
drawn; with no key the gserver refused to send it; a plain `.gpak` was not
sent to it. Real 6.03r7 received the key and then the package and saved it
to `levels/gpaks/`.

## Format (from the real-client lab runs, 2026-11-06)

- A plain PKZIP archive, stored or deflate, no ZIP password. At most
  10 000 entries; an entry over 20 MiB is skipped; names up to 255 bytes.
  The packer refuses two entries with the same file name in different
  folders, since the client would see only one.
- `.gpake`: each entry's uncompressed bytes are DES-ECB encrypted, every
  full 8-byte block, the short tail left plain, no padding. The ZIP sizes
  and CRC are of the encrypted bytes.
- The client's DES reads each key byte bit-reversed; otherwise it is
  standard DES. So the packer encrypts with standard DES under the key with
  every byte bit-reversed. The unit tests hold the client's own outputs for
  two keys.
- The key reaches the client in `PLO 104`: `GSTRING` package name (lower
  case, no path), `GCHAR 1`, then key bytes 0–3 and 4–7 as little-endian
  32-bit values in `GINT5`. It must come before the file (`PLO 102`, or
  68…69 for a large one) or before `PLO 45` "up to date".
