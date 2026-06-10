# AegisRescue - WSL Quick Build Launcher
# Usage: .\build\wsl-quick.ps1 [options]
#
# Options:
#   -Arch     x86_64|aarch64    (default: x86_64)
#   -Version  0.1.0             (default: 0.1.0)
#   -Jobs     N                 (default: auto)
#   -SkipRust                   skip Rust build
#   -SkipGui                    skip GUI build
#   -SkipBase                   skip debootstrap (reuse existing chroot)
#   -IsoOnly                    regenerate squashfs+ISO only (fastest)
#   -QemuTest                   run QEMU test after build
#   -Setup                      run setup (install packages) first

param(
    [string]$Arch    = "x86_64",
    [string]$Version = "0.1.0",
    [int]   $Jobs    = 0,
    [switch]$SkipRust,
    [switch]$SkipGui,
    [switch]$SkipBase,
    [switch]$IsoOnly,
    [switch]$QemuTest,
    [switch]$Setup
)

$ErrorActionPreference = "Continue"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DriveLetter = ($ProjectRoot -split ':')[0].ToLower()
$UnixPath    = "/mnt/$DriveLetter" + ($ProjectRoot -replace '^[A-Za-z]:', '' -replace '\\', '/')
$BuildScript = "$UnixPath/build/wsl-build.sh"

# Detect WSL distro
$rawList  = wsl --list --quiet 2>&1
$distros  = $rawList | ForEach-Object { ($_ -replace '\x00','').Trim() } | Where-Object { $_ -ne '' }
$distro   = ($distros | Where-Object { $_ -match "Ubuntu" } | Select-Object -First 1)
if (-not $distro) { $distro = $distros | Select-Object -First 1 }
if (-not $distro) {
    Write-Host "[FAIL] No WSL distro found. Run .\build\wsl-setup.ps1 first." -ForegroundColor Red
    exit 1
}
$distro = $distro.Trim()

# Build argument list
$argParts = @("--arch", $Arch, "--ver", $Version)
if ($Jobs -gt 0)  { $argParts += @("--jobs", "$Jobs") }
if ($SkipRust)    { $argParts += "--skip-rust" }
if ($SkipGui)     { $argParts += "--skip-gui" }
if ($SkipBase)    { $argParts += "--skip-base" }
if ($IsoOnly)     { $argParts += "--iso-only" }
if ($QemuTest)    { $argParts += "--qemu-test" }
if ($Setup)       { $argParts = @("--setup") }

$argStr = $argParts -join " "
$isoPath = "$ProjectRoot\build\out\aegisrescue-$Version-$Arch.iso"

Write-Host ""
Write-Host "  AegisRescue WSL Build" -ForegroundColor Blue
Write-Host "  Distro : $distro" -ForegroundColor DarkGray
Write-Host "  Script : $BuildScript" -ForegroundColor DarkGray
Write-Host "  Args   : $argStr" -ForegroundColor DarkGray
Write-Host "  Output : $isoPath" -ForegroundColor DarkGray
Write-Host ""

# Run
$startTime = Get-Date
wsl -d $distro -u root -- bash $BuildScript $argParts
$elapsed = [Math]::Round(((Get-Date) - $startTime).TotalMinutes, 1)

# Check output
Write-Host ""
if (Test-Path $isoPath) {
    $sizeMB = [Math]::Round((Get-Item $isoPath).Length / 1MB, 0)
    Write-Host "  Build complete in ${elapsed} min!" -ForegroundColor Green
    Write-Host "  ISO  : $isoPath" -ForegroundColor Green
    Write-Host "  Size : ${sizeMB} MB" -ForegroundColor Green

    $sha256File = "$isoPath.sha256"
    if (Test-Path $sha256File) {
        $hash = (Get-Content $sha256File -Raw).Split(' ')[0].Trim()
        Write-Host "  SHA256: $hash" -ForegroundColor DarkGray
    }

    Write-Host ""
    Write-Host "  Next steps:" -ForegroundColor White
    Write-Host "  - Flash to USB : use Rufus or balenaEtcher" -ForegroundColor DarkGray
    Write-Host "  - QEMU test    : wsl -- qemu-system-x86_64 -m 2G -cdrom '$UnixPath/build/out/aegisrescue-$Version-$Arch.iso' -boot d -vga std" -ForegroundColor DarkGray
    Write-Host ""
    
    $openExplorer = Read-Host "Open output folder in Explorer? (y/n)"
    if ($openExplorer -eq 'y' -or $openExplorer -eq 'Y') {
        explorer.exe "/select,$isoPath"
    }
} else {
    Write-Host "  [WARN] ISO not found at expected path." -ForegroundColor Yellow
    Write-Host "  Check WSL build output above for errors." -ForegroundColor Yellow
}
