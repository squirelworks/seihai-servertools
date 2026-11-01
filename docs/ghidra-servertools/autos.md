# AutoS

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | `AutoS.exe` from the autoprogs archive |
| SHA-256 (packed) | `7e427085717c47e2e1f11e45abfc448613730cd1f20f41a8f6a29a8dfd3a88ad` |
| Runtime | UPX, then AutoIt v2.61. Ghidra 12.2 on the unpacked image: about 300 functions, no Winsock or HTTP imports |
| Script | Recovered statically. Three lines. |

## What it does

`Repeat` forever, `Send` of the two characters `s#`, `EndRepeat`. No sleep, no window check, no message box.

No registry, file, process, or network commands in the script. Runtime imports match the other autoprogs (`keybd_event` and friends). No injection imports.

## Defense

A held attack or shortcut key, as fast as the AutoIt sender runs. Same class as AutoD: client keystrokes, server-side action cooldown.
