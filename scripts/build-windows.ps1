param(
  [ValidateSet("nsis", "msi", "all")]
  [string]$Bundle = "nsis",
  [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Require-Command {
  param([string]$Name, [string]$InstallHint)
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing '$Name'. $InstallHint"
  }
}

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

Write-Host "BetterFy Windows builder" -ForegroundColor Magenta
Write-Host "Project: $ProjectRoot"

Require-Command "node" "Install Node.js 22 LTS or newer: https://nodejs.org/"
Require-Command "npm" "Node.js must include npm."
Require-Command "cargo" "Install stable Rust with rustup: https://rustup.rs/"
Require-Command "rustc" "Install stable Rust with rustup: https://rustup.rs/"

$NodeMajor = [int]((node --version).TrimStart("v").Split(".")[0])
if ($NodeMajor -lt 20) {
  throw "Node.js 20+ is required. Installed: $(node --version)"
}

Write-Host "[1/4] Installing locked frontend dependencies" -ForegroundColor Cyan
npm ci

Write-Host "[2/4] Building and type-checking the interface" -ForegroundColor Cyan
npm run build

if (-not $SkipTests) {
  Write-Host "[3/4] Running Rust engine tests" -ForegroundColor Cyan
  cargo test --locked --manifest-path src-tauri/Cargo.toml
} else {
  Write-Host "[3/4] Rust tests skipped by request" -ForegroundColor Yellow
}

Write-Host "[4/4] Building BetterFy Windows bundle: $Bundle" -ForegroundColor Cyan
npm run tauri -- build --bundles $Bundle

$BundleRoot = Join-Path $ProjectRoot "src-tauri\target\release\bundle"
$Artifacts = @()
if (Test-Path $BundleRoot) {
  $Artifacts = Get-ChildItem $BundleRoot -Recurse -File |
    Where-Object { $_.Extension -in ".exe", ".msi" }
}

if ($Artifacts.Count -eq 0) {
  throw "Tauri completed but no Windows installer was found under $BundleRoot"
}

Write-Host "BetterFy build completed:" -ForegroundColor Green
$Artifacts | ForEach-Object { Write-Host "  $($_.FullName)" }
