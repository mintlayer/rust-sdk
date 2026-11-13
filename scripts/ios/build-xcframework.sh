#!/usr/bin/env bash
# Copyright (c) 2026 Mintlayer Institutional FZCO
# Contact: hello@mintlayer.org
#
# Use of this source code is governed by an MIT license
# that can be found in the LICENSE file.

# Builds the iOS XCFramework for the wallet bindings:
#   1. cross-compiles the static library for the two required slices
#      (iOS device arm64, Apple Silicon simulator)
#   2. generates the Swift bindings and the C header/modulemap
#   3. assembles MintlayerWalletBindings.xcframework and zips it
#
# Prerequisites: Rust 1.92+, macOS with Xcode. Outputs the zip path and its
# SHA-256 on stdout; artifacts land in dist/.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"
# The Intel-simulator slice (x86_64-apple-ios-sim) is omitted: Rust 1.92.0
# no longer publishes its rust-std for macOS hosts, matching Apple's removal
# of Intel simulator runtimes from recent Xcode. Intel-Mac developers cannot
# run recent simulators regardless.
SLICES=(aarch64-apple-ios aarch64-apple-ios-sim)

command -v cargo >/dev/null || { echo "cargo not found" >&2; exit 1; }
command -v rustup >/dev/null || { echo "rustup not found" >&2; exit 1; }
command -v xcodebuild >/dev/null || { echo "xcodebuild not found (macOS with Xcode required)" >&2; exit 1; }
[ "$(uname -s)" = "Darwin" ] || { echo "this script requires macOS with Xcode" >&2; exit 1; }

cd "$REPO_ROOT"

echo "[1/4] building static libraries for: ${SLICES[*]}" >&2
for target in "${SLICES[@]}"; do
  rustup target add "$target"
  cargo build --release --lib -p mintlayer-wallet-bindings --target "$target"
done
# The host dylib is only an input for the binding generator below.
cargo build --release --lib -p mintlayer-wallet-bindings

echo "[2/4] generating Swift bindings and the C header/modulemap" >&2
SWIFT_DIR="$DIST_DIR/swift-sources"
rm -rf "$SWIFT_DIR"
HOST_LIB="target/release/libmintlayer_wallet_bindings.dylib"
[ -f "$HOST_LIB" ] || { echo "host dylib missing: $HOST_LIB" >&2; exit 1; }
cargo run --quiet --release -p mintlayer-wallet-bindings --bin uniffi-bindgen -- \
  generate \
  --library "$HOST_LIB" \
  --language swift \
  --out-dir "$SWIFT_DIR" \
  --no-format

echo "[3/4] assembling the XCFramework" >&2
XC_DIR="$DIST_DIR/MintlayerWalletBindings.xcframework"
rm -rf "$XC_DIR"
XCFW_ARGS=()
for target in "${SLICES[@]}"; do
  lib="target/${target}/release/libmintlayer_wallet_bindings.a"
  [ -f "$lib" ] || { echo "missing static library: $lib" >&2; exit 1; }
  header_dir="target/${target}/release/headers"
  mkdir -p "$header_dir"
  cp "$SWIFT_DIR/MintlayerWalletBindingsFFI.h" "$header_dir/"
  cat > "$header_dir/module.modulemap" << EOF
module MintlayerWalletBindingsFFI {
    header "MintlayerWalletBindingsFFI.h"
    export *
    use "Darwin"
    use "_Builtin_stdbool"
    use "_Builtin_stdint"
}
EOF
  XCFW_ARGS+=(-library "$lib" -headers "$header_dir")
done
xcodebuild -create-xcframework "${XCFW_ARGS[@]}" -output "$XC_DIR"

echo "[4/4] zipping and hashing" >&2
cd "$DIST_DIR"
rm -f MintlayerWalletBindings.xcframework.zip SHA256SUMS
zip -q -r MintlayerWalletBindings.xcframework.zip MintlayerWalletBindings.xcframework
shasum -a 256 MintlayerWalletBindings.xcframework.zip > SHA256SUMS
echo "zip: $DIST_DIR/MintlayerWalletBindings.xcframework.zip" >&2
cat SHA256SUMS >&2
