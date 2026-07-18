#!/usr/bin/env bash

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

if ! command -v npm >/dev/null 2>&1; then
  echo "错误：未找到 npm，请先安装 Node.js。" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "错误：未找到 cargo，请先安装 Rust。" >&2
  exit 1
fi

if [[ ! -d node_modules ]]; then
  echo "首次构建，正在安装前端依赖……"
  npm ci
fi

echo "正在构建 idevice 桌面应用……"
BUNDLES="${BUNDLES:-app}"
npm run desktop:build -- --bundles "$BUNDLES" "$@"

echo
echo "构建完成。应用位于："
echo "  $PROJECT_DIR/src-tauri/target/release/bundle/"
