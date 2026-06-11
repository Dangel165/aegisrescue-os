# AegisRescue - WSL2 Build Environment Setup
# Run from PowerShell (Admin recommended for WSL feature activation)
# Usage: .\build\wsl-setup.ps1

$ErrorActionPreference = "Continue"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DriveLetter = ($ProjectRoot -split ':')[0].ToLower()
$UnixPath    = "/mnt/$DriveLetter" + ($ProjectRoot -replace '^[A-Za-z]:', '' -replace '\\', '/')

function Write-Step { param($msg)
    Write-Host "`n=== $msg ===" -ForegroundColor Cyan
}
function Write-Ok { param($msg)
    Write-Host "  [OK] $msg" -ForegroundColor Green
}
function Write-Warn { param($msg)
    Write-Host "  [WARN] $msg" -ForegroundColor Yellow
}
function Write-Fail { param($msg)
    Write-Host "  [FAIL] $msg" -ForegroundColor Red
}

Write-Host ""
Write-Host "  AegisRescue WSL2 Build Setup" -ForegroundColor Blue
Write-Host "  Project : $ProjectRoot" -ForegroundColor DarkGray
Write-Host "  Unix    : $UnixPath" -ForegroundColor DarkGray
Write-Host ""

# -----------------------------------------------------------------------
# STEP 1: Check / Enable WSL2
# -----------------------------------------------------------------------
Write-Step "STEP 1/5 - Check WSL2"

$wslExe = Get-Command wsl -ErrorAction SilentlyContinue
if (-not $wslExe) {
    Write-Warn "WSL not found. Enabling features (requires reboot)..."
    if (([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]"Administrator")) {
        Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Windows-Subsystem-Linux -NoRestart -ErrorAction SilentlyContinue
        Enable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -NoRestart -ErrorAction SilentlyContinue
        Write-Warn "Reboot required. Re-run this script after reboot."
    } else {
        Write-Fail "Run PowerShell as Administrator to enable WSL features."
    }
    exit 1
}

try {
    $wslVer = wsl --status 2>&1 | Select-String "Default Version"
    Write-Ok "WSL found: $wslVer"
    wsl --set-default-version 2 2>&1 | Out-Null
    Write-Ok "WSL default version set to 2"
} catch {
    Write-Warn "Could not set WSL default version: $_"
}

# Update WSL kernel
Write-Host "  Updating WSL kernel..." -ForegroundColor DarkGray
wsl --update 2>&1 | Select-String "updated|up to date" | ForEach-Object { Write-Ok $_ }

# -----------------------------------------------------------------------
# STEP 2: Check / Install Ubuntu
# -----------------------------------------------------------------------
Write-Step "STEP 2/5 - Check Ubuntu Distribution"

$distros = wsl --list --quiet 2>&1 | Where-Object { $_ -match '\S' }
$ubuntuDistro = $distros | Where-Object { $_ -match "Ubuntu" } | Select-Object -First 1

if ($ubuntuDistro) {
    $ubuntuDistro = $ubuntuDistro.Trim() -replace '\x00', ''
    Write-Ok "Found: $ubuntuDistro"
} else {
    Write-Warn "Ubuntu not found. Installing Ubuntu 24.04..."
    $installed = $false
    
    # Try winget first
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if ($winget) {
        Write-Host "  Trying winget..." -ForegroundColor DarkGray
        winget install --id Canonical.Ubuntu.2404 --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
        $installed = $true
    }
    
    if (-not $installed) {
        Write-Host "  Trying wsl --install..." -ForegroundColor DarkGray
        wsl --install Ubuntu-24.04 2>&1 | Out-Null
    }
    
    Write-Warn "Ubuntu installed. You may need to:"
    Write-Warn "  1. Open Ubuntu from Start Menu and complete initial setup"
    Write-Warn "  2. Re-run this script"
    $ubuntuDistro = "Ubuntu-24.04"
}

# -----------------------------------------------------------------------
# STEP 3: Configure .wslconfig
# -----------------------------------------------------------------------
Write-Step "STEP 3/5 - Configure WSL2 Resources (.wslconfig)"

$wslConfigPath = "$env:USERPROFILE\.wslconfig"
$totalRAM  = [Math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB)
$wslRAM    = [Math]::Max(4, [Math]::Min(8, $totalRAM - 2))
$cpuCount  = (Get-CimInstance Win32_Processor).NumberOfLogicalProcessors
$wslCPU    = [Math]::Max(2, $cpuCount - 2)

$wslConfigContent = "[wsl2]`nmemory=${wslRAM}GB`nprocessors=${wslCPU}`nswap=2GB`nlocalhostForwarding=true`nnestedVirtualization=true`n`n[experimental]`nsparseVhd=true`n"

