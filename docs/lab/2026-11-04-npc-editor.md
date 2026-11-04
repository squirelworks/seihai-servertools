# NPC editor page, level NPCs, 2026-11-04

What: the window's NPC editor (`seihai-servertools gui --world DIR --page
npcs`): open a level, pick an NPC, edit its script, Check, Apply to the
level, Show changes, Upload.

## Setup

- Linux, Xvfb at 1100x800, software GL. Clicks and keys were sent with
  XTest; the window was given keyboard focus first (there is no window
  manager under Xvfb, so it never gets focus on its own).
- World folder (a folder on this machine, not an RC login) with
  `levels/room.nw`: one board row and two NPCs, `door.png` at 10,12 with a
  three-line `playertouchsme` script, and `-` at 20,5 with a one-line
  `setimg lamp.png`.

## Result

1. List, pick `room.nw`, Open: "Opened levels/room.nw (148 bytes, 2 NPCs)",
   and the two NPCs as `door.png at 10,12 (3 lines)` and `- at 20,5 (1
   lines)`.
2. Picked the door NPC: its script in the editor.
3. Typed `play open.wav;` and an unclosed `if (x) {`, then Check:
   "GS1 problem, line 5: missing `}` (the client skips that part)" and
   "Names sound open.wav (line 3, play)". The page showed "Not applied yet."
4. Upload before Apply: refused with "The script in the editor is not
   applied yet. Apply it, or Revert, then upload."
5. Closed the block, Apply to level, then Check said "GS1: no parse
   problems."; Show changes listed the two added lines inside the NPC, and
   the NPC button now said 5 lines.
6. Upload: "Uploaded levels/room.nw." The file held the new script, the
   other NPC and the board row as before, and no temporary file was left.
   `seihai-servertools check` on the folder then listed `open.wav` and
   `lamp.png` as missing script files, as expected for this test world.

## Not covered here

- The refusal of a script line that is just `NPCEND`, and the refusal when
  the server's copy changed, are unit-tested (`level::tests`,
  `pages::review::tests`) and were shown for the level page in
  `docs/lab/2026-11-04-lat-refuse-upload.md`; not repeated here.
- A live RC link: no fork build in this container. The page uses the same
  `lat_core::remote` calls as the level page, which the loopback RC tests
  cover.
- Database NPCs, classes and weapons (NC login, `seihai-rc` stage 5).
