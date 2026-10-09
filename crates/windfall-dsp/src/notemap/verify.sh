#!/usr/bin/env bash
# Run from the workspace root with the MSVC environment already sourced.
set -euo pipefail
cargo build -p windfall-dsp --lib
deps="${CARGO_TARGET_DIR:-target}/debug/deps"
latest_rlib() {
  local -a candidates
  mapfile -t candidates < <(ls -t "$deps"/lib"$1"-*.rlib)
  printf '%s' "${candidates[0]}"
}
rustc --edition=2024 --test crates/windfall-dsp/src/notemap/verify.rs \
  -L "dependency=$deps" \
  --extern "windfall_dsp=$(latest_rlib windfall_dsp)" \
  --extern "serde=$(latest_rlib serde)" \
  --extern "serde_json=$(latest_rlib serde_json)" \
  --extern "ts_rs=$(latest_rlib ts_rs)" \
  -o "${CARGO_TARGET_DIR:-target}/notemap-verify.exe"
"${CARGO_TARGET_DIR:-target}/notemap-verify.exe" notemap
