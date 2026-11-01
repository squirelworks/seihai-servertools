# Static notes for server tools and trainers

Date: 2026-11-01. Ghidra **12.2 PUBLIC** headless (`analyzeHeadless`) on Temurin **25.0.5+8**. Copies were unpacked with UPX where the packer was intact. No recovered PE was executed. No project database or binary is in this repo.

These notes fill gaps the existing `recovered-*/ghidra-analysis` summaries do not answer: what the remaining AutoIt macros actually send, and how the Win32 listserver treats saved client logins. They are behaviour only.

| Note | Sample | Why it was re-read |
| --- | --- | --- |
| [listserver.md](listserver.md) | Win32 `ListServer.exe` | Prior summary describes a directory UI. It does not describe the registry read and browser hand-off. |
| [autod.md](autod.md) | `AutoD.EXE` | Confirm the autoprogs family is a macro, and which key it sends. |
| [autoidle.md](autoidle.md) | `AutoIdle.exe` | Same, for the idle program. |
| [autos.md](autos.md) | `AutoS.exe` | Same, for the short program. |
| [waterget.md](waterget.md) | `Graal WaterGet.exe` | Remaining water-can loop: key and wait. |
| [random-walker.md](random-walker.md) | `Graal Random Walker.exe` | Remaining walker: which keys, and whether it checks the client window. |
| [capslock.md](capslock.md) | `capslock.exe` | Packed loader in the trainers folder. Check for anything beyond a caps-lock toggle. |

## Not re-opened here

- `Graal_Miner` archives stay blocked by the Drive abuse flag. Not downloaded.
- Graal Digger remains the truncated UPX sample already summarized.
- Graal Prayer and `data.exe` script bodies stay encrypted in the prior summary. This pass did not open them again.
- Graal Load and AFK Scroll Bot were not imported in this pass. The earlier AFK Bot summary still covers the phrase bot; the scroll sibling and Graal Load stay on the prior string notes.

## What a checker can use

- Dig, water, and attack macros are client keystrokes. A world checker cannot see them. Rate limits belong on the game server.
- AutoIdle never chats. It oscillates the arrow keys. Chat-phrase rules will not see it.
- The Win32 listserver is a desktop program. Do not run it. If it has been run on a machine that stored a classic client login, treat that login as exposed. Servertools does not need a parser for it.
