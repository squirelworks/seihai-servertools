# Folder picker, 2026-11-04

What: a Browse... button next to every world folder field in the window
(Lab world, Server setup, World check, and the level, NPC and gani editors'
folder). It opens a small "Pick a folder" window over
`setup_core::browse`: the current folder, Up, Use this folder, and the
subfolders, with world folders marked.

## Setup

- Linux, Xvfb at 1100x800, software GL, XTest clicks.
- The window started with no `--world`, from a scratch folder holding
  `worlds/alpha` (a world: `servers/default/config/serveroptions.txt`) and
  `worlds/notes` (a plain folder).

## Result

1. Server setup, Browse...: the picker opened on the current folder (the
   field was empty), listing `worlds`.
2. Into `worlds`: `alpha (world folder)` and `notes`.
3. Into `alpha`: "This is a world folder.", one subfolder `servers`.
4. Use this folder: the picker closed, the field held the folder, and the
   page opened `servers/default` ("Lab (loopback only)") with its form
   filled, as the Open button does.
5. World check, Lab world and the level editor showed the Browse... button
   next to their folder fields.

## Notes

- No native dialog crate: the picker is plain egui and `std::fs`, so it
  builds and looks the same everywhere, and the headless build is
  unchanged.
- Hidden folders (`.name`) are not listed. Typing a path still works.
