#!/usr/bin/env bash
# Build the Torque (.dts) shape-loader fuzz harness.
#
# We do not ship a .dts parser of our own (Graal3D / TGE 3D mode is not
# modelled in seihai-core), so this harness fuzzes a TGE-lineage loader as a
# stand-in for the real client's TSShape::read. We use SimonBoorer's MIT
# `libdts`, which caps at shape version 24 -- the same ceiling the 4.04r2
# Graal3D client enforces.
#
# Nothing here is committed from libdts or from Torque3D: this script fetches
# both at pinned revisions. No lab captures, client binaries or shape assets
# live on the branch.
#
# Requires: git, clang++ (for the MSVC-flavoured headers), g++ (for the ASan
# runtime), curl, python3.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${1:-$HERE/.work}"
LIBDTS_COMMIT=ebe9efc586a285c33442c4ab7d4893831d123d98   # SimonBoorer/libdts, MIT
mkdir -p "$WORK"
cd "$WORK"

if [ ! -d libdts ]; then
  git clone https://github.com/SimonBoorer/libdts
fi
git -C libdts checkout -q "$LIBDTS_COMMIT"
L="$WORK/libdts/libdts"

# libdts' DTSEndian.h has ambiguous EndianSwap overloads that neither clang nor
# gcc will accept. The whole byte-swap path is dead on a little-endian host, so
# replace the header with a clean templated version. (Local build tree only.)
cat > "$L/DTSEndian.h" <<'EOF'
#ifndef DTS_ENDIAN_H_
#define DTS_ENDIAN_H_
#include <cstdint>
#include <cstring>
namespace DTS {
static inline bool IsLittleEndian() { const std::uint32_t i = 0x1; return reinterpret_cast<const char&>(i) == 1; }
template <typename T> inline T EndianSwap(const T in) {
    union { T v; unsigned char b[sizeof(T)]; } s{in}, d;
    for (size_t k = 0; k < sizeof(T); ++k) d.b[k] = s.b[sizeof(T)-1-k];
    return d.v;
}
template <typename T> inline T ConvertHostToLEndian(T i) { return IsLittleEndian() ? i : EndianSwap(i); }
template <typename T> inline T ConvertLEndianToHost(T i) { return IsLittleEndian() ? i : EndianSwap(i); }
}
#endif
EOF

# Translation units on the read path (the mesh-fit / convex-decomp code is not
# needed to load a shape and pulls in extra deps, so it is left out).
SRC="DTSShape DTSShapeOldRead DTSShapeAlloc DTSShapeEdit DTSMesh DTSSortedMesh \
     DTSMaterialList DTSIntegerSet DTSDecal DTSMatrix DTSPoint DTSQuat"
INCS="-include cstdint -include cstring -include cstdlib -include string \
      -include cmath -include math.h -include cfloat"

mkdir -p obj
for s in $SRC; do
  clang++ -std=c++11 -g -O1 -fsanitize=address -fno-omit-frame-pointer -DSTANDALONE \
    -fms-extensions -fdelayed-template-parsing -w $INCS -I"$L" \
    -c "$L/$s.cpp" -o "obj/$s.o"
done
clang++ -std=c++11 -g -O1 -fsanitize=address -fno-omit-frame-pointer -DSTANDALONE \
  -fms-extensions -fdelayed-template-parsing -w $INCS -I"$L" \
  -c "$HERE/fuzz_dts.cpp" -o obj/fuzz_dts.o
# Link with g++ so the GCC ASan runtime is used (clang's compiler-rt is often
# not installed); the clang-emitted instrumentation is ABI-compatible with it.
g++ -fsanitize=address obj/*.o -o "$HERE/fuzz_dts_asan"
echo "built: $HERE/fuzz_dts_asan"

# Fetch a few MIT Torque3D v3.10.1 template shapes as fuzz seeds (version 24).
SEEDS="$HERE/.seeds"
mkdir -p "$SEEDS"
BASE=https://raw.githubusercontent.com/TorqueGameEngines/Torque3D/v3.10.1/Templates/Full/game
for p in core/art/shapes/octahedron.dts core/art/shapes/simplecone.dts \
         art/shapes/weapons/shared/rocket.dts art/shapes/rocks/boulder.dts; do
  curl -fsSL "$BASE/$p" -o "$SEEDS/$(basename "$p")"
done
echo "seeds in: $SEEDS"
