# Client reach checks

`seihai-servertools check --client-reach [--json] WORLD` runs the normal world
check (see the [README](../README.md#check-a-world)) and adds warnings for
content that some client versions cannot use. `check --make-gif WORLD` writes
GIF copies first and then runs the check. This page says what each part reads
and what it does not claim.

## Images

The check warns about
level NPC and weapon images, and fixed GS1 script image names, that have PNG art but no
GIF counterpart for pre-2.1 clients. It inventories file names and never
converts or writes art. A matching GIF name does not certify its contents or
original-client rendering.

## Ganis

The same option reports parsed gani files and lists the pinned shared client
profiles whose `ui.ganis` field is false. Those profiles need server animation
fallback review if the file is used. This is an inventory, not a scan of
animation call sites. The pinned data marks 1.41 unsupported and 1.42 supported;
these are shared-profile values, not a new original-client observation. Unused
and standard ganis are included too. A GIF copy
of a sprite sheet does not give a client gani support, and no nearest built-in
animation is inferred. Symbolic sheets such as `BODY` and `HEAD` are not
treated as literal image filenames; ordinary checks still handle parse errors
and missing images/sounds.

## Gmaps and level formats

It also lists, for each parsed gmap, the shared profiles whose
`levels.gmap` is false (they need linked single levels if players reach
it), and for each level the profiles that cannot open its format. The
format comes from the level's magic (`GLEVNW01`, `GR-V1`, `Z3-V1`), not its
extension.

## Scripts

Each level NPC and weapon (`weapons/*.txt` files starting
`GRAWP001`) whose script is GS2 only, or mixes GS1 and GS2, is listed with
the shared profiles that do not run GS2, and those not known either way.
The era is read from the
text: GS2 event functions (`function onCreated()`), `temp.`, `player.`,
`params[` and `new Name(`; GS1 event flags (`if (playerenters)`) and `#`
codes outside strings. GS1-only scripts run on every client and are not
listed. Which clients run GS2 follows seihai #430's evidence (the binary
handles packet 131): yes from 3.153 on; no for 1.21 to 2.31 and 3.04;
unknown for 2.10 to 2.16, 2.30, 6.015 and 6.034, which have no binary. The
check reads seihai-core's `VersionProfile::scripts.gs2` field.

## Tiles

Each level with tiles past column 116 of the default `pics1.png` is
listed with the profiles whose shipped sheet is 1856×512 (1.28 beta 2 to
1.41, and 2.00 to 2.05); they draw nothing in those cells. That sheet's 116
columns match the left 116 of the 2048-wide 1.42 sheet cell for cell, so
tile ids map the same way and the old sheet just ends early. 1.21 (a
1280-wide sheet whose layout is not established) and 2.1x (no sheet in the
archives) are not listed, and `tiledefs` sheets are not matched.

## Run-time image names

A script image name built with `#` codes, such as
`setimg bomb#v(this.n).png`, is read as a pattern where each code can be
any text, and matched (ignoring case and any folder) against the PNG and
GIF names in the world. Each PNG it could be joins the GIF inventory and
`--make-gif`, and the finding names the pattern it came from. A pattern
that matches no file, or is only codes and an extension (`#s(this.pic)`,
`#v(n).png`), is listed to check by hand. A match says which files the
name could be, not which ones the script shows.

## Making GIF copies

`check --make-gif WORLD` (opt-in) first writes a GIF copy next to each PNG
that `--client-reach` reports as having none, then runs the check. A copy
keeps every pixel's colour; pixels with alpha below 128 become the GIF's
transparent colour (this tool's choice, not read from a client). It never
replaces a file, and it skips art with more than 256 colours (255 with
transparency) and says so, rather than picking a palette for you.
