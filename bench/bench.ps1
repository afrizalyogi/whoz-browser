<#
.SYNOPSIS
Measures browsers the same way, so "faster and lighter" is a number, not a claim.

.DESCRIPTION
For each browser and scenario, launches a fresh profile, waits, and records:
  - memory: private working set of the whole process tree (what Task Manager shows), MB
  - cpu:    CPU seconds the tree used while idle for -IdleSeconds
  - startup: ms until the first window appears (WhozLite shows its window before the engine starts, so its number is not comparable)
  - speedometer: Speedometer 3.1 score (Chromium-based browsers only, through the DevTools Protocol)
Each scenario runs -Runs times; the summary uses the median.
No mouse or keyboard input is sent. Your own profiles are not touched.

.EXAMPLE
.\bench\bench.ps1 -Browsers Whoz,Chrome,Edge -Runs 3
.\bench\bench.ps1 -Browsers Whoz,Chrome -Gate Chrome    # exit 1 if Whoz loses
#>
param(
    [string[]]$Browsers = @('Whoz', 'Chrome', 'Edge', 'Firefox', 'Zen', 'Brave', 'Helium', 'WhozLite'),
    [string[]]$Scenarios = @('example', 'wikipedia', 'github', 'youtube', 'tabs10', 'speedometer'),
    [int]$Runs = 3,
    [int]$Wait = 20,
    [int]$IdleSeconds = 15,
    [string]$Out = "$PSScriptRoot\results",
    [string]$WhozPath = '',
    [string]$Gate = '',
    # Same engine, same V8: a few percent is noise, so speed may be this fraction of the reference and still pass.
    [double]$SpeedTolerance = 0.97
)
$ErrorActionPreference = 'Stop'
# powershell -File passes "A,B" as one string.
$Browsers = $Browsers -split ','
$Scenarios = $Scenarios -split ','

$sites = [ordered]@{
    example   = 'https://example.com/'
    wikipedia = 'https://en.wikipedia.org/wiki/Web_browser'
    github    = 'https://github.com/chromium/chromium'
    youtube   = 'https://www.youtube.com/'
}
$tabs10 = @(
    'https://example.com/', 'https://en.wikipedia.org/wiki/Web_browser', 'https://github.com/chromium/chromium',
    'https://www.youtube.com/', 'https://news.ycombinator.com/', 'https://developer.mozilla.org/en-US/',
    'https://www.bbc.com/news', 'https://stackoverflow.com/questions', 'https://www.reddit.com/r/programming/',
    'https://docs.github.com/en'
)
$speedometerUrl = 'https://browserbench.org/Speedometer3.1/?startAutomatically'

function First-Existing([string[]]$paths) { $paths | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1 }

$pf = $env:ProgramFiles; $pf86 = ${env:ProgramFiles(x86)}; $la = $env:LOCALAPPDATA
$exe = @{
    Whoz     = First-Existing @($WhozPath, "$la\Whoz\Application\chrome.exe", "$PSScriptRoot\..\build\src\out\Default\chrome.exe")
    Chrome   = First-Existing @("$pf\Google\Chrome\Application\chrome.exe", "$pf86\Google\Chrome\Application\chrome.exe", "$la\Google\Chrome\Application\chrome.exe")
    Edge     = First-Existing @("$pf86\Microsoft\Edge\Application\msedge.exe", "$pf\Microsoft\Edge\Application\msedge.exe")
    Brave    = First-Existing @("$pf\BraveSoftware\Brave-Browser\Application\brave.exe", "$la\BraveSoftware\Brave-Browser\Application\brave.exe")
    Helium   = First-Existing @("$la\imput\Helium\Application\chrome.exe", "$pf\imput\Helium\Application\chrome.exe")
    Firefox  = First-Existing @("$pf\Mozilla Firefox\firefox.exe", "$pf86\Mozilla Firefox\firefox.exe")
    Zen      = First-Existing @("$pf\Zen Browser\zen.exe", "$la\Zen Browser\zen.exe")
    WhozLite = First-Existing @("$PSScriptRoot\..\lite\target\release\whoz.exe", "$la\Programs\Whoz\whoz.exe")
}
$chromium = 'Whoz', 'Chrome', 'Edge', 'Brave', 'Helium'
$gecko = 'Firefox', 'Zen'

$tmp = Join-Path $env:TEMP "whoz-bench-$PID"
New-Item -ItemType Directory $tmp, $Out -Force | Out-Null

# --- process tree --------------------------------------------------------------------------------
function Get-Tree([int]$root) {
    $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, CreationDate
    $byParent = $all | Group-Object ParentProcessId -AsHashTable -AsString
    $ids = New-Object System.Collections.Generic.List[int]
    $queue = New-Object System.Collections.Generic.Queue[int]
    $queue.Enqueue($root)
    while ($queue.Count) {
        $id = $queue.Dequeue()
        if ($ids.Contains($id)) { continue }
        $ids.Add($id)
        if ($byParent -and $byParent.ContainsKey("$id")) { $byParent["$id"] | ForEach-Object { $queue.Enqueue([int]$_.ProcessId) } }
    }
    , $ids.ToArray()
}

