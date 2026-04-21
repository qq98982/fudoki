# Fudoki Launcher Scripts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cross-platform launcher scripts that automatically download the latest official Sudachi core dictionary when missing, build the Rust backend, and start Fudoki.

**Architecture:** Keep launcher logic outside the Rust application itself. Add one real implementation for Unix-like systems (`run.sh`), one real implementation for Windows PowerShell (`run.ps1`), and one thin Windows wrapper (`run.bat`). Validate launcher behavior with small Node-based contract tests that inspect script contents and keep README aligned with the new startup flow.

**Tech Stack:** Bash, PowerShell, Windows batch, Node test runner, existing Rust/Cargo runtime, GitHub Releases API

---

## Planned File Structure

**Create**

- `run.sh`
- `run.ps1`
- `run.bat`
- `tests/launcher/run-sh.test.mjs`
- `tests/launcher/run-ps1.test.mjs`
- `tests/launcher/run-bat.test.mjs`

**Modify**

- `README.md`

## Task 1: Add Unix Launcher and Contract Test

**Files:**
- Create: `run.sh`
- Test: `tests/launcher/run-sh.test.mjs`

- [ ] **Step 1: Write the failing contract test**

```javascript
// tests/launcher/run-sh.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.sh downloads latest Sudachi core dictionary and starts Rust app", () => {
  const source = readFileSync("run.sh", "utf8");

  assert.match(source, /resources\/system\.dic/);
  assert.match(source, /https:\/\/api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/);
  assert.match(source, /core\.zip/);
  assert.match(source, /cargo build/);
  assert.match(source, /cargo run/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/launcher/run-sh.test.mjs`

Expected: FAIL with `ENOENT` because `run.sh` does not exist yet

- [ ] **Step 3: Write the Unix launcher**

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RESOURCES_DIR="$ROOT_DIR/resources"
DICT_PATH="$RESOURCES_DIR/system.dic"
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/launcher/run-sh.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add run.sh tests/launcher/run-sh.test.mjs
git commit -m "feat: add Unix launcher script"
```

## Task 2: Add PowerShell Launcher and Batch Wrapper

**Files:**
- Create: `run.ps1`
- Create: `run.bat`
- Test: `tests/launcher/run-ps1.test.mjs`
- Test: `tests/launcher/run-bat.test.mjs`

- [ ] **Step 1: Write the failing Windows launcher tests**

```javascript
// tests/launcher/run-ps1.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.ps1 downloads latest Sudachi core dictionary and starts Rust app", () => {
  const source = readFileSync("run.ps1", "utf8");

  assert.match(source, /resources[\\/]+system\.dic/);
  assert.match(source, /api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/);
  assert.match(source, /Invoke-RestMethod/);
  assert.match(source, /Expand-Archive/);
  assert.match(source, /cargo build/);
  assert.match(source, /cargo run/);
});
```

```javascript
// tests/launcher/run-bat.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.bat delegates to run.ps1", () => {
  const source = readFileSync("run.bat", "utf8");

  assert.match(source, /powershell/i);
  assert.match(source, /ExecutionPolicy Bypass/i);
  assert.match(source, /run\.ps1/i);
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `node --test tests/launcher/run-ps1.test.mjs tests/launcher/run-bat.test.mjs`

Expected: FAIL with `ENOENT` because `run.ps1` and `run.bat` do not exist yet

- [ ] **Step 3: Write the PowerShell launcher**

