#!/usr/bin/env bash
# 本地开发: 下载当前平台的 pdfium 动态库到 vendor/pdfium/lib/
# CI 中每平台同样逻辑（见 .github/workflows/ci.yml）
set -euo pipefail

VERSION="8057"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/vendor/pdfium"
mkdir -p "$OUT"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) ASSET="pdfium-mac-arm64.tgz" ;;
  Darwin-x86_64) ASSET="pdfium-mac-x64.tgz" ;;
  Linux-x86_64) ASSET="pdfium-linux-x64.tgz" ;;
  Linux-aarch64) ASSET="pdfium-linux-arm64.tgz" ;;
  MINGW*|MSYS*|Windows_NT) ASSET="pdfium-win-x64.zip" ;;
  *) echo "不支持的平台: $(uname -s)-$(uname -m)"; exit 1 ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/$VERSION/$ASSET"
echo "下载 $URL"
curl -sL "$URL" -o "$OUT/$ASSET"

case "$ASSET" in
  *.tgz) tar -xzf "$OUT/$ASSET" -C "$OUT" ;;
  *.zip) unzip -oq "$OUT/$ASSET" -d "$OUT" ;;
esac
rm -f "$OUT/$ASSET"

# 把动态库直接放到二进制旁边（发现链第二优先级），跑应用无需 export 环境变量
LIB="$(ls "$OUT/lib/" | grep -E 'pdfium' | head -1)"
mkdir -p "$ROOT/target/debug" "$ROOT/target/release"
cp "$OUT/lib/$LIB" "$ROOT/target/debug/"
cp "$OUT/lib/$LIB" "$ROOT/target/release/"

echo "pdfium 已就绪: $OUT/lib/"
ls "$OUT/lib/"
echo "已复制到 target/debug 与 target/release (应用自动发现, 无需环境变量)"
