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

function Get-DotenvValue {
    param([string]$Name)

    $EnvPath = Join-Path $RootDir ".env"
    if (-not (Test-Path $EnvPath)) {
        return $null
    }

    $Pattern = "^\s*" + [regex]::Escape($Name) + "\s*=\s*(.*)\s*$"
    foreach ($Line in Get-Content $EnvPath) {
        if ($Line -match "^\s*#") {
            continue
        }
        if ($Line -match $Pattern) {
            return $Matches[1].Trim().Trim('"').Trim("'")
        }
    }

    return $null
}

Require-Command cargo
Require-Command node
Require-Command npm
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
if (-not (Test-Path (Join-Path $RootDir "frontend/node_modules"))) {
    & npm --prefix frontend install
    if ($LASTEXITCODE -ne 0) {
        throw "frontend npm install failed."
    }
}
& npm --prefix frontend run build
if ($LASTEXITCODE -ne 0) {
    throw "frontend build failed."
}
cargo build
$BindAddr = if ($env:FUDOKI_BIND_ADDR) { $env:FUDOKI_BIND_ADDR } else { Get-DotenvValue "FUDOKI_BIND_ADDR" }
if (-not $BindAddr) {
    $BindAddr = "127.0.0.1:8000"
}
Write-Host ""
Write-Host "Fudoki is starting. Open http://$BindAddr in your browser."
Write-Host "For LAN sharing in PowerShell: `$env:FUDOKI_BIND_ADDR=`"0.0.0.0:8000`"; .\run.ps1"
Write-Host "For LAN sharing in Command Prompt: set FUDOKI_BIND_ADDR=0.0.0.0:8000 && run.bat"
Write-Host "Then open http://<this-computer-lan-ip>:8000 from another machine."
Write-Host ""
cargo run
