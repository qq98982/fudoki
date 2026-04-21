#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RESOURCES_DIR="$ROOT_DIR/resources"
DICT_PATH="$RESOURCES_DIR/system.dic"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

ensure_git_lfs() {
  if ! git lfs version >/dev/null 2>&1; then
    echo "Missing required command: git lfs" >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd node
need_cmd git
ensure_git_lfs
mkdir -p "$RESOURCES_DIR"

if [[ ! -f "$DICT_PATH" ]]; then
  echo "Running git lfs pull to fetch required assets..."
  cd "$ROOT_DIR"
  git lfs pull
fi

if [[ ! -f "$DICT_PATH" ]]; then
  echo "Missing required file: resources/system.dic" >&2
  echo "Ensure git-lfs is installed and 'git lfs pull' completed successfully." >&2
  exit 1
fi

cd "$ROOT_DIR"
cargo build
cargo run
