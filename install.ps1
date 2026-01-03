# Tarqeem Installation Script for Windows
# ترقيم - أول لغة برمجة عربية مُترجَمة
#
# Usage:
#   .\install.ps1                          # Install to %LocalAppData%\Tarqeem
#   $env:TARQEEM_HOME="C:\Custom" ; .\install.ps1  # Install to custom location

$ErrorActionPreference = "Stop"

# Installation directory
if ($env:TARQEEM_HOME) {
    $InstallDir = $env:TARQEEM_HOME
} else {
    $InstallDir = "$env:LOCALAPPDATA\Tarqeem"
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Blue
Write-Host "║           ترقيم - Tarqeem Compiler Installation           ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Blue
Write-Host ""

# Check if we're in the right directory
if (-not (Test-Path "$ScriptDir\Cargo.toml")) {
    Write-Host "Error: Must run from tarqeem source directory" -ForegroundColor Red
    exit 1
}

# Check if release binary exists
if (-not (Test-Path "$ScriptDir\target\release\tarqeem.exe")) {
    Write-Host "Release binary not found. Building..." -ForegroundColor Yellow
    Write-Host ""

    # Build compiler
    Write-Host "Building Tarqeem compiler (release mode)..." -ForegroundColor Blue
    Push-Location $ScriptDir
    cargo build --release
    Pop-Location
    Write-Host ""
}

# Verify runtime library was built
if (-not (Test-Path "$ScriptDir\target\release\libtrq.a")) {
    Write-Host "Error: Runtime library (libtrq.a) not found. Build may have failed." -ForegroundColor Red
    exit 1
}

Write-Host "Installing to: $InstallDir" -ForegroundColor Blue
Write-Host ""

# Create installation directory structure
New-Item -ItemType Directory -Force -Path "$InstallDir\bin" | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallDir\lib" | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallDir\stdlib_trq" | Out-Null

# Copy binary
Write-Host "  ✓ Installing tarqeem.exe" -ForegroundColor Green
Copy-Item "$ScriptDir\target\release\tarqeem.exe" "$InstallDir\bin\" -Force

# Copy runtime library (Rust implementation)
Write-Host "  ✓ Installing runtime library" -ForegroundColor Green
Copy-Item "$ScriptDir\target\release\libtrq.a" "$InstallDir\lib\" -Force

# Copy standard library
Write-Host "  ✓ Installing standard library" -ForegroundColor Green
Copy-Item "$ScriptDir\stdlib_trq\*" "$InstallDir\stdlib_trq\" -Recurse -Force

# Get version from Cargo.toml
$CargoContent = Get-Content "$ScriptDir\Cargo.toml" -Raw
if ($CargoContent -match 'version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
    $Version | Out-File "$InstallDir\VERSION" -NoNewline
    Write-Host "  ✓ Version: $Version" -ForegroundColor Green
}

# Set environment variables
Write-Host ""
Write-Host "Setting environment variables..." -ForegroundColor Blue

[Environment]::SetEnvironmentVariable("TARQEEM_HOME", $InstallDir, "User")
Write-Host "  ✓ TARQEEM_HOME = $InstallDir" -ForegroundColor Green

$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
$BinPath = "$InstallDir\bin"

if ($CurrentPath -notlike "*$BinPath*") {
    [Environment]::SetEnvironmentVariable("Path", "$BinPath;$CurrentPath", "User")
    Write-Host "  ✓ Added $BinPath to PATH" -ForegroundColor Green
} else {
    Write-Host "  ✓ PATH already contains $BinPath" -ForegroundColor Green
}

Write-Host ""
Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║               Installation Complete! ✓                     ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "Please restart your terminal for the changes to take effect." -ForegroundColor Yellow
Write-Host ""
Write-Host "Verify installation with:" -ForegroundColor Blue
Write-Host ""
Write-Host "  tarqeem --version" -ForegroundColor Cyan
Write-Host ""
