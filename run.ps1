$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ResourcesDir = Join-Path $RootDir "resources"
$DictRelativePath = "resources/system.dic"
$DictPath = Join-Path $RootDir $DictRelativePath
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

    try {
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
    finally {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
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
