#!/usr/bin/env python3
"""Loopback hard-stress scenarios for seihai-listserver.

Complements `seihai-suite-stress run` (bot load against the whole suite) with
listserver-focused cases that those bots do not cover:

  flood       — hold many idle client sockets past max_connections
  slowloris   — one-byte partial frames past max_connections
  malformed   — truncated / oversized / bad-zlib client and gserver bytes
  login       — parallel shape-B list logins
  ramp        — many fake gservers with heartbeats and player-count updates
  accounts    — synthetic argon2 account files (hash once, reuse PHC)

Synthetic traffic only, on 127.0.0.1. Writes metrics JSON under --out.
Keep out/ folders out of the repo. Set LS_LEGACY_CONFIG=1 for older tags
that reject newer config keys.
"""

from __future__ import annotations

import argparse
import json
import os
import socket
import struct
import subprocess
import sys
import threading
import time
import zlib
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

LOOP = "127.0.0.1"


def gchar(n: int) -> int:
    return min(n, 223) + 32


def gstr(data: bytes) -> bytes:
    data = data[:223]
    return bytes([gchar(len(data))]) + data


def frame(body: bytes) -> bytes:
    if len(body) > 65535:
        raise ValueError(f"frame too large: {len(body)}")
    return struct.pack(">H", len(body)) + body


def zlib_frame(msgs: list[bytes]) -> bytes:
    raw = b"".join(m + b"\n" for m in msgs)
    return frame(zlib.compress(raw))


def register_line(name: bytes = b"lab") -> bytes:
    return bytes([gchar(30)]) + name + b"\n"


def newserver(name: bytes, port: bytes = b"14802") -> bytes:
    fields = [name, b"desc", b"English", b"3.0.9", b"http://x/", b"AUTO", port, b""]
    return bytes([gchar(22)]) + b"".join(gstr(f) for f in fields)


def hq_level(level: int = 2) -> bytes:
    return bytes([gchar(24), gchar(level)])


def plyradd(pid: int, account: bytes, nick: bytes) -> bytes:
    # PLYRADD 14 + GSHORT player + GCHAR kind + props ACCOUNTNAME(34)/NICKNAME(0)
    hi = ((pid >> 7) & 0x7F) + 32
    lo = (pid & 0x7F) + 32
    return (
        bytes([gchar(14), hi, lo, gchar(0)])
        + bytes([gchar(34)])
        + gstr(account)
        + bytes([gchar(0)])
        + gstr(nick)
    )


def sample_proc(pid: int) -> dict:
    out = {"pid": pid, "rss_kib": None, "threads": None, "fds": None, "cpu_utime": None}
    try:
        st = Path(f"/proc/{pid}/status").read_text()
        for line in st.splitlines():
            if line.startswith("VmRSS:"):
                out["rss_kib"] = int(line.split()[1])
            elif line.startswith("Threads:"):
                out["threads"] = int(line.split()[1])
        out["fds"] = len(list(Path(f"/proc/{pid}/fd").iterdir()))
        # utime+stime from /proc/pid/stat (fields 14,15) in clock ticks
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        out["cpu_utime"] = int(fields[11]) + int(fields[12])
    except Exception as e:  # noqa: BLE001
        out["error"] = str(e)
    return out


