# capslock

Static read only. The PE was not executed, and the packer was not run to unpack it.

| | |
| --- | --- |
| Sample | `capslock.exe` |
| SHA-256 | `2576cbeacd5ed30ff2fc02be1e8004d6fb4c380d1df3725fd7a09ab708b4ac7d` |
| Packing | Ten sections, including `.aspack`. UPX does not recognize it. |
| Ghidra | 12.2 PUBLIC. Delphi language id on the stub. Four functions. Analysis did not recover the packed body. |

## What is visible without executing

Ghidra's import list for the stub is `LoadLibraryA`, `GetProcAddress`, `GetModuleHandleA`, `keybd_event`, `GetKeyboardType`, `RegQueryValueExA`, plus small COM and GDI helpers. There is no socket DLL, no `OpenProcess`, no `FindWindow`, no `WriteProcessMemory`.

A version-resource string in the same image says the program turns the caps-lock key on and off. The bundled readme (plain HTML, not a PE) says the same: CapsLock v1.00, a toggle, with command-line options. No Graal window title or game string appears in the packed image.

`LoadLibraryA` and `GetProcAddress` are the packer stub. They are not, by themselves, a second program. Nothing in the surviving strings names another payload.

## Defense

Treat it as a caps-lock utility that was filed under trainers. It is not evidence of a Graal macro or a downloader. Do not execute it to finish the unpack. Game-server checks do not need a signature for it.
