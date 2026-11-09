#!/usr/bin/env bash
# 再生 UniFFI 各端绑定桩（Kotlin / Swift）。
#
# 前置：
#   cargo build 已完成（产出 target/debug/librich_text_uniffi.so）
#   uniffi-bindgen 在 PATH 中：  cargo install uniffi_bindgen --version 0.27.3
#
# 用法：  ./scripts/gen-bindings.sh
set -euo pipefail

cd "$(dirname "$0")/.."

LIB="target/debug/librich_text_uniffi.so"
if [ ! -f "$LIB" ]; then
  echo "缺少 $LIB，先执行 cargo build" >&2
  exit 1
fi

BIN="${UNIFFI_BINDGEN:-uniffi-bindgen}"

echo ">> Kotlin"
"$BIN" generate --library "$LIB" --language kotlin --out-dir bindings/kotlin

echo ">> Swift"
"$BIN" generate --library "$LIB" --language swift --out-dir bindings/swift

echo ">> Dart：核心 uniffi-bindgen 不产出 Dart，需用社区 uniffi-dart-bindgen（dart:ffi + ffigen）。"
echo "   见 native/README.md §Dart 绑定 一节。"
echo "done."
