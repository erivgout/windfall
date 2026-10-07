#!/usr/bin/env bash
# Rebuilds the WebAssembly document that the UI's mock backend runs, and
# records what it was built from. Run it after any change to windfall-sim,
# windfall-project or windfall-core, and commit both files it writes:
#
#   scripts/build-sim.sh
#
# Needs the target once: rustup target add wasm32-unknown-unknown
# `node scripts/check-sim.mjs` tells whether the checked-in module is stale.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$root/apps/desktop/src/lib/ipc/sim/windfall_sim.wasm"

native() {
  if command -v cygpath >/dev/null; then cygpath -w "$1"; else printf '%s' "$1"; fi
}

# The procedural macros are built for this machine, which on Windows needs
# the Visual Studio environment.
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) source "$root/scripts/msvc-env.sh" ;;
esac

# The module is downloaded by every browser session and loaded by every test
# file, so it is built for size. The profile is set here because Cargo
# ignores profiles outside the workspace root, and the root's release
# profile is the app's.
export CARGO_PROFILE_RELEASE_OPT_LEVEL=s
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_PANIC=abort
export CARGO_PROFILE_RELEASE_DEBUG=false
export CARGO_PROFILE_RELEASE_STRIP=true

# Panic messages name source files. Without this they would carry the home
# folder of whoever built the module.
cargo_home="$(native "${CARGO_HOME:-$HOME/.cargo}")"
separator=$'\x1f'
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$(native "$root")=.${separator}--remap-path-prefix=${cargo_home}=cargo"

# A target folder of its own, so this never waits for, or throws away, a
# build of the app.
export CARGO_TARGET_DIR="$root/target/sim"

cd "$root"
cargo build --release --package windfall-sim --lib --target wasm32-unknown-unknown
built="$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/windfall_sim.wasm"

if command -v wasm-opt >/dev/null; then
  wasm-opt -Os --strip-debug --strip-producers "$built" -o "$out"
else
  cp "$built" "$out"
fi
node "$root/scripts/check-sim.mjs" --write
