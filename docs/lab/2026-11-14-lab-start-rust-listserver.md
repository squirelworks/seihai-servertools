# Lab start with the Rust listserver and a seihai-gserver world (2026-11-14)

Roadmap #50, "a real run of `lab start` with the Rust gserver kind", now with
`seihai-listserver` in the same run (left open by
`2026-11-12-lab-start-rust.md`). This reruns a lab run that was cut short
when the lab host was reset. **Result: PASS.**

Machine: the Linux lab host, with no VM. Everything ran inside a
loopback-only network namespace (`bwrap --unshare-net`, entered with
`nsenter`), so the namespace had only `lo` and no route out.

## Builds

| Component | Source | Commit / version |
|---|---|---|
| seihai-servertools | main, after #73 | `b3e682f` (0.0.1) |
| seihai-listserver | main | `c37d5b4` (0.1.1) |
| seihai-gserver | main | `b827cd2` (0.1.0) |
| seihai-mae | seihai main, `crates/seihai-mae` | `4462cf5` (0.0.7), `--no-default-features --features dev` |

Each was built with `cargo build --release -j2` using rustc 1.93.1.

## World

```
W/
  seihai-servertools          target/release copy
  seihai-listserver           target/release copy
  seihai-listserver.toml      the repo's sample, with silent_list_after_ms = 500
  rust/                       seihai-gserver's sample world/, with two lines changed in config/server.txt:
                                port = 14806
                                listserver = 127.0.0.1:14900
```

`listserver_secure` was left at its default (`true`), so the gserver
link is encrypted.

## Run

```
cd W
seihai-servertools lab plan  --world . --rust rust --rust-gserver <path>/seihai-gserver
seihai-servertools lab start --world . --rust rust --rust-gserver <path>/seihai-gserver
export GRAAL_PW=labpw
seihai-mae --online 127.0.0.1:14922 --list-only --account test1 --password-env GRAAL_PW --version 2.22
seihai-mae --online 127.0.0.1:14922 --list-only --account test1 --password-env GRAAL_PW --version 1.41
# then SIGINT (Ctrl+C) to `lab start`
```

## Result table

| Step | Expected | Seen | Pass |
|---|---|---|---|
| `lab plan` | listserver + rust, no warning about the `listserver` line | `listserver ... (gservers register on 127.0.0.1:14900 ...)` / `rust  rust on 127.0.0.1:14806`, with no warning | PASS |
| `lab start`, listserver | listening on 14900 and on 14921 to 14923 | `listening on 127.0.0.1:14900 for gservers and 127.0.0.1:14921, 127.0.0.1:14922, 127.0.0.1:14923 for clients` | PASS |
| `lab start`, Rust gserver | listening on 14806, registered with the listserver | `Seihai Lab listening on 127.0.0.1:14806`, then `listserver link to 127.0.0.1:14900: registered (encrypted)` | PASS |
| 2.22 client login | gets the list with Seihai Lab | `Welcome to Lab listserver, test1.` / `There are 1 server(s) online.` / `0) [H] Seihai Lab  127.0.0.1:14806  2.31  players: 0` | PASS |
| 1.41 profile (silent list) | gets the list | `client 4 sent nothing; sending the list`; the client prints `0) [-] Seihai Lab  127.0.0.1:14806  2.31` | PASS |
| Ctrl+C | everything stops | `Stopping...`; gserver: `stopping: saved 0 online player(s)`; both servers exited, and nothing of the run was left | PASS |

## Console excerpts

`logs/listserver-console.txt`:

```
seihai-listserver 0.1.1 starting, config ./seihai-listserver.toml
listening on 127.0.0.1:14900 for gservers and 127.0.0.1:14921, 127.0.0.1:14922, 127.0.0.1:14923 for clients; accounts: open (every account gets in)
silent list (shape S): after 500 ms
gserver 1 connected from 127.0.0.1
gserver 1 closed (no REGISTERV3 line): (no name), entry dropped
gserver 2 connected from 127.0.0.1
gserver 2 encrypted link (SEI00001 SVLINK01)
gserver 2 register seihai-gserver 0.1.0
gserver 2 newserver Seihai Lab port 14806 version 2.31 ip 127.0.0.1
client 3 on 14922 from 127.0.0.1
client 3 handshake id 7, gen 5, token GNW30123, list newmain
client 3 account test1
client 3 closed
client 4 on 14922 from 127.0.0.1
client 4 sent nothing; sending the list
client 4 closed
```

`logs/rust-console.txt`:

```
Seihai Lab listening on 127.0.0.1:14806
listserver link: first visit to 127.0.0.1:14900; trusting its key SHA256:… (saved to ./rust/config/listserver.pin)
listserver link to 127.0.0.1:14900: registered (encrypted)
stopping: saved 0 online player(s)
```

## Notes

1. Gserver 1 is `lab start`'s readiness probe (`closed (no REGISTERV3
   line)`), as in the 2026-11-12 run. It is harmless.
2. The Rust gserver advertises version `2.31` in the list.
3. On first contact, the gserver pins the listserver key in
   `rust/config/listserver.pin`. A world folder that is reused with a
   listserver that has made a new `seihai-login.key` will need that pin
   deleted.
4. `seihai-mae --version` with no other arguments starts the game. It does
   not print a version.

## Not covered

- A real Windows client under Wine against this run. The silent-list path
  was covered by seihai-mae's 1.41 profile.
- Logging in to the Rust gserver itself after the list. This run covered
  only listserver login and the list.
