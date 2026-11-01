# Graal Random Walker

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | `Graal Random Walker.exe` |
| SHA-256 (packed) | `b41973d19fc0089c0b5b59e7477ad008354a4e6132204692b89ad7b5d5ea8f48` |
| Runtime | UPX, then compiled AutoIt v2.64. The script sits in a compressed container in the overlay. Ghidra 12.2: about 300 functions. Imports include `keybd_event` and `FindWindowA`. No Winsock or HTTP. |
| Script | Recovered statically after the container was unpacked in memory only (the PE was not started). |

## What it does

It looks for a top-level window titled `Graal Auto`.

1. A message says to focus that window within a few seconds. It waits 2 seconds. If the window is not active it shows "use the key first" and exits.
2. It shows a "Graal Water Get" ok/cancel box (same prompt family as the water tool). Cancel exits.
3. It shows "click ok to start random walker", waits for `Graal Auto`, and activates it.
4. The loop sets the key delay to 200 milliseconds, draws a random integer from 0 through 5, and sends five of one key: 1 is Up, 2 is Down, 3 is Left, 4 is Right, 5 is `S`. Zero sends nothing and loops again. If the window is not active it shows a waiting box, sleeps 1 second, and resumes when the window is active.

No registry, file, or network commands in the script. `OpenProcess` and `VirtualAlloc` are present as AutoIt runtime imports. There is no `WriteProcessMemory` or `CreateRemoteThread`.

## Defense

Movement and an occasional `S` burst, only while a window titled `Graal Auto` is in front. It is not a packet tool. Server-side speed and action limits still apply. A client whose window title is not `Graal Auto` never enters the loop.
