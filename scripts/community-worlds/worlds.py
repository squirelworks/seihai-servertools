#!/usr/bin/env python3
"""Helpers for scripts/community-worlds/build.sh: the parts of the community
test worlds that no servertool writes on its own.

    worlds.py sample-playerworld OUT
        A small original 2D playerworld (a start level with a house whose
        door leads into a one-room interior, and back), for seihai-pw3d.
    worlds.py grid OUT --graal-analysis DIR [--n 142] [--step 56]
        The distance-scoping test world: N x N red cubes STEP units apart on
        a flat streamed terrain (142 x 142 = 20,164 objects by default).
    worlds.py conformance OUT --graal-analysis DIR
        Every conformance case that needs no held content pack, no lab
        injector and no seihai-gserver-v2 as a server folder of its own.
    worlds.py server-txt DIR --name NAME [KEY=VALUE ...]
        Write DIR/config/server.txt for seihai-gserver (when the tool that
        made the world wrote none), or add KEY=VALUE lines to one.

Content made from nothing: plain shapes, solid-colour textures, flat
terrain. The grid and conformance worlds use Graal_analysis's
tools/conformance (gen_content.py, conformance.py), read from a checkout
given with --graal-analysis. Python 3.11+ standard library only.
"""

from __future__ import annotations

import argparse
import importlib.util
import shutil
import sys
from pathlib import Path

# Tile ids on Seihai's built-in tiledefs (seihai-core assets/default/
# tiledefs.txt), the ones seihai-pw3d reads unless --tiledefs names others:
# the first open, wall and water ids.
FLOOR, FLOOR_IN, WALL, WATER = 0, 1, 4, 264

B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


def code(i: int) -> str:
    return B64[(i >> 6) & 63] + B64[i & 63]


def board(tile) -> str:
    out = ["GLEVNW01"]
    for y in range(64):
        out.append(f"BOARD 0 {y} 64 0 " + "".join(code(tile(x, y)) for x in range(64)))
    return "\n".join(out) + "\n"


def sample_playerworld(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)

    # The house: walls from 24,14 to 39,25, a door gap at x 31-32 on its
    # south wall (y 25). A pond to the east, open ground all round.
    def town(x, y):
        if 24 <= x <= 39 and 14 <= y <= 25:
            edge = x in (24, 39) or y in (14, 25)
            if edge and not (y == 25 and 31 <= x <= 32):
                return WALL
            return WALL if not edge else FLOOR
        if 46 <= x <= 54 and 36 <= y <= 44:
            return WATER
        return FLOOR

    start = board(town)
    start += "LINK sample_house.nw 31 25 2 1 31.5 60\n"
    start += "SIGN 28 30\nWelcome to the Seihai 3D interiors sample.\nWalk through the door.\nSIGNEND\n"
    (out / "sample_start.nw").write_text(start, encoding="utf-8")

    # The interior: one room with walls round it, a table in the middle and
    # the door gap at the bottom (x 31-32, y 63) that leads back out.
    def room(x, y):
        edge = x in (0, 63) or y in (0, 63)
        if edge and not (y == 63 and 31 <= x <= 32):
            return WALL
        if 28 <= x <= 35 and 26 <= y <= 29:
            return WALL
        return FLOOR_IN

    house = board(room)
    house += "LINK sample_start.nw 31 63 2 1 31.5 26.5\n"
    house += "SIGN 30 40\nInside the house. The door behind you leads back out.\nSIGNEND\n"
    (out / "sample_house.nw").write_text(house, encoding="utf-8")


def load(ga: Path, name: str):
    path = ga / "tools" / "conformance" / f"{name}.py"
    if not path.exists():
        sys.exit(f"worlds.py: no {path}: give a Graal_analysis checkout with --graal-analysis")
    sys.path.insert(0, str(path.parent))
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


SERVER_HEAD = """# seihai-gserver settings for the {what}, from seihai-servertools
# scripts/community-worlds. Serve it with: seihai-gserver --world <this folder>
# A 5.x client links 3D only to a server whose name starts with `3 `.
# Other settings take their defaults (see seihai-gserver's world/config/server.txt).

name = {name}
graal3d = true
# 4.x to 6.x clients log in to a lobby first; set the port they use.
#lobby_port = 14900
"""


def server_txt(root: Path, name: str, what: str, extra: list[str]) -> None:
    cfg = root / "config" / "server.txt"
    cfg.parent.mkdir(parents=True, exist_ok=True)
    text = cfg.read_text(encoding="utf-8") if cfg.exists() else SERVER_HEAD.format(name=name, what=what)
    keys = {l.split("=", 1)[0].strip() for l in extra if "=" in l}
    kept = [l for l in text.splitlines() if l.split("=", 1)[0].strip() not in keys or l.lstrip().startswith("#")]
    cfg.write_text("\n".join(kept + extra) + "\n", encoding="utf-8")


