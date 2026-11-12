#!/bin/sh
# Expected-vs-actual check of a lab world's consoles after `lab start` or
# `setup start` (issue #47). Reads only; changes nothing. Loopback only.
# Part of seihai-servertools (MIT OR Apache-2.0).
#
#   check-consoles.sh WORLD                    default, classic, modern
#   check-consoles.sh WORLD default classic    only those servers
#   check-consoles.sh --addr 127.0.0.2 WORLD default
#                      also require "Listening on: 127.0.0.2:<port>"
#   check-consoles.sh --no-listserver WORLD default
#                      skip the listserver console (setup start alone)
#
# Ports follow the lab world: default 14802, classic 14803, modern 14805,
# listserver 127.0.0.1:14900. Other servers need SERVER=PORT.
# Prints one row per check and exits 1 when any row is FAIL.

ADDR=
LIST=yes
while [ $# -gt 0 ]; do
	case "$1" in
	--addr) ADDR=$2; shift 2 ;;
	--no-listserver) LIST=no; shift ;;
	-h|--help) sed -n '2,16p' "$0"; exit 0 ;;
	*) break ;;
	esac
done
[ $# -ge 1 ] || { echo "usage: check-consoles.sh [--addr ADDR] [--no-listserver] WORLD [SERVER[=PORT]...]" >&2; exit 2; }
WORLD=$1; shift
[ $# -gt 0 ] || set -- default classic modern

FAILS=0
row() { # result, what, seen
	printf '%-4s  %-44s  %s\n' "$1" "$2" "$3"
	[ "$1" = FAIL ] && FAILS=$((FAILS + 1))
	return 0
}

if [ "$LIST" = yes ]; then
	L="$WORLD/logs/listserver-console.txt"
	if [ ! -f "$L" ]; then
		row FAIL "listserver console" "no $L"
	elif grep -q '127\.0\.0\.1:14900' "$L"; then
		row PASS "listserver on 127.0.0.1:14900" "$(grep -m1 '127\.0\.0\.1:14900' "$L")"
	else
		row FAIL "listserver on 127.0.0.1:14900" "$(head -n1 "$L")"
	fi
fi

for s in "$@"; do
	name=${s%%=*}
	case "$s" in
	*=*) port=${s#*=} ;;
	default) port=14802 ;;
	classic) port=14803 ;;
	modern) port=14805 ;;
	*) row FAIL "$name: port" "give it as $name=PORT"; continue ;;
	esac
	C="$WORLD/servers/$name/logs/console.txt"
	if [ ! -f "$C" ]; then
		row FAIL "$name: console" "no $C"
		continue
	fi
	seen=$(grep -m1 "Listening on: .*:$port\([^0-9]\|$\)" "$C")
	if [ -z "$seen" ]; then
		row FAIL "$name: listening on port $port" "$(grep -m1 'Listening on: ' "$C")"
	elif [ -n "$ADDR" ] && ! printf '%s' "$seen" | grep -q "Listening on: $ADDR:$port"; then
		row FAIL "$name: listening on $ADDR:$port" "$seen"
	else
		row PASS "$name: listening on ${ADDR:-*}:$port" "$seen"
	fi
	seen=$(grep -m1 'listserver - Connected to ' "$C")
	if printf '%s' "$seen" | grep -q 'Connected to 127\.0\.0\.1:14900'; then
		row PASS "$name: linked to 127.0.0.1:14900" "$seen"
	else
		row FAIL "$name: linked to 127.0.0.1:14900" "${seen:-no Connected line}"
	fi
	bad=$(grep -m1 -e 'Could not .*socket' -e 'listserver - \[Error\]' -e 'listserver - Disconnected' "$C")
	if [ -n "$bad" ]; then
		row FAIL "$name: no socket or listserver errors" "$bad"
	else
		row PASS "$name: no socket or listserver errors" ""
	fi
done

if [ "$FAILS" -gt 0 ]; then
	echo "$FAILS check(s) failed"
	exit 1
fi
echo "all checks passed"
