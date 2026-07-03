# Build release and copy single portable exe to dist/
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "Building minelts (release)..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$src = Join-Path $PSScriptRoot "target\release\minelts.exe"
$dist = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item -Force $src (Join-Path $dist "minelts.exe")

$size = (Get-Item (Join-Path $dist "minelts.exe")).Length
$mb = [math]::Round($size / 1MB, 2)
Write-Host "Portable exe: dist\minelts.exe ($mb MB)" -ForegroundColor Green