function Measure-Tree([int[]]$ids) {
    $perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids -contains [int]$_.IDProcess }
    [math]::Round(($perf | Measure-Object WorkingSetPrivate -Sum).Sum / 1MB)
}

function Cpu-Snapshot([int[]]$ids) {
    $h = @{}
    foreach ($id in $ids) { $p = Get-Process -Id $id -ErrorAction SilentlyContinue; if ($p) { $h[$id] = $p.TotalProcessorTime.TotalSeconds } }
    $h
}

# CPU used between two snapshots. Processes that exited in between are left out rather than counted negative.
function Cpu-Delta($before, $after) {
    $sum = 0.0
    foreach ($id in $after.Keys) { $sum += $after[$id] - $(if ($before.ContainsKey($id)) { $before[$id] } else { 0 }) }
    $sum
}

function Stop-Tree([int]$root) {
    Get-Tree $root | ForEach-Object { Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }
}

# --- launching -----------------------------------------------------------------------------------
function Start-Browser([string]$name, [string]$profileDir, [string[]]$urls, [int]$port) {
    $path = $exe[$name]
    if ($name -eq 'WhozLite') {
        $psi = New-Object System.Diagnostics.ProcessStartInfo $path, ('"' + $urls[0] + '"')
        $psi.UseShellExecute = $false
        $psi.EnvironmentVariables['LOCALAPPDATA'] = $profileDir
        if ($port) { $psi.EnvironmentVariables['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'] = "--remote-debugging-port=$port" }
        return [System.Diagnostics.Process]::Start($psi)
    }
    if ($gecko -contains $name) {
        $args = @('-no-remote', '-profile', "`"$profileDir`"", '-width', '1280', '-height', '900') + ($urls | ForEach-Object { "`"$_`"" })
    } else {
        $args = @("--user-data-dir=`"$profileDir`"", '--no-first-run', '--no-default-browser-check', '--disable-search-engine-choice-screen',
            '--window-size=1280,900', '--window-position=40,40')
        if ($port) { $args += "--remote-debugging-port=$port" }
        $args += $urls | ForEach-Object { "`"$_`"" }
    }
    Start-Process $path -ArgumentList $args -PassThru
}

function Wait-Window([System.Diagnostics.Process]$p, [int]$timeoutMs = 30000) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $timeoutMs) {
        $p.Refresh()
        if ($p.HasExited) { return $null }
        if ($p.MainWindowHandle -ne 0) { return $sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 25
    }
    $null
}

# --- DevTools Protocol (Speedometer) -------------------------------------------------------------
function Invoke-Cdp([string]$wsUrl, [string]$expression) {
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $ct = [Threading.CancellationToken]::None
    $ws.ConnectAsync([Uri]$wsUrl, $ct).Wait()
    $msg = @{ id = 1; method = 'Runtime.evaluate'; params = @{ expression = $expression; returnByValue = $true } } | ConvertTo-Json -Compress -Depth 4
    $bytes = [Text.Encoding]::UTF8.GetBytes($msg)
    $ws.SendAsync((New-Object ArraySegment[byte] (, $bytes)), 'Text', $true, $ct).Wait()
    $buf = New-Object byte[] 65536
    $sb = New-Object Text.StringBuilder
    do {
        $r = $ws.ReceiveAsync((New-Object ArraySegment[byte] (, $buf)), $ct).Result
        $sb.Append([Text.Encoding]::UTF8.GetString($buf, 0, $r.Count)) | Out-Null
    } until ($r.EndOfMessage)
    $ws.Dispose()
    ($sb.ToString() | ConvertFrom-Json).result.result.value
}

function Get-Speedometer([int]$port, [int]$timeoutSec = 900) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $timeoutSec) {
        Start-Sleep 5
        try {
            $page = (Invoke-RestMethod "http://127.0.0.1:$port/json" -TimeoutSec 5) | Where-Object { $_.type -eq 'page' -and $_.url -like '*Speedometer*' } | Select-Object -First 1
            if (-not $page) { continue }
            # Speedometer 3.1 switches to #summary when done; result-number then holds the score or "Error".
            $res = Invoke-Cdp $page.webSocketDebuggerUrl "location.hash === '#summary' ? document.getElementById('result-number').textContent : ''"
            if ($res) { $n = 0.0; if ([double]::TryParse($res, [ref]$n)) { return $n } else { return $null } }
        } catch { }
    }
    $null
}

