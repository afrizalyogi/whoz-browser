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
