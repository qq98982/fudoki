$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ResourcesDir = Join-Path $RootDir "resources"
$DictPath = Join-Path $ResourcesDir "system.dic"

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required command: $Name"
    }
}

function Require-GitLfs {
    & git lfs version *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "Missing required command: git lfs"
    }
}

Require-Command cargo
Require-Command node
Require-Command git
Require-GitLfs

New-Item -ItemType Directory -Path $ResourcesDir -Force | Out-Null

if (-not (Test-Path $DictPath)) {
    Write-Host "Running git lfs pull to fetch required assets..."
    Set-Location $RootDir
    & git lfs pull
    if ($LASTEXITCODE -ne 0) {
        throw "git lfs pull failed."
    }
}

if (-not (Test-Path $DictPath)) {
    throw "Missing required file: resources/system.dic. Ensure git-lfs is installed and 'git lfs pull' completed successfully."
}

Set-Location $RootDir
cargo build
cargo run
