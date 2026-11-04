# LAT upload refused when the server copy changed, 2026-11-04

What: the level editor page shows your changes against the downloaded
bytes, and refuses the upload when the file changed on the server after you
opened it (`seihai-servertools gui --world DIR --page levels`). The gani
page uses the same code (`crates/app/src/pages/review.rs`).

## Setup

- Linux, Xvfb at 1100x800, software GL.
- World folder (a folder on this machine, not an RC login) with one level,
  `levels/room.nw`: `GLEVNW01` and `BOARD 0 0 4 0 AAAAAAAA`.
- The "someone else" edit was a direct write to that file while the page
  had it open.

## Result

1. List, pick `room.nw`, Open: "Opened levels/room.nw (32 bytes)".
2. Set tile 0,0 to 1, then Show changes: "Your changes to levels/room.nw
   (1 line removed, 1 added)", with `- BOARD 0 0 4 0 AAAAAAAA` and
   `+ BOARD 0 0 4 0 ABAAAAAA`.
3. The file on disk was changed to `BOARD 0 0 4 0 AAAAAAAC`. Upload:
   "Not uploaded: someone changed levels/room.nw on the server after you
   opened it", with the server's change shown as a diff
   (`- ...AAAAAAAA`, `+ ...AAAAAAAC`). The file on disk still held the
   other edit.
4. Open again, set the same tile, Upload: "Uploaded levels/room.nw." The
   file held `BOARD 0 0 4 0 ABAAAAAC` (both edits), and no temporary file
   was left in the folder.

## Not covered here

- The same over a live RC file link is covered by the loopback test
  `upload_is_refused_when_the_server_copy_changed_on_loopback`
  (`crates/lat-core/tests/rc_edit.rs`), not by a fork run: no fork build
  was available in this container.
- The check downloads the file again right before the upload. The file link
  has no compare-and-swap, so an edit that lands in that one round trip is
  not seen.
- The design's "keep the downloaded original in a local backup folder" is
  not done yet.
