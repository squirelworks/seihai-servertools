#!/bin/sh
# Turn the two key words of a packet 140 or 197 header into the HEX16 that
# `seihai-servertools code show|dump|pack --key` takes (issue #37): each
# word as a little-endian 32-bit value, first word first.
# Part of seihai-servertools (MIT OR Apache-2.0).
#
#   code-key.sh WORD1 WORD2      decimal values, as decoded from the GINT5s
#
# Example: code-key.sh 1 2  prints  0100000002000000
[ $# -eq 2 ] || { echo "usage: code-key.sh WORD1 WORD2 (decimal)" >&2; exit 2; }
for w in "$1" "$2"; do
	case "$w" in ''|*[!0-9]*) echo "code-key.sh: $w is not a decimal number" >&2; exit 2 ;; esac
	[ "$w" -le 4294967295 ] || { echo "code-key.sh: $w does not fit 32 bits" >&2; exit 2; }
	printf '%02x%02x%02x%02x' $((w & 255)) $((w >> 8 & 255)) $((w >> 16 & 255)) $((w >> 24 & 255))
done
echo
