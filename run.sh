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

read_dotenv_value() {
  local name="$1"
  local env_file="$ROOT_DIR/.env"
  local line key value

  [[ -f "$env_file" ]] || return 0
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line#"${line%%[![:space:]]*}"}"
    [[ -z "$line" || "$line" == \#* || "$line" != *=* ]] && continue

    key="${line%%=*}"
    key="${key%"${key##*[![:space:]]}"}"
    if [[ "$key" == "$name" ]]; then
      value="${line#*=}"
      value="${value#"${value%%[![:space:]]*}"}"
      value="${value%"${value##*[![:space:]]}"}"
      value="${value%\"}"
      value="${value#\"}"
      value="${value%\'}"
      value="${value#\'}"
      printf '%s\n' "$value"
      return 0
    fi
  done < "$env_file"
}

need_cmd cargo
need_cmd node
need_cmd npm
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
if [[ ! -d "$ROOT_DIR/frontend/node_modules" ]]; then
  npm --prefix frontend install
fi
npm --prefix frontend run build
cargo build
dotenv_bind_addr="$(read_dotenv_value FUDOKI_BIND_ADDR)"
bind_addr="${FUDOKI_BIND_ADDR:-${dotenv_bind_addr:-127.0.0.1:8000}}"
echo
echo "Fudoki is starting. Open http://$bind_addr in your browser."
echo "For LAN sharing: FUDOKI_BIND_ADDR=0.0.0.0:8000 ./run.sh"
echo "Then open http://<this-computer-lan-ip>:8000 from another machine."
echo
cargo run