def grid(out: Path, ga: Path, n: int, step: float) -> int:
    gc = load(ga, "gen_content")
    files = out / "files"
    files.mkdir(parents=True, exist_ok=True)
    content = gc.files()
    for name in ["conf_red.3ds", "conf_ground.png", "heights.info"] + \
            [f"conf_sky{i}.png" for i in range(6)]:
        (files / name).write_bytes(content[name])
    for rel, data in gc.flat_terrain("conf_ter").items():
        p = out / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(data)
    (out / "conf_ter" / "heights.info").write_bytes(content["heights.info"])
    lines = [
        f"# Distance-scoping test: {n} x {n} = {n * n} cubes, {step:g} units apart, on a flat terrain.",
        "sun 0.57735 0.57735 -0.57735 0.8 0.8 0.8 1 0.3 0.3 0.35 1",
        "sky conf_sky0,conf_sky1,conf_sky2,conf_sky3,conf_sky4,conf_sky5 1000",
        "skyoptions 1 0 0 0.4 0.6 0.9",
        "terrain conf_ter 32",
        # Off the grid lines and outside the 5.12's origin cell (|x|, |y| < 8).
        f"spawn {step / 2:g} {-step / 2 - 12:g} 1",
    ]
    for j in range(n):
        for i in range(n):
            x = (i - (n - 1) / 2) * step
            y = (j - (n - 1) / 2) * step
            lines.append(f"object G{j * n + i} conf_red.3ds {x:g} {y:g} 1")
    (out / "config").mkdir(parents=True, exist_ok=True)
    (out / "config" / "graal3dworld.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    server_txt(out, "3 Seihai Scoping Grid", "distance-scoping grid world", [
        "# Only objects near each player are sent: a 5.12 holds 4,096 ghosts.",
        "graal3d_scope = on",
        "graal3d_scoperadius = 1100",
        "graal3d_pushterrain = true",
    ])
    return n * n


# Cases left out of the community set, with the reason (README lists them).
EXCLUDE_WHY = {
    "needs": "needs a held third-party content pack (.dts/.dif), not redistributable",
    "v2": "runs on seihai-gserver-v2 in the lab runner's own layout",
    "inject": "needs the lab's UDP junk/flood injector",
}


def conformance(out: Path, ga: Path) -> tuple[list, list]:
    cf = load(ga, "conformance")
    kept, dropped = [], []
    stage = out / ".stage"
    for case in cf.load_cases(None):
        why = ("needs" if case.needs or case.derive else
               "v2" if case.server_kind == "v2" else
               "inject" if case.mid_action.startswith("inject") else "")
        if why:
            dropped.append((case, EXCLUDE_WHY[why]))
            continue
        root = cf.build_world(case, stage)
        dest = out / f"{case.id.lower()}-{case.slug}"
        if dest.exists():
            shutil.rmtree(dest)
        shutil.copytree(root, dest)
        add = (dest / "config" / "server.add").read_text(encoding="utf-8").splitlines()
        (dest / "config" / "server.add").unlink()
        (dest / "CASE.json").unlink()
        # The trace is a lab option; leave it on but say what it is.
        extra = [l for l in add if not l.startswith("name =")]
        extra = [("# lab trace of every 3D packet, read by the suite's checks\n" + l)
                 if l.startswith("graal3d_trace") else l for l in extra]
        server_txt(dest, case.lobby_name if case.lobby_name.startswith("3 ") else case.lobby_name,
                   f"conformance case {case.id} ({case.title})", [l for l in extra if l != "graal3d = true"])
        kept.append(case)
    shutil.rmtree(stage, ignore_errors=True)
    rows = ["| Case | Folder | What it checks | Sessions / clients |", "|---|---|---|---|"]
    for c in kept:
        rows.append(f"| {c.id} | `{c.id.lower()}-{c.slug}/` | {c.title} | {c.sessions} / {c.clients} |")
    rows += ["", "Left out:", ""]
    for c, why in dropped:
        rows.append(f"- {c.id} `{c.slug}`: {why}")
    (out / "CASES.md").write_text("\n".join(rows) + "\n", encoding="utf-8")
    shutil.copy(ga / "torque" / "conformance" / "EXPECTED.md", out / "EXPECTED.md")
    return kept, dropped


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("sample-playerworld")
    s.add_argument("out", type=Path)
    g = sub.add_parser("grid")
    g.add_argument("out", type=Path)
    g.add_argument("--graal-analysis", type=Path, required=True)
    g.add_argument("--n", type=int, default=142)
    g.add_argument("--step", type=float, default=56)
    c = sub.add_parser("conformance")
    c.add_argument("out", type=Path)
    c.add_argument("--graal-analysis", type=Path, required=True)
    t = sub.add_parser("server-txt")
    t.add_argument("dir", type=Path)
    t.add_argument("--name", required=True)
    t.add_argument("--what", default="3D world")
    t.add_argument("lines", nargs="*")
    a = ap.parse_args(argv)
    if a.cmd == "sample-playerworld":
        sample_playerworld(a.out)
    elif a.cmd == "grid":
        print(f"{grid(a.out, a.graal_analysis, a.n, a.step)} objects")
    elif a.cmd == "conformance":
        kept, dropped = conformance(a.out, a.graal_analysis)
        print(f"{len(kept)} cases, {len(dropped)} left out")
    elif a.cmd == "server-txt":
        server_txt(a.dir, a.name, a.what, [l.replace("=", " = ", 1) if " = " not in l else l for l in a.lines])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