if (Test-Path $wslConfigPath) {
    Copy-Item $wslConfigPath "$wslConfigPath.bak" -Force
    Write-Ok "Backed up existing .wslconfig"
}
[System.IO.File]::WriteAllText($wslConfigPath, $wslConfigContent, [System.Text.Encoding]::ASCII)
Write-Ok "Created .wslconfig: RAM=${wslRAM}GB, CPU=${wslCPU} cores"

# Restart WSL to apply new config
Write-Host "  Restarting WSL..." -ForegroundColor DarkGray
wsl --shutdown
Start-Sleep -Seconds 2
Write-Ok "WSL restarted"

# -----------------------------------------------------------------------
# STEP 4: Install build packages inside WSL
# -----------------------------------------------------------------------
Write-Step "STEP 4/5 - Install Build Packages in WSL"

# Write bash setup script to a temp file in WSL
$bashScriptPath = "/tmp/aegis_wsl_setup.sh"

# Write the bash script content via WSL echo commands (avoids encoding issues)
$bashLines = @(
    "#!/bin/bash",
    "set -e",
    "echo '--- Updating apt ---'",
    "apt-get update -qq",
    "echo '--- Installing build tools ---'",
    "apt-get install -y --no-install-recommends \",
    "  debootstrap xorriso squashfs-tools \",
    "  grub-pc-bin grub-efi-amd64-bin grub-efi-amd64-signed \",
    "  grub2-common grub-common \",
    "  isolinux syslinux syslinux-common \",
    "  dosfstools mtools \",
    "  xz-utils zstd lz4 gzip \",
    "  rsync curl wget ca-certificates git \",
    "  build-essential pkg-config libssl-dev \",
    "  musl-tools musl-dev \",
    "  libwebkit2gtk-4.0-dev libgtk-3-dev \",
    "  libayatana-appindicator3-dev librsvg2-dev \",
    "  qemu-system-x86 ovmf \",
    "  python3-minimal \",
    "  2>/dev/null | tail -5",
    "echo '--- Installing Rust ---'",
    "if ! command -v cargo > /dev/null 2>&1; then",
    "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \",
    "    sh -s -- -y --default-toolchain stable --no-modify-path",
    "  echo 'source \$HOME/.cargo/env' >> /root/.bashrc",
    "fi",
    "export PATH=`"/root/.cargo/bin:\$PATH`"",
    "rustup target add x86_64-unknown-linux-musl 2>/dev/null || true",
    "echo '--- Installing Node.js ---'",
    "if ! command -v node > /dev/null 2>&1; then",
    "  curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -",
    "  apt-get install -y nodejs",
    "fi",
    "echo '--- Making scripts executable ---'",
    "chmod +x $UnixPath/build/build.sh 2>/dev/null || true",
    "chmod +x $UnixPath/build/wsl-build.sh 2>/dev/null || true",
    "echo ''",
    "echo '=== Setup complete! ==='",
    "cargo --version 2>/dev/null || echo 'Rust: not in PATH (run: source ~/.cargo/env)'",
    "node --version 2>/dev/null || echo 'Node: not found'",
    "echo 'Next: sudo bash $UnixPath/build/wsl-build.sh'"
)

# Join lines and write via wsl
$bashContent = $bashLines -join "`n"
$tempFile = [System.IO.Path]::GetTempFileName() + ".sh"
[System.IO.File]::WriteAllText($tempFile, $bashContent, [System.Text.Encoding]::UTF8)

# Convert Windows temp path to WSL path
$tempDrive = ($tempFile -split ':')[0].ToLower()
$tempUnix  = "/mnt/$tempDrive" + ($tempFile -replace '^[A-Za-z]:', '' -replace '\\', '/')

Write-Host "  Running setup script in WSL (this may take 5-10 minutes)..." -ForegroundColor DarkGray
wsl -d $ubuntuDistro -u root -- bash $tempUnix

Remove-Item $tempFile -Force -ErrorAction SilentlyContinue

# -----------------------------------------------------------------------
# STEP 5: Print next steps
# -----------------------------------------------------------------------
Write-Step "STEP 5/5 - Setup Complete"

Write-Host ""
Write-Host "  Setup finished! To build the ISO:" -ForegroundColor Green
Write-Host ""
Write-Host "  Option A - PowerShell (quick launcher):" -ForegroundColor White
Write-Host "    .\build\wsl-quick.ps1" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Option B - WSL terminal:" -ForegroundColor White
Write-Host "    wsl -u root -- bash $UnixPath/build/wsl-build.sh" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Option C - Open WSL and run:" -ForegroundColor White
Write-Host "    sudo bash $UnixPath/build/wsl-build.sh" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Output: $ProjectRoot\build\out\aegisrescue-beta-0.1.0-x86_64.iso" -ForegroundColor Cyan
Write-Host ""

$startNow = Read-Host "Start building now? (y/n)"
if ($startNow -eq 'y' -or $startNow -eq 'Y') {
    Write-Host "`nStarting build..." -ForegroundColor Cyan
    wsl -d $ubuntuDistro -u root -- bash "$UnixPath/build/wsl-build.sh"
}
