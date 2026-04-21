#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RESOURCES_DIR="$ROOT_DIR/resources"
DICT_PATH="$ROOT_DIR/resources/system.dic"
LATEST_RELEASE_API="https://api.github.com/repos/WorksApplications/SudachiDict/releases/latest"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

fetch_to_stdout() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
  else
    echo "Missing required command: curl or wget" >&2
    exit 1
  fi
}

download_to_file() {
  local url="$1"
  local output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$output" "$url"
  else
    echo "Missing required command: curl or wget" >&2
    exit 1
  fi
}

extract_zip() {
  local archive="$1"
  local destination="$2"
  if command -v unzip >/dev/null 2>&1; then
    unzip -oq "$archive" -d "$destination"
  elif command -v bsdtar >/dev/null 2>&1; then
    bsdtar -xf "$archive" -C "$destination"
  elif command -v tar >/dev/null 2>&1; then
    tar -xf "$archive" -C "$destination"
  else
    echo "Missing required command: unzip, bsdtar, or tar" >&2
    exit 1
  fi
}

download_latest_dictionary() {
  local release_json archive_url tmp_dir archive_path found_dict
  release_json="$(fetch_to_stdout "$LATEST_RELEASE_API")"
  archive_url="$(
    printf '%s' "$release_json" | node -e '
      let input = "";
      process.stdin.on("data", (chunk) => { input += chunk; });
      process.stdin.on("end", () => {
        const release = JSON.parse(input);
        const asset = (release.assets || []).find((item) => /-core\.zip$/.test(item.name));
        if (!asset) {
          console.error("Could not find Sudachi core zip asset in latest release.");
          process.exit(1);
        }
        process.stdout.write(asset.browser_download_url);
      });
    '
  )"

  tmp_dir="$(mktemp -d)"
  archive_path="$tmp_dir/sudachi-core.zip"

  echo "Downloading latest Sudachi core dictionary..."
  download_to_file "$archive_url" "$archive_path"
  extract_zip "$archive_path" "$tmp_dir"

  found_dict="$(find "$tmp_dir" -type f -name 'system*.dic' | head -n 1)"
  if [[ -z "$found_dict" ]]; then
    echo "Could not find extracted system dictionary." >&2
    exit 1
  fi

  mkdir -p "$RESOURCES_DIR"
  cp "$found_dict" "$DICT_PATH"
}

need_cmd cargo
need_cmd node
mkdir -p "$RESOURCES_DIR"

if [[ ! -f "$DICT_PATH" ]]; then
  download_latest_dictionary
fi

cd "$ROOT_DIR"
cargo build
cargo run
