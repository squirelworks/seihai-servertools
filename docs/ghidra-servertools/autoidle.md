# AutoIdle

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | `AutoIdle.exe` from the autoprogs archive |
| SHA-256 (packed) | `ebf4a5913132fa8faca1647bb80aed2f57cde6f9e579bac71592093b3853a43b` |
| Runtime | UPX, then AutoIt v2.61. Ghidra 12.2 on the unpacked image: about 300 functions, no Winsock or HTTP imports |
| Script | Recovered statically from the unpacked image |

## What it does

One loop, then a goto back to the start. The body is only `Send` of `{LEFT}` and `{RIGHT}`. Most steps alternate one left and one right. A few stretches send the same arrow two or three times. There is no `Sleep`, no chat text, and no window check.

No registry, file, process, or network commands in the script. Same AutoIt runtime imports as AutoD (`keybd_event`, `FindWindowA`, registry, `OpenProcess`). No remote-thread or write-process imports.

## Defense

This is an anti-idle movement jitter, not an AFK chat bot. Phrase lists and chat rate limits do not see it. A session that oscillates facing or position with no other actions does.
