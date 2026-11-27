#!/bin/sh
# Build the community 3D test worlds, each as its own tarball with a README,
# a licence note, the build's commits and checksums.
#
#   scripts/community-worlds/build.sh OUT [--graal-analysis DIR] [--date YYYY-MM-DD]
#
# OUT gets <name>-<date>.tar.gz, <name>-<date>.tar.gz.sha256 for each world,
# and SHA256SUMS over all of them. The tarballs are reproducible: file order,
# times, owners and gzip headers are fixed, so the same commits give the
# same checksums.
#
# Needs cargo (builds the servertools it uses), python3 (3.11+), GNU tar,
# gzip and sha256sum. --graal-analysis is a Graal_analysis checkout, for the
# conformance cases and the scoping grid's content (default: a sibling
# ../Graal_analysis or ../graal_analysis). Worlds that need it are skipped
# without it.
set -eu

usage() { sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
[ $# -ge 1 ] || usage
OUT=$1; shift
GA= DATE=$(date -u +%Y-%m-%d)
while [ $# -gt 0 ]; do
  case $1 in
    --graal-analysis) GA=$2; shift 2;;
    --date) DATE=$2; shift 2;;
    *) usage;;
  esac
done

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
if [ -z "$GA" ]; then
  for d in "$REPO/../Graal_analysis" "$REPO/../graal_analysis"; do
    [ -d "$d/tools/conformance" ] && GA=$(cd "$d" && pwd) && break
  done
fi
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
WORK=$OUT/.work; rm -rf "$WORK"; mkdir -p "$WORK"
PY="python3 -I $HERE/worlds.py"

echo "building servertools"
(cd "$REPO" && cargo build --release --locked -q -p seihai-gen -p seihai-regions -p seihai-pw3d)
BIN=$REPO/target/release

commit() { git -C "$1" rev-parse --short=12 HEAD 2>/dev/null || echo unknown; }
ST_COMMIT=$(commit "$REPO")
GA_COMMIT=$( [ -n "$GA" ] && commit "$GA" || echo none )
# Fixed time for every file in a tarball: midnight UTC at the start of the package date.
STAMP="$DATE 00:00:00Z"

# finish NAME TITLE: README, LICENCE.md, BUILD-INFO.txt, SHA256SUMS, then the tarball.
finish() {
  name=$1 dir=$WORK/$1
  { cat "$HERE/readme/$name.md"; cat "$HERE/readme/_common.md"; } > "$dir/README.md"
  cp "$HERE/LICENCE-WORLDS.md" "$dir/LICENCE.md"
  printf 'package %s-%s\nseihai-servertools %s\nGraal_analysis %s\nbuilt by scripts/community-worlds/build.sh\n' \
    "$name" "$DATE" "$ST_COMMIT" "$GA_COMMIT" > "$dir/BUILD-INFO.txt"
  (cd "$dir" && find . -type f ! -name SHA256SUMS | LC_ALL=C sort | sed 's|^\./||' \
     | while IFS= read -r f; do sha256sum "$f"; done > SHA256SUMS)
  tarball=$name-$DATE.tar.gz
  (cd "$WORK" && LC_ALL=C tar --sort=name --mtime="$STAMP" --owner=0 --group=0 --numeric-owner \
     --mode='u+rwX,go+rX,go-w' --format=gnu -cf - "$name") | gzip -9 -n > "$OUT/$tarball"
  (cd "$OUT" && sha256sum "$tarball" > "$tarball.sha256")
  echo "  $tarball ($(du -h "$OUT/$tarball" | cut -f1))"
}

echo "isle terrain"
"$BIN/seihai-gen" terrain "$WORK/seihai3d-isle-terrain/seihai-gserver" --world isle --size 4x4 \
  --3d --stream --playerworld > /dev/null
"$BIN/seihai-gen" terrain "$WORK/seihai3d-isle-terrain/gserver-v2" --world isle --size 4x4 \
  --3d --stream --playerworld --v2 > /dev/null
finish seihai3d-isle-terrain

echo "regions overworld"
D=$WORK/seihai3d-regions-overworld
"$BIN/seihai-regions" demo "$D" > /dev/null
$PY server-txt "$D" --name "3 Seihai Regions" --what "regions demo overworld" \
  "# Required: 18,055 objects, far past a 5.12's 4,096 ghosts." \
  "graal3d_scope=on" "graal3d_pushterrain=true"
(cd "$D" && "$BIN/seihai-regions" stats config/graal3dworld.txt --root . > STATS.txt)
finish seihai3d-regions-overworld

echo "interiors sample"
D=$WORK/seihai3d-interiors-sample
$PY sample-playerworld "$D/input"
# Relative paths: the report names its input folder.
(cd "$D" && "$BIN/seihai-pw3d" input world --start sample_start.nw > /dev/null)
$PY server-txt "$D/world" --name "3 Seihai Interiors" --what "interiors sample" \
  "# Required: the house is sent only when a player nears its door." "graal3d_scope=on"
finish seihai3d-interiors-sample

if [ -n "$GA" ]; then
  echo "scoping grid (Graal_analysis $GA_COMMIT)"
  $PY grid "$WORK/seihai3d-scoping-grid" --graal-analysis "$GA" > /dev/null
  finish seihai3d-scoping-grid
  echo "conformance worlds (Graal_analysis $GA_COMMIT)"
  $PY conformance "$WORK/seihai3d-conformance-worlds" --graal-analysis "$GA"
  finish seihai3d-conformance-worlds
else
  echo "no Graal_analysis checkout: skipping the scoping grid and conformance worlds" >&2
fi

rm -rf "$WORK"
(cd "$OUT" && ls *.tar.gz | LC_ALL=C sort | xargs sha256sum > SHA256SUMS)
echo "done: $OUT"
