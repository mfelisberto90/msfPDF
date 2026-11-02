#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_HOME_DIR="${RUSTUP_HOME:-$HOME/.rustup}"

if [[ -d "${CARGO_HOME_DIR}/bin" ]]; then
  export PATH="${CARGO_HOME_DIR}/bin:${PATH}"
fi

EXTRA_FLAGS=(
  "-C" "force-unwind-tables=no"
  "-C" "link-arg=-Wl,-z,pack-relative-relocs"
  "-C" "link-arg=-Wl,--as-needed"
  "--remap-path-prefix=${ROOT_DIR}=/src"
  "--remap-path-prefix=${CARGO_HOME_DIR}=/cargo"
  "--remap-path-prefix=${RUSTUP_HOME_DIR}=/rustup"
  "--remap-path-prefix=${HOME}=/build"
)

export RUSTFLAGS="${RUSTFLAGS:-} ${EXTRA_FLAGS[*]}"
cd "${ROOT_DIR}"
exec cargo build --release "$@"
