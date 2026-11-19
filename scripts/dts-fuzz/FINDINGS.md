# Findings — fuzzing the TGE `.dts` shape loader

Harness: `./build.sh` then `./dts_mutate.py .seeds .corpus 2000 4242` and
`./triage.py .corpus`. Target: SimonBoorer/libdts `ebe9efc` (MIT, version-24
cap, the same cap the 4.04r2 Graal3D client enforces). Seeds: four MIT Torque3D
v3.10.1 template shapes. ASan build, little-endian x86-64.

## Result

**1089 of 2000 single-pass mutants crash.** They nearly all reduce to one root
cause: `TSShape::read` trusts the file header and section counts completely and
validates nothing before using them as allocation sizes, pointer offsets and
copy lengths.

Crash buckets from one 2000-input run (`triage.py`):

| count | class | site |
| --- | --- | --- |
| 278 | guard assert `check16` | debug-only; see note |
| 217 | allocation-size-too-big | `DTSShape.cpp:509` — `sequences_.resize(num_sequences)` |
| 177 | **SEGV (wild read)** | `DTSShape.cpp:187` — guard read via `mem_buffer16/8` |
| 110 | guard assert `check8` | debug-only |
|  80 | guard assert `sz <= kMaxSetDWords` | `TSIntegerSet::read`, debug-only |
|  71 | guard assert `check32` | debug-only |
|  65 | allocation DoS (`std::vector` resize) | various counts |
|  44 | allocation-size-too-big | `DTSShape.cpp:496` — `new int32_t[size_mem_buffer]` |
|  25 | **heap-buffer-overflow** | `DTSShape.cpp:187` |
| ~20 | heap-buffer-overflow / SEGV | `DTSShape.cpp:149/260/270/273/289/413/424`, `DTSMesh.cpp:463/519/532`, `DTSMesh.h:89`, `DTSVector.h:56` — section/mesh/skin reads |

## Root cause

The header is three unvalidated dwords (`DTSShape.cpp:492`–`499`):

```c
stream.Read(&size_mem_buffer);
stream.Read(&startU16);
stream.Read(&startU8);
int32_t* tmp = new int32_t[size_mem_buffer];          // (1) size not bounded
stream.Read(tmp, sizeof(int32_t) * size_mem_buffer);
mem_buffer16 = (int16_t*)(tmp + startU16);            // (2) offset not bounded
mem_buffer8  = (int8_t *)(tmp + startU8);             // (2)
count16 = startU8  - startU16;                         // (3) may go negative
count8  = size_mem_buffer - startU8;                   // (3)
```

Then `AssembleShape()` walks `tmp`/`mem_buffer16`/`mem_buffer8` with the element
counts stored *inside* that buffer, through `IMemBuffer::Get()` /
`CopyToShape()` (`DTSShapeAlloc.h`), which are a bare `*p++` and a `memcpy` with
no bounds check of any kind. The only integrity check is a periodic guard word
(`CheckGuard`).

Consequences, all reachable from a single hostile file:

1. **Attacker-controlled allocation.** `size_mem_buffer` and the section counts
   (`num_sequences`, `numNodes`, `numMeshes`, …) are used directly as `new[]` /
   `std::vector::resize` sizes → multi-GB allocations and short reads.
2. **Out-of-bounds reads.** `startU16` / `startU8` past the buffer make
   `mem_buffer16/8` point outside the allocation; the first guard read then
   dereferences a wild or just-past-the-end pointer → SEGV / heap-buffer-overflow.
   `startU16 > startU8` makes `count16` negative and the 16-bit reads walk
   backwards out of bounds.
3. **OOB in section/mesh assembly.** Over-large `numNodes`, `numMeshes`,
   `numNames`, skin counts, integer-set sizes, etc. drive OOB `memcpy`s deeper in
   `AssembleShape` / `TSMesh::assemble` even when the header offsets are sane.

## Note on the guard asserts — why the real client is likely worse

The `check16` / `check8` / `check32` / `kMaxSetDWords` buckets (~465 of the 1089)
are `assert()`s that fire in this debug build. **`assert` compiles out under
`NDEBUG`**, and the shipped 4.04r2 client is a release build. So in the real
client those inputs do **not** stop at the guard — they continue straight into
the unchecked `memcpy` / short-buffer path above. In other words, a debug-build
"assert" here is a candidate memory-safety crash in the real client.

## Minimal reproducers

`./gen_repro.py .seeds/octahedron.dts .repro` writes five files, each a valid
version-24 shape with exactly one header field corrupted:

| file | corruption | debug-build (libdts) result | real 4.04r2 client (lab) |
| --- | --- | --- | --- |
| `R1_startU8_wild.dts` | `startU8 = 0x40000000` | SEGV (wild read) | **handled** — SEH-guarded, link holds |
| `R2_startU8_oob.dts` | `startU8 = size+4` | heap-buffer-overflow (read) | **handled** — SEH-guarded, link holds |
| `R3_start16_gt_start8.dts` | `startU16 > startU8` | `check16` assert | **crash** — access violation (read) in the engine DLL, 3D link drops |
| `R4_size_huge.dts` | `sizeMemBuffer = 0x3fffffff` | `check32` assert after a 4 GB alloc | **crash** — access violation on an OOB **write** while filling the buffer |
| `R5_numseq_huge.dts` | `num_sequences = 0x3fffffff` | allocation-size-too-big | **crash (worst)** — bad count drives execution to a near-null address |

These are the payloads served to the real client in the lab.

## Lab result — real 4.04r2 client

The five reproducers were served to the real 4.04r2 client (Wine, probelab VM)
through our own lab Graal3D server — never a public server — one client session
per file, with a clean control run first and the good shape restored between
runs. Outcome:

- **R1 and R2 are handled, not crashes.** The `startU8` header reads happen
  inside `Graal4.exe` and are wrapped in structured exception handling, so the
  wild / just-past-the-end read is caught: the sky still renders and the link
  holds. So the client's own `TSShape::read` offset-trust path is SEH-guarded,
  and the OOB-read prediction from the libdts build does **not** carry over to
  4.04r2 for those two.
- **R3, R4 and R5 crash the real client.** Each raises an access violation in
  the Graal3D engine DLL that drops the 3D link. These are the fields that are
  trusted *downstream in the engine*, past the SEH wrapper:
  - **R3** (`startU16 > startU8`, negative `count16`) — AV on a bad **read**.
  - **R4** (giant `sizeMemBuffer`) — not merely an allocation DoS: a short read
    followed by an out-of-bounds **write** while the buffer is filled.
  - **R5** (giant `num_sequences`) — the most serious: the bad count drives
    execution to a near-null address (control-flow-hijack potential), not just a
    DoS.

Takeaway: the header **offset** trust (R1/R2) is caught by SEH in the client,
but the **size / count** fields trusted deeper in the engine (R3/R4/R5) are not,
and one of them (R5) corrupts control flow. Exact fault addresses / RVAs stay in
the lab write-up in the VM (`wine-fixes/4.04r2-pr111-fuzz.md`); per `AGENTS.md`
no binary offsets or addresses go on a branch.
