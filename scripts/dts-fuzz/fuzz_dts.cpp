// ASan driver for the Torque (TGE-lineage) .dts shape loader.
// Target: DTS::TSShape::LoadFromStream  (libdts caps at version 24 — the
// same ceiling the 4.04r2 Graal3D client's TSShape::read enforces).
//
// Two build modes:
//   libFuzzer:   defines LLVMFuzzerTestOneInput (coverage-guided, if available)
//   standalone:  -DSTANDALONE builds a main() that runs each file argument
//                through the loader under ASan; prints the crashing file.
#include <cstdint>
#include <cstddef>
#include <sstream>
#include <string>
#include "DTSShape.h"

static void run_one(const uint8_t *data, size_t size) {
    std::string buf(reinterpret_cast<const char*>(data), size);
    std::istringstream is(buf, std::ios::binary);
    DTS::TSShape shape;
    shape.LoadFromStream(is);
}

extern "C" int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
    run_one(data, size);
    return 0;
}

#ifdef STANDALONE
#include <cstdio>
#include <vector>
int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        FILE *f = fopen(argv[i], "rb");
        if (!f) { fprintf(stderr, "open fail %s\n", argv[i]); continue; }
        fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
        std::vector<uint8_t> b(n > 0 ? n : 0);
        if (n > 0 && fread(b.data(), 1, n, f) != (size_t)n) { fclose(f); continue; }
        fclose(f);
        fprintf(stderr, "[run] %s (%ld bytes)\n", argv[i], n);
        run_one(b.data(), b.size());
    }
    return 0;
}
#endif
