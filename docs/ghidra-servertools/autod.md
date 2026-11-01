# AutoD

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | `AutoD.EXE` from the autoprogs archive |
| SHA-256 (packed) | `5a4493d79d0619a930413bd92bffd09935b11dcb33ce0e9e21270ab717d0be6d` |
| Runtime | UPX, then AutoIt v2.60. Ghidra 12.2 on the unpacked image: about 300 functions, no Winsock or HTTP imports |
| Script | Recovered statically from the unpacked image |

## What it does

It shows a yes/no box asking whether to run. No exits. It then tells the operator the program was made in 2001 and to use the tray icon to stop. The loop is `Sleep` 1 millisecond and `Send` of the letter `d`, forever.

The script has no registry, file, process, or network commands. The AutoIt runtime still imports `keybd_event`, `FindWindowA`, `SendMessageA`, registry calls, and `OpenProcess`. Those imports are the interpreter, not extra stages in this script. There is no `WriteProcessMemory` or `CreateRemoteThread`.

## Defense

This is a dig-key macro, not a client patch. A server sees a stream of the dig action with almost no pause. Chat limits will not catch it. Gather cooldowns will.
