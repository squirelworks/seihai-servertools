# Graal WaterGet

Static read only. The PE was not executed.

| | |
| --- | --- |
| Sample | `Graal WaterGet.exe` (v1.1 archive) |
| SHA-256 (packed) | `7482de4824d370f1c9f5b73323470db90ae2fac69fca40e8ce14626a29481da0` |
| Runtime | UPX, then AutoIt v2.63 (script container marked v2.62). Ghidra 12.2: about 300 functions, no Winsock or HTTP imports |
| Script | Recovered statically. The bundled howto is the operator steps, not the key loop. |

## What it does

A message box asks the operator to click ok when ready, and names RpgWarrior. Cancel exits. It then sets the key delay to 300 milliseconds and loops: `Send` `{DOWN 2}`, `Send` the letter `a`, `Sleep` 2000 milliseconds.

The howto tells the operator to equip a water can, stand on a water source facing down, focus the client, and start the program. The script itself supplies the down tap and the use key. It does not open sockets or read the registry. The runtime still imports `keybd_event`, `FindWindowA`, registry functions, and `OpenProcess`, and it has a tray icon.

## Defense

One use action every two seconds, plus a facing key. A per-account gather cooldown longer than that wait, or a yield cap, stops the farm. The client is unmodified.
