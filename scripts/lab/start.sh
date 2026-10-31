#!/bin/sh
# Seihai lab world: start the listserver and the gservers on 127.0.0.1 with
# one command (Linux and macOS). Part of seihai-servertools (MIT OR Apache-2.0).
#
# Put this file in the lab world folder (next to gs2emu and servers/), with
# seihai-servertools and seihai-listserver beside it or on PATH.
#
#   ./start.sh            listserver, then servers/default and servers/classic
#   ./start.sh classic    listserver, then only servers/classic
#
# Stop with Ctrl+C. Consoles go to servers/<name>/logs/console.txt and
# logs/listserver-console.txt. All options: seihai-servertools --help

cd "$(dirname "$0")" || exit 1

if [ -x ./seihai-servertools ]; then
	TOOL=./seihai-servertools
elif command -v seihai-servertools >/dev/null 2>&1; then
	TOOL=seihai-servertools
else
	echo "start.sh: no seihai-servertools next to this script or on PATH. Unpack a seihai-servertools release here." >&2
	exit 1
fi

exec "$TOOL" lab start --world . "$@"