```powershell
# run.ps1
$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ResourcesDir = Join-Path $RootDir "resources"
$DictPath = Join-Path $ResourcesDir "system.dic"
$LatestReleaseApi = "https://api.github.com/repos/WorksApplications/SudachiDict/releases/latest"

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required command: $Name"
    }
}

function Install-LatestSudachiDictionary {
    Write-Host "Downloading latest Sudachi core dictionary..."
    $release = Invoke-RestMethod -Uri $LatestReleaseApi
    $asset = $release.assets | Where-Object { $_.name -match "-core\.zip$" } | Select-Object -First 1
    if (-not $asset) {
        throw "Could not find Sudachi core zip asset in latest release."
    }

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("fudoki-sudachi-" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    $ZipPath = Join-Path $TempDir "sudachi-core.zip"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    $ExtractedDic = Get-ChildItem -Path $TempDir -Recurse -Filter "system*.dic" | Select-Object -First 1
    if (-not $ExtractedDic) {
        throw "Could not find extracted system dictionary."
    }

    New-Item -ItemType Directory -Path $ResourcesDir -Force | Out-Null
    Copy-Item -Path $ExtractedDic.FullName -Destination $DictPath -Force
}

Require-Command cargo
Require-Command node

New-Item -ItemType Directory -Path $ResourcesDir -Force | Out-Null

if (-not (Test-Path $DictPath)) {
    Install-LatestSudachiDictionary
}

Set-Location $RootDir
cargo build
cargo run
```

```bat
@echo off
set SCRIPT_DIR=%~dp0
powershell -ExecutionPolicy Bypass -File "%SCRIPT_DIR%run.ps1"
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `node --test tests/launcher/run-ps1.test.mjs tests/launcher/run-bat.test.mjs`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add run.ps1 run.bat tests/launcher/run-ps1.test.mjs tests/launcher/run-bat.test.mjs
git commit -m "feat: add Windows launcher scripts"
```

## Task 3: Update README and Verify Launcher Entry Points

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the failing documentation check**

Run:

```bash
rg -n "./run.sh|\\.\\\\run.ps1|run.bat|latest Sudachi core" README.md
```

Expected: no matches

- [ ] **Step 2: Update README launcher instructions**

```md
# README.md

## Local Development

Preferred launchers:

### Linux/macOS

```bash
./run.sh
```

### Windows PowerShell

```powershell
.\run.ps1
```

### Windows Command Prompt

```bat
run.bat
```

Launcher behavior:

- checks required tools
- downloads the latest official Sudachi core dictionary if `resources/system.dic` is missing
- runs `cargo build`
- starts the server with `cargo run`
```

- [ ] **Step 3: Verify launcher docs and scripts**

Run:

```bash
node --test tests/launcher/run-sh.test.mjs tests/launcher/run-ps1.test.mjs tests/launcher/run-bat.test.mjs
bash -n run.sh
rg -n "./run.sh|\\.\\\\run.ps1|run.bat|latest official Sudachi core dictionary" README.md
```

Expected:

- Node launcher tests PASS
- `bash -n run.sh` exits 0
- README contains the new launcher commands and download behavior

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add launcher script usage"
```

## Task 4: Manual Smoke Verification on Current Machine

**Files:**
- Modify: none

- [ ] **Step 1: Verify existing dictionary path skips download**

Run:

```bash
test -f resources/system.dic
```

Expected:

- exit 0 on current machine

- [ ] **Step 2: Run Unix launcher on current machine**

Run:

```bash
chmod +x run.sh
timeout 10 ./run.sh
```

Expected:

- launcher does not attempt dictionary download if `resources/system.dic` already exists
- Rust build succeeds
- server starts and is terminated by timeout with exit code `124`

- [ ] **Step 3: Record Windows smoke commands for later manual execution**

Run:

```text
.\run.ps1
run.bat
```

Expected:

- both commands should download dictionary if missing
- both commands should build and start the Rust server

- [ ] **Step 4: Commit**

```bash
git status --short
```

Expected:

- no new tracked file changes

## Spec Coverage Check

- Unix launcher: Task 1
- Windows PowerShell launcher: Task 2
- Windows batch wrapper: Task 2
- latest Sudachi core auto-download when missing: Task 1, Task 2
- automatic build then run: Task 1, Task 2
- README launcher docs: Task 3
- local smoke verification: Task 4

## Placeholder Scan

- No `TBD`
- No `TODO`
- No “implement later”
- Every task contains exact files, commands, and concrete code

## Type Consistency Check

- All launchers use the same installed dictionary target path: `resources/system.dic`
- All launchers use the same upstream source: GitHub latest SudachiDict release API
- All launchers build with `cargo build` and start with `cargo run`
