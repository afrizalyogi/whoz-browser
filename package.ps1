# Whoz Browser Packaging Script
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "       PACKAGING WHOZ BROWSER (PORTABLE + RELEASE)        " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Compile release binary
Write-Host "[1/4] Building release binary with Cargo..." -ForegroundColor Yellow
cargo build --release --bin whoz

$targetBin = "target\release\whoz.exe"
if (-not (Test-Path $targetBin)) {
    Write-Error "Binary $targetBin not found!"
    exit 1
}

# 2. Setup dist directory
$distDir = "dist\whoz-browser"
Write-Host "[2/4] Assembling release folder in $distDir..." -ForegroundColor Yellow
if (Test-Path $distDir) {
    Remove-Item -Path $distDir -Recurse -Force
}
New-Item -ItemType Directory -Path $distDir -Force | Out-Null
New-Item -ItemType Directory -Path "$distDir\data\user_data" -Force | Out-Null
New-Item -ItemType Directory -Path "$distDir\extensions" -Force | Out-Null

# 3. Copy binary & configuration
Copy-Item -Path $targetBin -Destination "$distDir\whoz.exe"
Copy-Item -Path $targetBin -Destination "$distDir\whoz-browser.exe"

$configContent = @'
{
  "max_renderer_processes": 4,
  "tab_suspend_timeout_secs": 60,
  "memory_pressure_threshold_mb": 800,
  "disable_telemetry": true,
  "use_native_internal_pages": true,
  "enable_google_sync": true,
  "user_data_dir": "./data/user_data",
  "chrome_base_dir": "C:\\Program Files\\Google\\Chrome\\Application"
}
'@
Set-Content -Path "$distDir\config.json" -Value $configContent

# Create VisualElementsManifest for Windows Start Menu tile
$manifestContent = @'
<Application xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance'>
  <VisualElements
      ShowNameOnSquare150x150Logo='on'
      ForegroundText='light'
      BackgroundColor='#1a73e8'/>
</Application>
'@
Set-Content -Path "$distDir\whoz.VisualElementsManifest.xml" -Value $manifestContent

# Create Quick Launcher
$launcherBatch = @'
@echo off
start "" "%~dp0whoz-browser.exe" %*
'@
Set-Content -Path "$distDir\launch.bat" -Value $launcherBatch

# 4. Generate Windows Installer Script (NSIS & PowerShell setup)
Write-Host "[3/4] Generating Windows Installer scripts..." -ForegroundColor Yellow
$installerScript = @'
# Whoz Browser Self-Installing Setup
$installPath = "$env:LOCALAPPDATA\Programs\WhozBrowser"
Write-Host "Installing Whoz Browser to $installPath..." -ForegroundColor Cyan

if (-not (Test-Path $installPath)) {
    New-Item -ItemType Directory -Path $installPath -Force | Out-Null
}

$sourceDir = $PSScriptRoot
Copy-Item -Path "$sourceDir\*" -Destination $installPath -Recurse -Force

# Create Desktop Shortcut
$wsh = New-Object -ComObject WScript.Shell
$desktop = [System.Environment]::GetFolderPath('Desktop')
$shortcut = $wsh.CreateShortcut("$desktop\Whoz Browser.lnk")
$shortcut.TargetPath = "$installPath\whoz-browser.exe"
$shortcut.IconLocation = "$installPath\whoz-browser.exe,0"
$shortcut.Description = "Whoz Browser - Ultra-Lightweight Chrome in Rust"
$shortcut.Save()

Write-Host "Installed successfully! Desktop shortcut created." -ForegroundColor Green
'@
Set-Content -Path "dist\install-whoz-browser.ps1" -Value $installerScript

Write-Host "[4/4] Package ready!" -ForegroundColor Green
Write-Host "Executable : $distDir\whoz.exe" -ForegroundColor White
Write-Host "Alias Exe  : $distDir\whoz-browser.exe" -ForegroundColor White
Write-Host "Installer  : dist\install-whoz-browser.ps1" -ForegroundColor White