def start_ls(bin_path: Path, work: Path, max_conn: int, accounts: str = "", extra: dict | None = None) -> tuple[subprocess.Popen, int, int]:
    work.mkdir(parents=True, exist_ok=True)
    # pick free ports
    def free_port() -> int:
        s = socket.socket()
        s.bind((LOOP, 0))
        p = s.getsockname()[1]
        s.close()
        return p

    gport = free_port()
    cport = free_port()
    toml = work / "seihai-listserver.toml"
    legacy = os.environ.get("LS_LEGACY_CONFIG") == "1"
    lines = [
        f'bind = "{LOOP}"',
        f"gserver_port = {gport}",
        f"client_ports = [{cport}]",
        'list_name = "hard-stress"',
        f'accounts = "{accounts}"',
        f'log_file = "{work / "listserver.log"}"',
        f"max_connections = {max_conn}",
        "client_timeout_secs = 30",
        "gserver_timeout_secs = 300",
        "silent_list_after_ms = 500",
        "login_fail_limit = 0",
        "status_log_secs = 5",
        'secure_login_key = ""',
    ]
    if not legacy:
        lines.extend([
            "gserver_ping_secs = 5",
            'machines = ""',
            'profiles = ""',
            'buddies = ""',
            'data = ""',
        ])
    if extra:
        for k, v in extra.items():
            lines.append(f"{k} = {v}")
    toml.write_text("\n".join(lines) + "\n")
    log = open(work / "console.log", "w")  # noqa: SIM115
    proc = subprocess.Popen(
        [str(bin_path), "--config", str(toml)],
        cwd=str(work),
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    # wait for listen
    deadline = time.time() + 10
    while time.time() < deadline:
        try:
            socket.create_connection((LOOP, cport), timeout=0.2).close()
            socket.create_connection((LOOP, gport), timeout=0.2).close()
            break
        except OSError:
            if proc.poll() is not None:
                raise RuntimeError(f"listserver exited early: {proc.returncode}")
            time.sleep(0.05)
    else:
        proc.kill()
        raise RuntimeError("listserver ports never opened")
    return proc, gport, cport


def shape_a_login(
    cport: int,
    account: bytes | None = b"bot",
    password: bytes = b"pw",
    timeout: float = 5.0,
) -> tuple[float, int, bool]:
    """Shape A (id0 only) or B (id0+id1) in one zlib frame. Returns (ms, bytes_in, ok)."""
    t0 = time.perf_counter()
    s = socket.create_connection((LOOP, cport), timeout=timeout)
    s.settimeout(timeout)
    try:
        msg = bytes([gchar(0)]) + b"newmain\n"
        if account is not None:
            msg += bytes([gchar(1)]) + gstr(account) + gstr(password) + b"\n"
        s.sendall(frame(zlib.compress(msg)))
        data = b""
        while True:
            try:
                chunk = s.recv(65536)
            except socket.timeout:
                break
            if not chunk:
                break
            data += chunk
            if len(data) >= 4:
                # one reply frame is enough for timing
                break
        ms = (time.perf_counter() - t0) * 1000
        return ms, len(data), len(data) > 0
    finally:
        s.close()


def register_gserver(gport: int, name: bytes, hold: list, players: int = 0) -> socket.socket:
    s = socket.create_connection((LOOP, gport), timeout=5)
    s.settimeout(5)
    s.sendall(register_line(name[:20]))
    msgs = [hq_level(2), newserver(name)]
    for i in range(players):
        acc = f"p{i}".encode()
        msgs.append(plyradd(i + 1, acc, acc))
    s.sendall(zlib_frame(msgs))
    hold.append(s)
    return s


def heartbeat_loop(socks: list, stop: threading.Event, interval: float, player_updates: bool):
    n = 0
    while not stop.wait(interval):
        dead = []
        for i, s in enumerate(list(socks)):
            try:
                # empty ping id 99 as pong-style keepalive from gserver side is svo::PING
                # gserver sends PING in response; we can send SETIP or a tiny NEWSERVER refresh
                # Use a player-count style: PLYRADD a rotating player when asked
                if player_updates:
                    msg = plyradd(1, b"hb", f"hb{n}".encode())
                    s.sendall(zlib_frame([msg]))
                else:
                    s.sendall(zlib_frame([bytes([gchar(16)])]))  # empty pong-ish / ping id 16
            except OSError:
                dead.append(s)
        for s in dead:
            try:
                socks.remove(s)
            except ValueError:
                pass
        n += 1


def list_latency(cport: int, samples: int = 20) -> dict:
    times = []
    sizes = []
    fails = 0
    for i in range(samples):
        try:
            ms, n, ok = shape_a_login(cport, account=f"lat{i}".encode())
            if ok:
                times.append(ms)
                sizes.append(n)
            else:
                fails += 1
        except OSError:
            fails += 1
    times.sort()
    def pct(p):
        if not times:
            return None
        return times[min(len(times) - 1, int(len(times) * p / 100))]
    return {
        "samples": samples,
        "ok": len(times),
        "fails": fails,
        "ms_p50": pct(50),
        "ms_p95": pct(95),
        "ms_max": times[-1] if times else None,
        "bytes_p50": sorted(sizes)[len(sizes) // 2] if sizes else None,
    }


def scenario_flood_max(bin_path: Path, out: Path, max_conn: int = 32) -> dict:
    work = out / "flood-max"
    proc, gport, cport = start_ls(bin_path, work, max_conn=max_conn)
    held = []
    accepted = 0
    refused = 0
    try:
        # Hold max_conn + extra half-open / idle sockets on client port
        target = max_conn * 3
        for i in range(target):
            try:
                s = socket.create_connection((LOOP, cport), timeout=2)
                s.settimeout(0.1)
                held.append(s)
                accepted += 1
            except OSError:
                refused += 1
        time.sleep(0.5)
        # try one more real login
        login_ok = False
        try:
            _, _, login_ok = shape_a_login(cport, timeout=2)
        except OSError:
            login_ok = False
        samp = sample_proc(proc.pid)
        log = (work / "listserver.log").read_text(errors="replace") if (work / "listserver.log").exists() else ""
        refused_lines = [ln for ln in log.splitlines() if "refused a link" in ln]
        return {
            "scenario": "connection_flood_past_max",
            "max_connections": max_conn,
            "connect_attempts": target,
            "connect_accepted_local": accepted,
            "connect_errors": refused,
            "held_sockets": len(held),
            "login_while_flooded": login_ok,
            "refused_log_lines": len(refused_lines),
            "proc": samp,
        }
    finally:
        for s in held:
            try:
                s.close()
            except OSError:
                pass
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


def scenario_slowloris(bin_path: Path, out: Path, max_conn: int = 32, n: int = 64) -> dict:
    work = out / "slowloris"
    proc, gport, cport = start_ls(bin_path, work, max_conn=max_conn)
    held = []
    try:
        for _ in range(n):
            try:
                s = socket.create_connection((LOOP, cport), timeout=2)
                # send partial length header only (half-open / slowloris)
                s.sendall(b"\x00")
                held.append(s)
            except OSError:
                break
        time.sleep(2)
        # Can a complete client still get in?
        late_ok = False
        try:
            _, _, late_ok = shape_a_login(cport, timeout=3)
        except OSError:
            late_ok = False
        samp = sample_proc(proc.pid)
        log = (work / "listserver.log").read_text(errors="replace") if (work / "listserver.log").exists() else ""
        return {
            "scenario": "slowloris_half_open",
            "max_connections": max_conn,
            "partial_sockets": len(held),
            "login_after": late_ok,
            "refused_log_lines": sum(1 for ln in log.splitlines() if "refused a link" in ln),
            "proc": samp,
        }
    finally:
        for s in held:
            try:
                s.close()
            except OSError:
                pass
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


def scenario_malformed(bin_path: Path, out: Path) -> dict:
    work = out / "malformed"
    proc, gport, cport = start_ls(bin_path, work, max_conn=256)
    cases = []
    payloads = {
        "truncated_len": b"\x00\x10" + b"x" * 4,  # claims 16, sends 4 then close
        "oversized_claim_empty": b"\xff\xff",  # claims 65535 then close
        "bad_zlib": frame(b"not-zlib-data-!!!!"),
        "zero_len_frame": b"\x00\x00",
        "random_4k": os.urandom(4096),
        "gserver_oversized_register": b"X" * 2000 + b"\n",
        "gserver_bad_zlib": None,  # filled below after connect
    }
    try:
        for name, blob in payloads.items():
            if name.startswith("gserver_"):
                s = socket.create_connection((LOOP, gport), timeout=2)
                s.settimeout(1)
                if name == "gserver_oversized_register":
                    try:
                        s.sendall(blob)
                        s.recv(16)
                        cases.append({"case": name, "survived": proc.poll() is None, "error": None})
                    except OSError as e:
                        cases.append({"case": name, "survived": proc.poll() is None, "error": str(e)})
                else:
                    try:
                        s.sendall(register_line(b"badz"))
                        s.sendall(frame(b"nope"))
                        s.recv(16)
                        cases.append({"case": name, "survived": proc.poll() is None, "error": None})
                    except OSError as e:
                        cases.append({"case": name, "survived": proc.poll() is None, "error": str(e)})
                s.close()
            else:
                s = socket.create_connection((LOOP, cport), timeout=2)
                s.settimeout(1)
                try:
                    s.sendall(blob)
                    try:
                        s.recv(64)
                    except OSError:
                        pass
                    cases.append({"case": name, "survived": proc.poll() is None, "error": None})
                except OSError as e:
                    cases.append({"case": name, "survived": proc.poll() is None, "error": str(e)})
                s.close()
            if proc.poll() is not None:
                break
        # server should still answer a clean login
        ok = False
        if proc.poll() is None:
            try:
                _, _, ok = shape_a_login(cport)
            except OSError:
                ok = False
        return {
            "scenario": "malformed_truncated_oversized",
            "cases": cases,
            "alive_after": proc.poll() is None,
            "clean_login_after": ok,
            "proc": sample_proc(proc.pid) if proc.poll() is None else {"dead": True, "code": proc.returncode},
        }
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()


def scenario_gserver_ramp(bin_path: Path, out: Path, counts: list[int], max_conn: int) -> dict:
    work = out / "gserver-ramp"
    proc, gport, cport = start_ls(bin_path, work, max_conn=max_conn)
    results = []
    hold: list[socket.socket] = []
    stop = threading.Event()
    hb = threading.Thread(target=heartbeat_loop, args=(hold, stop, 2.0, True), daemon=True)
    hb.start()
    try:
        prev = 0
        for n in counts:
            t0 = time.perf_counter()
            errors = 0
            for i in range(prev, n):
                name = f"W{i}".encode()
                try:
                    register_gserver(gport, name, hold, players=3)
                except OSError:
                    errors += 1
                    break
            ramp_s = time.perf_counter() - t0
            time.sleep(0.5)
            lat = list_latency(cport, samples=15)
            samp = sample_proc(proc.pid)
            results.append(
                {
                    "registered_target": n,
                    "held": len(hold),
                    "ramp_errors": errors,
                    "ramp_secs": round(ramp_s, 3),
                    "list": lat,
                    "proc": samp,
                }
            )
            # stop escalating if we cannot hold more
            if errors and len(hold) < n * 0.5:
                break
            prev = len(hold)
        return {"scenario": "gserver_capacity_ramp", "max_connections": max_conn, "steps": results}
    finally:
        stop.set()
        for s in hold:
            try:
                s.close()
            except OSError:
                pass
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


def scenario_login_storm(bin_path: Path, out: Path, n: int = 500, workers: int = 64) -> dict:
    work = out / "login-storm"
    proc, gport, cport = start_ls(bin_path, work, max_conn=1024)
    # one registered server so lists are non-empty
    hold = []
    register_gserver(gport, b"StormWorld", hold, players=5)
    times = []
    fails = 0
    t0 = time.perf_counter()

    def one(i: int):
        try:
            return shape_a_login(cport, account=f"u{i}".encode(), timeout=10)
        except OSError as e:
            return None

    with ThreadPoolExecutor(max_workers=workers) as ex:
        futs = [ex.submit(one, i) for i in range(n)]
        for f in as_completed(futs):
            r = f.result()
            if r is None or not r[2]:
                fails += 1
            else:
                times.append(r[0])
    elapsed = time.perf_counter() - t0
    times.sort()
    samp = sample_proc(proc.pid)
    for s in hold:
        s.close()
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()

    def pct(p):
        if not times:
            return None
        return round(times[min(len(times) - 1, int(len(times) * p / 100))], 2)

    return {
        "scenario": "login_storm",
        "n": n,
        "workers": workers,
        "ok": len(times),
        "fails": fails,
        "elapsed_s": round(elapsed, 3),
        "logins_per_s": round(len(times) / elapsed, 1) if elapsed else None,
        "ms_p50": pct(50),
        "ms_p95": pct(95),
        "ms_max": round(times[-1], 2) if times else None,
        "proc": samp,
    }


def make_accounts(path: Path, n: int, password: bytes = b"labpw") -> dict:
    """Seed synthetic accounts file. Hash once, reuse PHC for all rows."""
    # Use listserver binary's add-account for one hash, or argon2 via hashlib? Prefer calling binary.
    # Generate with a tiny Rust-less approach: invoke `seihai-listserver add-account` once then replicate.
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".one")
    if tmp.exists():
        tmp.unlink()
    # caller passes bin via env
    bin_path = Path(os.environ["LS_BIN"])
    env = os.environ.copy()
    env["SEIHAI_LAB_PASSWORD"] = password.decode()
    subprocess.check_call(
        [
            str(bin_path),
            "add-account",
            str(tmp),
            "seed0",
            "--password-env",
            "SEIHAI_LAB_PASSWORD",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=env,
    )
    line = tmp.read_text().strip().splitlines()[0]
    _acc, phc = line.split(None, 1)
    t0 = time.perf_counter()
    with path.open("w") as f:
        for i in range(n):
            f.write(f"acc{i} {phc}\n")
    write_s = time.perf_counter() - t0
    size = path.stat().st_size
    tmp.unlink(missing_ok=True)
    return {"n": n, "path": str(path), "bytes": size, "write_secs": round(write_s, 3), "phc_reused": True}


def scenario_large_accounts(bin_path: Path, out: Path, sizes: list[int]) -> dict:
    os.environ["LS_BIN"] = str(bin_path)
    steps = []
    for n in sizes:
        work = out / f"accounts-{n}"
        work.mkdir(parents=True, exist_ok=True)
        acc = work / "accounts.txt"
        seed = make_accounts(acc, n)
        # startup / load
        t0 = time.perf_counter()
        proc, gport, cport = start_ls(bin_path, work / "run", max_conn=256, accounts=str(acc))
        start_s = time.perf_counter() - t0
        # wait until log mentions account count or just probe
        time.sleep(0.2)
        samp0 = sample_proc(proc.pid)
        # lookup existing + missing
        t_ok = []
        t_miss = []
        for i in range(30):
            try:
                ms, _, ok = shape_a_login(cport, account=f"acc{i}".encode(), password=b"labpw", timeout=30)
                if ok:
                    t_ok.append(ms)
            except OSError:
                pass
            try:
                ms, _, ok = shape_a_login(cport, account=f"missing{i}".encode(), password=b"labpw", timeout=30)
                t_miss.append(ms)
            except OSError:
                pass
        samp1 = sample_proc(proc.pid)
        proc.terminate()
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.kill()
        t_ok.sort()
        t_miss.sort()
        steps.append(
            {
                "accounts": n,
                "seed": seed,
                "startup_secs": round(start_s, 3),
                "rss_kib_after_start": samp0.get("rss_kib"),
                "rss_kib_after_logins": samp1.get("rss_kib"),
                "login_hit_ms_p50": t_ok[len(t_ok) // 2] if t_ok else None,
                "login_hit_ms_max": t_ok[-1] if t_ok else None,
                "login_miss_ms_p50": t_miss[len(t_miss) // 2] if t_miss else None,
                "login_ok": len(t_ok),
            }
        )
    return {"scenario": "large_account_stores", "steps": steps}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--label", default="main")
    ap.add_argument(
        "--only",
        default="flood,slowloris,malformed,login,ramp,accounts",
        help="comma scenarios",
    )
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    only = {x.strip() for x in args.only.split(",") if x.strip()}
    report = {"label": args.label, "bin": args.bin.name, "started": time.time(), "results": []}

    if "flood" in only:
        report["results"].append(scenario_flood_max(args.bin, args.out, max_conn=32))
    if "slowloris" in only:
        report["results"].append(scenario_slowloris(args.bin, args.out, max_conn=32, n=96))
    if "malformed" in only:
        report["results"].append(scenario_malformed(args.bin, args.out))
    if "login" in only:
        report["results"].append(scenario_login_storm(args.bin, args.out, n=400, workers=80))
    if "ramp" in only:
        # raise max_connections for capacity; 10k needs headroom for sockets+threads
        report["results"].append(
            scenario_gserver_ramp(
                args.bin,
                args.out,
                counts=[10, 100, 1000, 5000, 10000],
                max_conn=12000,
            )
        )
    if "accounts" in only:
        # 1M may be large; do 100k then 1M if disk/time allow
        report["results"].append(scenario_large_accounts(args.bin, args.out, sizes=[100_000, 1_000_000]))

    report["ended"] = time.time()
    path = args.out / f"report-{args.label}.json"
    path.write_text(json.dumps(report, indent=2))
    print(json.dumps({"wrote": str(path), "scenarios": [r.get("scenario") for r in report["results"]]}, indent=2))


if __name__ == "__main__":
    main()
