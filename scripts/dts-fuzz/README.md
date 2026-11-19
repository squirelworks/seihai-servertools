# TGE `.dts` shape-loader fuzzer

A format-aware fuzz harness for the Torque Game Engine `.dts` shape format —
the model format the Graal3D / "3D mode" clients (4.04r2 and later, Torque
protocol 19+) load through `TSShape::read`.

We do **not** parse `.dts` anywhere in our own stack yet (the Graal3D 3D mode is
unmodelled in `seihai-core`), so there is no server-side loader to fuzz. This
harness instead exercises a TGE-lineage loader as a stand-in for the real
client's reader, to map the attack surface and produce concrete crashers that
can then be served to the real client in the lab.

## What it fuzzes

[SimonBoorer/libdts](https://github.com/SimonBoorer/libdts) (MIT), pinned at
`ebe9efc`. It is a clean, self-contained reimplementation of the TGE DTS reader
and **caps at shape version 24 — the same ceiling the 4.04r2 client enforces**,
which makes it a good proxy for the real loader's structure. Seeds are MIT
Torque3D v3.10.1 template shapes (all version 24).

Nothing from libdts or Torque3D is committed here; `build.sh` fetches both at
pinned revisions. No lab captures, client binaries or shape assets live on the
branch — only our own harness, mutator and the derived write-up.

## Files

| File | Purpose |
| --- | --- |
| `build.sh` | Fetch + build the ASan harness and download fuzz seeds. |
| `fuzz_dts.cpp` | Driver. libFuzzer entry point, plus a `-DSTANDALONE` `main()` that runs each file argument under ASan. |
| `dts_mutate.py` | Format-aware mutator: corrupts header offsets, section counts, the sequence count, plus bit-flips / truncation / extension. |
| `gen_repro.py` | Writes the minimal, deterministic reproducers (one per root cause) from a seed. |
| `triage.py` | Runs a corpus through the harness and buckets crashers by ASan type + crash site. |
| `FINDINGS.md` | The write-up: root cause, crash sites, and what it means for the real client. |

## Run

```sh
./build.sh                                   # builds ./fuzz_dts_asan, fetches seeds to .seeds/
./dts_mutate.py .seeds .corpus 2000 4242     # 2000 mutants, seed 4242
./triage.py .corpus                          # bucket the crashers
./gen_repro.py .seeds/octahedron.dts .repro  # the 5 minimal reproducers
```

The harness needs `clang++` (the libdts headers are MSVC-flavoured and want
`-fms-extensions -fdelayed-template-parsing`) and `g++` (its ASan runtime is
linked in, since clang's compiler-rt is often not installed). Both are standard.

## Scope / lab

The memory-safety findings here are code-only and complete. Confirming that the
minimal reproducers crash the **real** 4.04r2 client is a separate lab step: the
client is driven in a Mirae/GwenTea2 VM and fed the mutated `.dts` through our
own lab Graal3D server, never a public server. See `FINDINGS.md`.