# --- run -----------------------------------------------------------------------------------------
$rows = New-Object System.Collections.Generic.List[object]
$port = 9333
foreach ($b in $Browsers) {
    if (-not $exe[$b]) { Write-Host "skip $b (not installed)"; continue }
    foreach ($s in $Scenarios) {
        if ($s -eq 'speedometer' -and $chromium -notcontains $b) { continue }   # needs the DevTools Protocol
        if ($s -eq 'tabs10' -and $b -eq 'WhozLite') { continue }               # opens one URL per launch
        for ($i = 1; $i -le $Runs; $i++) {
            $dir = Join-Path $tmp "$b-$s-$i"
            New-Item -ItemType Directory $dir -Force | Out-Null
            $urls = if ($s -eq 'tabs10') { $tabs10 } elseif ($s -eq 'speedometer') { @($speedometerUrl) } else { @($sites[$s]) }
            $p = if ($s -eq 'speedometer') { $port++; Start-Browser $b $dir $urls $port } else { Start-Browser $b $dir $urls 0 }
            $startup = Wait-Window $p
            $row = [ordered]@{ browser = $b; scenario = $s; run = $i; startupMs = $startup; memoryMB = $null; idleCpuSec = $null; speedometer = $null; processes = $null }
            if ($s -eq 'speedometer') {
                $row.speedometer = Get-Speedometer $port
            } else {
                Start-Sleep $Wait
                $ids = Get-Tree $p.Id
                $row.processes = $ids.Count
                $row.memoryMB = Measure-Tree $ids
                $c0 = Cpu-Snapshot $ids
                Start-Sleep $IdleSeconds
                $row.idleCpuSec = [math]::Round((Cpu-Delta $c0 (Cpu-Snapshot (Get-Tree $p.Id))), 2)
            }
            $rows.Add([pscustomobject]$row)
            Write-Host ("{0,-8} {1,-11} #{2}  mem {3,5} MB  idle cpu {4,5} s  startup {5,5} ms  speedometer {6}" -f $b, $s, $i, $row.memoryMB, $row.idleCpuSec, $row.startupMs, $row.speedometer)
            Stop-Tree $p.Id
            Start-Sleep 3
            Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# --- summary -------------------------------------------------------------------------------------
function Median($xs) {
    $v = @($xs | Where-Object { $_ -ne $null } | Sort-Object)
    if (-not $v.Count) { return $null }
    if ($v.Count % 2) { return $v[[int][math]::Floor($v.Count / 2)] }
    ($v[$v.Count / 2 - 1] + $v[$v.Count / 2]) / 2
}
$summary = $rows | Group-Object browser, scenario | ForEach-Object {
    $g = $_.Group
    [pscustomobject]@{
        browser = $g[0].browser; scenario = $g[0].scenario
        memoryMB = Median $g.memoryMB; idleCpuSec = Median $g.idleCpuSec; startupMs = Median $g.startupMs; speedometer = Median $g.speedometer
    }
}
$installMB = @{}
foreach ($b in $Browsers) {
    if ($exe[$b]) { $installMB[$b] = [math]::Round((Get-ChildItem (Split-Path $exe[$b]) -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum / 1MB) }
}
$os = Get-CimInstance Win32_OperatingSystem
$cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
$versions = @{}
foreach ($b in $Browsers) { if ($exe[$b]) { $versions[$b] = (Get-Item $exe[$b]).VersionInfo.ProductVersion } }
$report = [ordered]@{
    date = (Get-Date).ToString('s'); machine = @{ cpu = $cpu; ramGB = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1); freeRamGB = [math]::Round($os.FreePhysicalMemory / 1MB, 1); os = $os.Caption }
    runs = $Runs; waitSeconds = $Wait; versions = $versions; installMB = $installMB; summary = $summary; rows = $rows
}
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$report | ConvertTo-Json -Depth 5 | Set-Content "$Out\bench-$stamp.json"
Copy-Item "$Out\bench-$stamp.json" "$Out\latest.json" -Force
$summary | Sort-Object scenario, browser | Format-Table scenario, browser, memoryMB, idleCpuSec, startupMs, speedometer -AutoSize | Out-String | Write-Host
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

# --- gate ----------------------------------------------------------------------------------------
if ($Gate) {
    $fail = @()
    foreach ($s in $Scenarios) {
        $w = $summary | Where-Object { $_.browser -eq 'Whoz' -and $_.scenario -eq $s }
        $r = $summary | Where-Object { $_.browser -eq $Gate -and $_.scenario -eq $s }
        if (-not $w -or -not $r) { continue }
        if ($s -eq 'speedometer') {
            if ($w.speedometer -and $r.speedometer -and $w.speedometer -lt $r.speedometer * $SpeedTolerance) { $fail += "$s : Speedometer $($w.speedometer) < $Gate $($r.speedometer) x $SpeedTolerance" }
        } elseif ($w.memoryMB -gt $r.memoryMB) {
            $fail += "$s : memory $($w.memoryMB) MB > $Gate $($r.memoryMB) MB"
        }
    }
    if ($fail) { Write-Host "GATE FAILED vs ${Gate}:"; $fail | ForEach-Object { Write-Host "  $_" }; exit 1 }
    Write-Host "GATE PASSED vs $Gate"
}
