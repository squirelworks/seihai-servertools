#!/usr/bin/env python3
"""Format-aware mutator/fuzzer for Torque (TGE) .dts shape files.

A .dts v19-v24 file begins:
  u32 version      (low byte = version, high 16 = exporter version)
  u32 sizeMemBuffer (dwords)   u32 startU16 (dword offset)  u32 startU8 (dword offset)
  <sizeMemBuffer*4 bytes of packed 32/16/8-bit section data>
  u32 numSequences ; per-sequence records ; material list

The loader does `new int32_t[sizeMemBuffer]` then derives interior pointers
mem16 = buf+startU16, mem8 = buf+startU8 with NO validation, and the section
reader walks counts out of that buffer with no bounds checks beyond periodic
guard words. This mutator targets exactly those fields plus generic flips and
truncation, so every output is a concrete file that can also be served to the
real client in the lab.
"""
import struct, os, sys, random

HDR = struct.Struct('<IIII')  # version, size, start16, start8

def parse_header(d):
    if len(d) < 16: return None
    ver, size, s16, s8 = HDR.unpack_from(d, 0)
    return ver, size, s16, s8

def set_header(d, ver, size, s16, s8):
    d = bytearray(d)
    HDR.pack_into(d, 0, ver & 0xffffffff, size & 0xffffffff, s16 & 0xffffffff, s8 & 0xffffffff)
    return bytes(d)

INTERESTING32 = [0,1,2,3,0xffffffff,0x7fffffff,0x80000000,0x40000000,
                 0xfffffffe,0x10000000,0x00010000,0xffff,0x7fff,0x8000,
                 0xdeadbeef,0xcafebabe,0x20000000,0x0fffffff,100000000,0x55555555]

def mut_header(d, rng):
    h = parse_header(d)
    if not h: return d
    ver, size, s16, s8 = h
    pick = rng.randrange(8)
    if pick == 0:   size = rng.choice(INTERESTING32)                 # huge/zero alloc
    elif pick == 1: s16  = rng.choice(INTERESTING32)                 # OOB interior ptr
    elif pick == 2: s8   = rng.choice(INTERESTING32)
    elif pick == 3: s16, s8 = s8, s16                                # swap -> negative count16
    elif pick == 4: s8  = max(0, s16 - rng.randrange(1,8))           # count16 negative
    elif pick == 5: size = max(0, s8 - rng.randrange(1,16))          # count8 negative
    elif pick == 6: ver = (ver & 0xffff0000) | rng.choice([18,19,22,23,24,25,26,255,0])
    else:           size = size + rng.choice([-1,-2,1,2,1<<20])
    return set_header(d, ver, size, s16, s8)

def mut_counts(d, rng):
    """Corrupt a dword inside the packed 32-bit section (counts: numNodes,
    numObjects, numMeshes, numNames, numDetails, ...)."""
    h = parse_header(d)
    if not h: return mut_flip(d, rng)
    size = h[1]
    body = 16
    maxoff = min(len(d)-4, body + max(0,size)*4)
    if maxoff <= body: return mut_flip(d, rng)
    off = body + 4*rng.randrange(max(1,(maxoff-body)//4))
    d = bytearray(d); struct.pack_into('<I', d, off, rng.choice(INTERESTING32))
    return bytes(d)

def mut_numseq(d, rng):
    """Blow up the sequence count (read just after the packed buffer)."""
    h = parse_header(d)
    if not h: return d
    size = h[1]
    off = 16 + max(0,size)*4
    if off+4 > len(d): return mut_header(d, rng)
    d = bytearray(d); struct.pack_into('<I', d, off, rng.choice([0xffffffff,0x7fffffff,100000,0x40000000,0x10000000]))
    return bytes(d)

def mut_flip(d, rng):
    if not d: return d
    d = bytearray(d)
    for _ in range(rng.randrange(1,8)):
        d[rng.randrange(len(d))] ^= (1 << rng.randrange(8))
    return bytes(d)

def mut_trunc(d, rng):
    if len(d) <= 16: return d
    return d[:rng.randrange(16, len(d))]

def mut_extend(d, rng):
    return d + bytes(rng.getrandbits(8) for _ in range(rng.randrange(4, 64)))

MUTATORS = [mut_header, mut_header, mut_header, mut_counts, mut_counts,
            mut_numseq, mut_flip, mut_trunc, mut_extend]

def mutate(d, rng, n=1):
    for _ in range(n):
        d = rng.choice(MUTATORS)(d, rng)
    return d

def main():
    seeds_dir, out_dir, count = sys.argv[1], sys.argv[2], int(sys.argv[3])
    seed = int(sys.argv[4]) if len(sys.argv) > 4 else 1337
    rng = random.Random(seed)
    seeds = []
    for fn in sorted(os.listdir(seeds_dir)):
        if fn.lower().endswith('.dts'):
            with open(os.path.join(seeds_dir, fn), 'rb') as f:
                seeds.append(f.read())
    os.makedirs(out_dir, exist_ok=True)
    for i in range(count):
        base = rng.choice(seeds)
        out = mutate(base, rng, n=rng.randrange(1,4))
        with open(os.path.join(out_dir, f'm{i:06d}.dts'), 'wb') as f:
            f.write(out)
    print(f'wrote {count} mutants from {len(seeds)} seeds -> {out_dir}')

if __name__ == '__main__':
    main()
