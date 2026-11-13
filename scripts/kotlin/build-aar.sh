#!/usr/bin/env bash
# Copyright (c) 2026 Mintlayer Institutional FZCO
# Contact: hello@mintlayer.org
#
# Use of this source code is governed by an MIT license
# that can be found in the LICENSE file.

# Builds the Android AAR for the wallet bindings:
#   1. cross-compiles the cdylib for the three required ABIs (cargo-ndk)
#   2. generates the Kotlin bindings from the compiled library
#   3. assembles the AAR with Gradle (JNA dependency + jniLibs)
#
# Prerequisites: Rust 1.92+, cargo-ndk, JDK 17, Android SDK, Android NDK
# (ANDROID_NDK_HOME), Gradle 8.7+. Outputs the AAR path on stdout.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
KT_DIR="$REPO_ROOT/bindings/kotlin"
ABI_TARGETS=(aarch64-linux-android armv7-linux-androideabi x86_64-linux-android)

command -v cargo >/dev/null || { echo "cargo not found" >&2; exit 1; }
command -v rustup >/dev/null || { echo "rustup not found" >&2; exit 1; }
if [ -z "${ANDROID_HOME:-}" ]; then
  echo "ANDROID_HOME is not set (Android SDK required for Gradle)" >&2
  exit 1
fi
command -v cargo-ndk >/dev/null || { echo "cargo-ndk not found (cargo install cargo-ndk)" >&2; exit 1; }
command -v gradle >/dev/null || { echo "gradle not found" >&2; exit 1; }
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  echo "ANDROID_NDK_HOME is not set" >&2
  exit 1
fi

cd "$REPO_ROOT"

rm -rf "$KT_DIR/src/main/kotlin" "$KT_DIR/src/main/jniLibs"

echo "[1/4] cross-compiling cdylib for: ${ABI_TARGETS[*]}" >&2
for target in "${ABI_TARGETS[@]}"; do
  rustup target add "$target"
done
cargo build --release --lib -p mintlayer-wallet-bindings
TARGET_ARGS=()
for target in "${ABI_TARGETS[@]}"; do
  TARGET_ARGS+=(-t "$target")
done
cargo ndk --platform 24 \
  "${TARGET_ARGS[@]}" \
  -o "$KT_DIR/src/main/jniLibs" \
  build --release --lib -p mintlayer-wallet-bindings

echo "[2/4] generating Kotlin bindings" >&2
case "$(uname -s)" in
  Darwin) HOST_LIB="target/release/libmintlayer_wallet_bindings.dylib" ;;
  *) HOST_LIB="target/release/libmintlayer_wallet_bindings.so" ;;
esac
cargo run --quiet -p mintlayer-wallet-bindings --bin uniffi-bindgen -- \
  generate \
  --library "$HOST_LIB" \
  --language kotlin \
  --out-dir "$KT_DIR/src/main/kotlin" \
  --no-format

echo "[3/4] assembling AAR" >&2
( cd "$KT_DIR" && gradle assembleRelease )

echo "[4/4] done" >&2
ls "$KT_DIR"/build/outputs/aar/*-release.aar
