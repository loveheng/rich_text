#!/usr/bin/env bash
# 构建 rich_text 原生核心的 Android arm64-v8a 产物，并拷入 goodshare 的 jniLibs。
# 用法：
#   ./build-android.sh                 # 默认拷到 ../goodshare/android/app/src/main/jniLibs
#   OUT_DIR=/path/to/jniLibs ./build-android.sh
# 依赖：cargo-ndk（cargo install cargo-ndk）+ Android NDK（ANDROID_NDK_HOME 或 ANDROID_HOME/ndk/*）
set -euo pipefail

cd "$(dirname "$0")/.."   # native/rich_text_uniffi

export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/$(ls "$ANDROID_HOME/ndk" 2>/dev/null | head -1)}"
[ -d "$ANDROID_NDK_HOME" ] || { echo "找不到 NDK，请设置 ANDROID_NDK_HOME"; exit 1; }

source "$HOME/.cargo/env" 2>/dev/null || true

CARGO=${CARGO:-cargo}
TARGETS="arm64-v8a"   # 当前仅支持 arm64-v8a（aarch64-linux-android）
ARGS=()
for t in $TARGETS; do ARGS+=(-t "$t"); done

$CARGO ndk -P 21 "${ARGS[@]}" -- build --release -p rich_text_uniffi

DST="${OUT_DIR:-/home/zzh/app/goodshare/android/app/src/main/jniLibs}"
declare -A M=( [aarch64-linux-android]=arm64-v8a )
SRC=target
for t in "${!M[@]}"; do
  so="$SRC/$t/release/librich_text_uniffi.so"
  [ -f "$so" ] && mkdir -p "$DST/${M[$t]}" && cp "$so" "$DST/${M[$t]}/" && echo "-> ${M[$t]}"
done
echo "已写入 $DST"
