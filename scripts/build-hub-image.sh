#!/usr/bin/env bash
# 本地构建 Hub Docker 镜像
# 用法: ./scripts/build-hub-image.sh [tag]
set -euo pipefail

TAG="${1:-rex-hub:local}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"

echo "=== 清理 dist/ ==="
rm -rf "$DIST"
mkdir -p "$DIST/static"

echo "=== 构建前端 ==="
cd "$ROOT/packages/rex-console-web"
bun run build
cp -r dist/* "$DIST/static/"
cd "$ROOT"

echo "=== 构建 rex-hub ==="
cargo build --release -p rex-hub
cp target/release/rex-hub "$DIST/"

echo "=== 构建 Docker 镜像: $TAG ==="
docker build -t "$TAG" -f Dockerfile.hub "$ROOT"

echo "=== 完成 ==="
echo "运行: docker run -p 3000:3000 $TAG"
