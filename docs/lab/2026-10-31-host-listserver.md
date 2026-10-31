# Lab run: Host a listserver, 2026-10-31

Where: cloud container, Linux, loopback only. `127.0.0.2` stood in for an
"outside" interface.

What: `seihai-servertools setup host-listserver` (setup step 6) on an empty
world folder, with the real `seihai-listserver` (`main` at the time, release
build) for `add-account` and for running the result.

```
GRAAL_PW=labpw seihai-servertools setup host-listserver --world . --yes \
  --bind 127.0.0.2 --first-account test1 --password-env GRAAL_PW \
  --listserver-bin <path>/seihai-listserver
seihai-listserver --config seihai-listserver.toml
seihai-servertools setup test-connection 127.0.0.2:14922
seihai-servertools setup test-connection 127.0.0.1:14922
```

Results:

- The review listed bind, gserver_port, client_ports, accounts and
  gserver_ip (all new), warned that port 14900 has no allow-list and that
  `accounts.txt` did not exist yet, and saved.
- `add-account` wrote an argon2id line for `test1` to `accounts.txt`.
- The listserver started with the file: `listening on 127.0.0.2:14900 for
  gservers and 127.0.0.2:14921, 127.0.0.2:14922, 127.0.0.2:14923 for
  clients; accounts: 1 account(s) from accounts.txt`.
- Test connection: `127.0.0.2:14922: open`, `127.0.0.1:14922: refused`.
- In the window (Xvfb), the Host a listserver review showed the same diff
  and the two warnings, with Save enabled.

Not checked: a gserver registering with it and a client login through it
(needs a built fork; see `docs/NEXT.md`).
