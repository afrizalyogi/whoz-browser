# Builds whoz.exe and writes a portable folder plus a per-user installer to dist\.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# Through cmd so Windows PowerShell 5 does not treat cargo's progress on stderr as an error.
cmd /c "cargo build --release 2>&1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$out = "dist\whoz-browser"
Remove-Item dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $out -Force | Out-Null
Copy-Item target\release\whoz.exe $out
Copy-Item README.md, LICENSE $out

Set-Content "$out\install.ps1" @'
# Installs Whoz for the current user: no admin rights needed.
$ErrorActionPreference = "Stop"
$dir = "$env:LOCALAPPDATA\Programs\Whoz"
New-Item -ItemType Directory $dir -Force | Out-Null
Copy-Item "$PSScriptRoot\whoz.exe" $dir -Force
$exe = "$dir\whoz.exe"
$lnk = (New-Object -ComObject WScript.Shell).CreateShortcut("$([Environment]::GetFolderPath('Programs'))\Whoz.lnk")
$lnk.TargetPath = $exe
$lnk.Save()
# Register as a browser so Windows lists Whoz under Settings > Apps > Default apps.
$r = "HKCU:\Software\Clients\StartMenuInternet\Whoz"
$cls = "HKCU:\Software\Classes\WhozHTML"
New-Item "$cls\shell\open\command" -Force | Out-Null
Set-ItemProperty $cls "(default)" "Whoz HTML Document"
Set-ItemProperty "$cls\shell\open\command" "(default)" "`"$exe`" `"%1`""
New-Item "$r\Capabilities\URLAssociations" -Force | Out-Null
New-Item "$r\Capabilities\FileAssociations" -Force | Out-Null
New-Item "$r\shell\open\command" -Force | Out-Null
Set-ItemProperty $r "(default)" "Whoz"
Set-ItemProperty "$r\shell\open\command" "(default)" "`"$exe`""
Set-ItemProperty "$r\Capabilities" "ApplicationName" "Whoz"
Set-ItemProperty "$r\Capabilities" "ApplicationDescription" "A minimal browser"
foreach ($s in "http", "https") { Set-ItemProperty "$r\Capabilities\URLAssociations" $s "WhozHTML" }
foreach ($e in ".htm", ".html", ".pdf", ".svg") { Set-ItemProperty "$r\Capabilities\FileAssociations" $e "WhozHTML" }
# Never New-Item -Force here: on an existing key it would wipe other apps' registrations.
if (-not (Test-Path "HKCU:\Software\RegisteredApplications")) { New-Item "HKCU:\Software\RegisteredApplications" | Out-Null }
Set-ItemProperty "HKCU:\Software\RegisteredApplications" "Whoz" "Software\Clients\StartMenuInternet\Whoz\Capabilities"
Write-Host "Installed to $dir (Start menu: Whoz). To make it your default browser: Settings > Apps > Default apps > Whoz."
'@

Compress-Archive "$out\*" dist\whoz-browser-windows-x64.zip -Force
Write-Host "Done: $out\whoz.exe, dist\whoz-browser-windows-x64.zip"
