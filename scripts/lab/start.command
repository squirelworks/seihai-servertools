#!/bin/sh
# Seihai lab world for macOS: double-click this file in Finder to start the
# lab listserver and gservers in Terminal. It runs start.sh beside it.
# Part of seihai-servertools (MIT OR Apache-2.0).
exec "$(dirname "$0")/start.sh" "$@"
