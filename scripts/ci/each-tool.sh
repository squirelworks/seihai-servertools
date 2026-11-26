#!/bin/sh
# Builds and tests every tool under tools/ on its own: one `cargo build -p`
# and one `cargo test -p` per tool, so a tool that only builds because a
# sibling enabled a dependency or feature for it fails here
# (tools/README.md, "Standalone"). Run from the repository root.
# Usage: scripts/ci/each-tool.sh [build|test|all]   (default: all)
set -eu
what="${1:-all}"
for manifest in tools/*/Cargo.toml; do
    pkg=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$manifest" | head -n 1)
    dir=$(dirname "$manifest")
    for f in README.md src/main.rs src/lib.rs; do
        if [ ! -f "$dir/$f" ]; then
            echo "$dir: missing $f (every tool has its own README, binary and lib)" >&2
            exit 1
        fi
    done
    # A tool depends on the shared crates and seihai-core, never on another
    # tool: shared code goes into crates/lat-core or crates/setup-core.
    for other in tools/*/Cargo.toml; do
        [ "$other" = "$manifest" ] && continue
        opkg=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$other" | head -n 1)
        if grep -Eq "^$opkg( |\.|=)" "$manifest"; then
            echo "$dir: depends on the tool $opkg; move the shared code to a core crate" >&2
            exit 1
        fi
    done
    case "$what" in
        build | all) echo "== build $pkg"; cargo build --locked -p "$pkg" ;;
    esac
    case "$what" in
        test | all) echo "== test $pkg"; cargo test --locked -p "$pkg" ;;
    esac
done
