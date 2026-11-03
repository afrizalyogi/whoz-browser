# Builds whoz.exe and writes a portable folder plus a per-user installer to dist\.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$out = "dist\whoz-browser"
Remove-Item dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $out -Force | Out-Null
Copy-Item target\release\whoz.exe $out
Copy-Item README.md $out

Set-Content "$out\install.ps1" @'
# Installs Whoz for the current user: no admin rights needed.
$ErrorActionPreference = "Stop"
$dir = "$env:LOCALAPPDATA\Programs\Whoz"
New-Item -ItemType Directory $dir -Force | Out-Null
Copy-Item "$PSScriptRoot\whoz.exe" $dir -Force
$lnk = (New-Object -ComObject WScript.Shell).CreateShortcut("$([Environment]::GetFolderPath('Programs'))\Whoz.lnk")
$lnk.TargetPath = "$dir\whoz.exe"
$lnk.Save()
Write-Host "Installed to $dir (Start menu: Whoz)"
'@

Compress-Archive "$out\*" dist\whoz-browser-windows-x64.zip -Force
Write-Host "Done: $out\whoz.exe, dist\whoz-browser-windows-x64.zip"
