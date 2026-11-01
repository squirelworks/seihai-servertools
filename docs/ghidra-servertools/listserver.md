# Win32 ListServer

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | Packed `ListServer.exe` from the Server Lister archive |
| SHA-256 (packed) | `3e15bcc71ede779809bad4c05f46c6128793fed5135fec3120810d15a1928b40` |
| Image Ghidra loaded | UPX-unpacked copy of that PE |
| Ghidra | 12.2 PUBLIC, Delphi, 3299 functions, headless analysis succeeded |
| Imports that matter | `WSAStartup`, `socket`, `inet_addr`, `RegEnumValueA`, `ShellExecuteA` |

## Directory behaviour

The image and the bundled `servers.ini` agree on a small local directory. The options section names a show-more URL and a list of server ids. Each server section has name, address, port, description, language, version, players, URL, and a favorite flag. The bundled example entry is loopback on lab port 14900. The file's own comment says a favorite entry is sent to the client with a `P ` prefix so the client files it on the gold tab. Winsock listen/connect imports match a program that accepts client connections and answers with that list.

## Credential hand-off

The same image carries a second cluster, next to `ShellExecuteA`:

- Registry path `Software\Microsoft\Windows\CurrentVersion\Internet Settings`, value name `Drag`, marker text `WindowsUser@`.
- Registry path `Software\Graal\Client`.
- Program name `iexplore`, verb `Open`.
- A base64 text beside those strings. Decoded, it is an `http` URL with a query string. The host is not recorded here.

`RegEnumValueA` is imported, which is what a walk of every value under the client key needs. Together this is the 2003 listserver that reads saved classic client logins and asks the local browser to open a third-party URL with that text attached. The marker under Internet Settings is how it avoids repeating the hand-off.

Do not run the packed or unpacked PE. Servertools should not shell out to it, ship it, or treat it as a listserver implementation. If it has already been started on an operator machine, saved classic client passwords on that machine should be rotated.
