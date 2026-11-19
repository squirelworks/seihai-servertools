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

| file | corruption | debug-build result | release-build expectation |
| --- | --- | --- | --- |
| `R1_startU8_wild.dts` | `startU8 = 0x40000000` | SEGV (wild read) | SEGV |
| `R2_startU8_oob.dts` | `startU8 = size+4` | heap-buffer-overflow (read) | OOB read |
| `R3_start16_gt_start8.dts` | `startU16 > startU8` | `check16` assert | negative-length `memcpy` |
| `R4_size_huge.dts` | `sizeMemBuffer = 0x3fffffff` | `check32` assert after a 4 GB alloc | 4 GB alloc + short read |
| `R5_numseq_huge.dts` | `num_sequences = 0x3fffffff` | allocation-size-too-big | multi-GB `resize` DoS |

These are the payloads to serve to the real client in the lab.

## Lab follow-up (real 4.04r2 client)

Code-only results above are complete. To confirm against the real loader, the
five reproducers are served to 4.04r2 through our own lab Graal3D server in a
Mirae/GwenTea2 VM (never a public server), the same way the GR V1.05 packet
fuzzing and the #105 Sky/Player work drive the client. Watch `console.log` for
`Couldn't load shape` versus an actual crash / hang, and whether the link drops.
Results (which reproducers crash the real client, and where) are recorded with
the lab run, not on this branch.
