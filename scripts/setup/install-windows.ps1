##############################################################################
# Armbian Imager - Windows Dependencies Installation Script
#
# This script installs all system dependencies required for development
# on Windows
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\setup\install-windows.ps1
# Requires: Administrator privileges
##############################################################################

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

# Color functions
function Print-Info {
    param([string]$Message)
    Write-Host "ℹ" -NoNewline -ForegroundColor Blue
    Write-Host " $Message" -ForegroundColor Cyan
}

function Print-Success {
    param([string]$Message)
    Write-Host "✓" -NoNewline -ForegroundColor Green
    Write-Host " $Message" -ForegroundColor Green
}

function Print-Warning {
    param([string]$Message)
    Write-Host "⚠" -NoNewline -ForegroundColor Yellow
    Write-Host " $Message" -ForegroundColor Yellow
}

function Print-Error {
    param([string]$Message)
    Write-Host "✗" -NoNewline -ForegroundColor Red
    Write-Host " $Message" -ForegroundColor Red
}

Print-Info "Armbian Imager - Windows Dependencies Installation"
Write-Host ""

# Check if running on Windows
if ($PSVersionTable.PSVersion.Major -lt 5) {
    Print-Error "PowerShell 5.0 or higher is required"
    exit 1
}

$OSInfo = [System.Environment]::OSVersion.VersionString
Print-Info "Operating System: $OSInfo"

# Check if Chocolatey is installed
Print-Info "Checking Chocolatey installation..."
if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
    Print-Warning "Chocolatey is not installed"
    Print-Info "Installing Chocolatey..."

    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

    # Refresh environment variables
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

    Print-Success "Chocolatey installed"
} else {
    $ChocoVersion = choco --version
    Print-Success "Chocolatey is already installed: $ChocoVersion"
}

# Update Chocolatey
Print-Info "Updating Chocolatey..."
choco upgrade chocolatey -y

# Install Visual Studio Build Tools
Print-Info "Checking Visual Studio Build Tools..."
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsInstall = & $vsWhere -latest -products * -requires Microsoft.Component.MSBuild -property installationPath
    if ($vsInstall) {
        Print-Success "Visual Studio Build Tools are installed"
    } else {
        Print-Warning "Visual Studio Build Tools not found"
        Print-Info "Installing Visual Studio Build Tools..."

        choco install visualstudio2022buildtools -y --params "--add Microsoft.VisualStudio.Workload.VCTools;includeRecommended"

        Print-Success "Visual Studio Build Tools installed"
    }
} else {
    Print-Warning "Visual Studio Build Tools not found"
    Print-Info "Installing Visual Studio Build Tools..."

    choco install visualstudio2022buildtools -y --params "--add Microsoft.VisualStudio.Workload.VCTools;includeRecommended"

    Print-Success "Visual Studio Build Tools installed"
}

# Install WebView2
Print-Info "Checking WebView2 Runtime..."
if (-not (Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue)) {
    Print-Warning "WebView2 Runtime not found"
    Print-Info "Installing WebView2 Runtime..."

    choco install microsoft-webview2-runtime -y

    Print-Success "WebView2 Runtime installed"
} else {
    Print-Success "WebView2 Runtime is installed"
}

# Check if Node.js is installed
Print-Info "Checking Node.js installation..."
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Print-Warning "Node.js is not installed"
    Print-Info "Installing Node.js via Chocolatey..."

    choco install nodejs-lts -y

    # Refresh environment
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

    Print-Success "Node.js installed: $(node --version)"
} else {
    $NodeVersion = node --version
    Print-Success "Node.js is already installed: $NodeVersion"
}

# Check if Rust is installed
Print-Info "Checking Rust/Cargo installation..."
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Print-Warning "Rust/Cargo is not installed"
    Print-Info "Installing Rust via rustup..."

    # Download and run rustup-init
    $rustupInit = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
    & $rustupInit -y

    # Remove installer
    Remove-Item $rustupInit -Force

    # Add Cargo to PATH
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    $currentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$cargoPath*") {
        [System.Environment]::SetEnvironmentVariable("Path", "$currentPath;$cargoPath", "User")
    }

    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

    Print-Success "Rust installed: $(rustc --version)"
} else {
    $RustVersion = rustc --version
    Print-Success "Rust is already installed: $RustVersion"
}

# Install OpenSSL for Windows (required for some npm packages)
Print-Info "Checking OpenSSL..."
if (-not (Get-Command openssl -ErrorAction SilentlyContinue)) {
    Print-Info "Installing OpenSSL..."

    choco install openssl -y

    Print-Success "OpenSSL installed"
} else {
    Print-Success "OpenSSL is already installed"
}

Write-Host ""
Print-Success "Installation completed!"
Write-Host ""
Print-Info "Next steps:"
Write-Host "  1. Restart your terminal to refresh environment variables" -ForegroundColor Yellow
Write-Host "  2. Install npm dependencies: npm install" -ForegroundColor White
Write-Host "  3. Start development server: npm run tauri:dev" -ForegroundColor White
Write-Host ""
Print-Warning "You may need to restart your computer for all changes to take effect"
Write-Host ""
