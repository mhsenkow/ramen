#!/usr/bin/env bash
# Build the Rust core and install it for Godot.
#
# The codesign step is NOT optional on Apple Silicon: overwriting a dylib in
# place invalidates its ad-hoc signature and the kernel then SIGKILLs Godot at
# load with no output at all. Remove, copy, re-sign.
set -e
cd "$(dirname "$0")/sim"
cargo build --release --lib "$@"
rm -f ../game/bin/librama_sim.dylib
cp target/release/librama_sim.dylib ../game/bin/
codesign --force --sign - ../game/bin/librama_sim.dylib
echo "rama_sim installed and signed"
