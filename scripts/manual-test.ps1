# Manual test for Whoz: walks through each check, records pass/fail, writes a Markdown report.
# Run:  powershell -ExecutionPolicy Bypass -File scripts\manual-test.ps1   (optional: -Exe <path to whoz.exe>)
param([string]$Exe = "")
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot
if (-not $Exe) {
    $Exe = @("$root\dist\whoz-browser\whoz.exe", "$root\target\release\whoz.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $Exe -or -not (Test-Path $Exe)) { throw "whoz.exe tidak ditemukan. Jalankan .\package.ps1 dulu, atau pakai -Exe <path>." }

# Each test: id, title, steps, expect. Optional: clip (copied to clipboard), relaunch (Whoz is closed in the steps
# and reopened before you judge), memory (records RAM, no question).
$tests = @(
    @{ id = 'A1'; title = 'Start'; steps = @('Lihat window Whoz yang baru terbuka.'); expect = 'Google terbuka. Tidak ada title bar, toolbar, atau tab. Hanya garis tipis di tepi atas dan kiri.' }

    @{ id = 'B1'; title = 'Header muncul saat hover'; steps = @('Arahkan mouse ke tepi paling atas window (1-4 px) dan diam sebentar.'); expect = 'Header muncul: judul halaman, ikon cari, +, empat kotak, minimize, maximize, close.' }
    @{ id = 'B2'; title = 'Header hilang sendiri'; steps = @('Setelah header muncul, pindahkan mouse ke tengah halaman dan tunggu 1 detik.', 'Klik link di bagian atas halaman (mis. "Gmail" di pojok kanan atas Google).'); expect = 'Header hilang dalam +/- 0,5 detik. Link di bawah bekas header bisa diklik.' }
    @{ id = 'B3'; title = 'Tombol maximize / restore'; steps = @('Munculkan header, klik maximize.', 'Munculkan lagi, klik restore.'); expect = 'Window maximize lalu kembali. Ikon tombol berubah sesuai status.' }
    @{ id = 'B4'; title = 'Double-click header'; steps = @('Double-click area kosong di header.', 'Ulangi sekali lagi.'); expect = 'Maximize, lalu restore.' }
    @{ id = 'B5'; title = 'Geser window'; steps = @('Drag area kosong di header.', 'Tahan Alt lalu drag di mana saja di halaman.'); expect = 'Window ikut bergeser di kedua cara.' }
    @{ id = 'B6'; title = 'Tombol minimize'; steps = @('Munculkan header, klik minimize.', 'Buka lagi dari taskbar.'); expect = 'Window minimize. Setelah dibuka, halaman masih sama.' }
    @{ id = 'B7'; title = 'Resize window'; steps = @('Saat tidak maximize: tarik tepi kiri, kanan, bawah, dan pojok.', 'Arahkan ke tepi paling atas sampai kursor jadi panah resize, lalu tarik.'); expect = 'Semua tepi bisa resize. Tulis di catatan apa yang terjadi di tepi atas.' }

    @{ id = 'C1'; title = 'Sidebar tab muncul saat hover'; steps = @('Arahkan mouse ke tepi paling kiri (bukan dekat atas) dan diam sebentar.'); expect = 'Sidebar tab muncul di kiri bersama header.' }
    @{ id = 'C2'; title = 'Sidebar tidak muncul kalau cuma lewat'; steps = @('Gerakkan mouse cepat melewati tepi kiri, misalnya menuju border untuk resize.'); expect = 'Sidebar tidak muncul kalau mouse hanya lewat cepat.' }
    @{ id = 'C3'; title = 'Sidebar hilang sendiri'; steps = @('Dengan sidebar terbuka, pindahkan mouse ke tengah halaman.'); expect = 'Sidebar dan header hilang.' }
    @{ id = 'C4'; title = 'Tab baru'; steps = @('Tekan Ctrl+T, ketik: rust lang, tekan Enter.', 'Munculkan sidebar.'); expect = 'Tab baru berisi hasil pencarian Google. Sidebar menampilkan 2 tab, tab aktif ter-highlight.' }
    @{ id = 'C5'; title = 'Pindah tab'; steps = @('Klik tab pertama di sidebar.', 'Coba Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1, Ctrl+9.'); expect = 'Setiap cara pindah ke tab yang benar.' }
    @{ id = 'C6'; title = 'Tutup tab'; steps = @('Buka beberapa tab dengan Ctrl+T (sisakan minimal 2).', 'Tutup satu dengan tombol x di sidebar, satu dengan klik tengah, satu dengan Ctrl+W.'); expect = 'Tab tertutup sesuai yang dipilih. Setelah menutup tab aktif, kembali ke tab terakhir yang dipakai.' }
    @{ id = 'C7'; title = 'Tab di header'; steps = @('Munculkan header, klik ikon empat kotak.', 'Hover tepi atas, lalu hover tepi kiri.', 'Klik ikon empat kotak lagi.'); expect = 'Tab pindah ke header. Tepi kiri tidak membuka sidebar. Klik kedua mengembalikan tab ke kiri.' }
    @{ id = 'C8'; title = 'Pilihan posisi tab tersimpan'; steps = @('Pindahkan tab ke header (ikon empat kotak).', 'Tutup Whoz dengan Alt+F4. Script akan membukanya lagi.', 'Setelah terbuka: hover tepi atas. Lalu kembalikan tab ke kiri.'); expect = 'Setelah dibuka ulang, tab tetap di header.'; relaunch = $true }

    @{ id = 'D1'; title = 'Address bar'; steps = @('Tekan Ctrl+L.', 'Ketik: example.com, tekan Enter.'); expect = 'Bar muncul dengan URL terseleksi. Tab yang sama membuka Example Domain.' }
    @{ id = 'D2'; title = 'Filter tab'; steps = @('Buka 2-3 tab berbeda.', 'Tekan Ctrl+L, ketik sebagian judul salah satu tab, tekan panah bawah lalu Enter.'); expect = 'Daftar tersaring, Enter pindah ke tab itu.' }
    @{ id = 'D3'; title = 'Tutup address bar'; steps = @('Ctrl+L lalu Esc.', 'Ctrl+L lalu klik di halaman.'); expect = 'Bar tertutup di kedua cara, halaman tetap bisa diketik/diklik.' }
    @{ id = 'D4'; title = 'Tombol cari dan + di header'; steps = @('Klik ikon cari di header.', 'Esc, lalu klik ikon +.'); expect = 'Ikon cari membuka bar berisi URL sekarang. Ikon + membuka bar kosong untuk tab baru.' }
    @{ id = 'D5'; title = 'Shortcut lain address bar'; steps = @('Coba Alt+D, F6, Ctrl+Space (Esc di antaranya).'); expect = 'Ketiganya membuka address bar.' }

    @{ id = 'E1'; title = 'Link target=_blank jadi tab'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.', 'Klik link "link".'); expect = 'Example Domain terbuka di tab baru.'; clip = 'data:text/html,<a href="https://example.com" target="_blank">link</a>' }
    @{ id = 'E2'; title = 'Popup menutup diri'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.', 'Klik tombol "popup", tunggu 2 detik.'); expect = 'Tab baru muncul, lalu tertutup sendiri dan kembali ke tab tombol.'; clip = 'data:text/html,<button onclick="w=open(''about:blank'');setTimeout(()=>w.close(),2000)">popup</button>' }
    @{ id = 'E3'; title = 'Login lewat popup (opsional)'; steps = @('Buka situs dengan tombol "Sign in with Google" (mis. canva.com) dan login.'); expect = 'Popup login terbuka sebagai tab, setelah login kembali ke situs dalam keadaan login.' }

    @{ id = 'F1'; title = 'F11 fullscreen'; steps = @('Tekan F11, hover tepi atas.', 'Tekan F11 lagi.'); expect = 'Fullscreen tanpa header (hover tidak memunculkan apa pun). F11 kedua kembali normal.' }
    @{ id = 'F2'; title = 'Fullscreen video'; steps = @('Buka video YouTube, tekan f di player.', 'Tekan Esc, lalu hover tepi atas.'); expect = 'Video memenuhi layar. Esc kembali ke window biasa, header bisa muncul lagi.' }
    @{ id = 'F3'; title = 'Audio di tab tersembunyi'; steps = @('Putar video YouTube, lalu Ctrl+T dan buka situs lain.'); expect = 'Suara tetap jalan saat tab YouTube tidak terlihat.' }

    @{ id = 'G1'; title = 'Download file'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.'); expect = 'Tulis di catatan apa yang terlihat (popup download? file masuk folder Downloads?).'; clip = 'https://www.python.org/ftp/python/3.12.0/python-3.12.0-embed-amd64.zip' }
    @{ id = 'G2'; title = 'Izin kamera / mikrofon'; steps = @('Buka https://webcamtests.com dan klik tes kamera.'); expect = 'Tulis di catatan: muncul prompt izin atau tidak, kamera jalan atau tidak.' }
    @{ id = 'G3'; title = 'Login tersimpan'; steps = @('Login ke situs apa saja (mis. github.com).', 'Tutup Whoz dengan Alt+F4. Script akan membukanya lagi.', 'Buka situs itu lagi.'); expect = 'Masih dalam keadaan login.'; relaunch = $true }
    @{ id = 'G4'; title = 'Shortcut bawaan'; steps = @('Coba Alt+Kiri / Alt+Kanan, F5, Ctrl+F, Ctrl+Plus / Ctrl+Minus / Ctrl+0.'); expect = 'Back, forward, reload, find, zoom berfungsi.' }
    @{ id = 'G5'; title = 'Tema gelap / terang (opsional)'; steps = @('Ganti tema Windows (Settings > Personalization > Colors), lalu munculkan header.'); expect = 'Warna header dan sidebar mengikuti tema.' }

    @{ id = 'M1'; title = 'Pemakaian RAM'; memory = $true }

    @{ id = 'Q1'; title = 'Tutup tab terakhir'; steps = @('Tekan Ctrl+W sampai semua tab tertutup.'); expect = 'Whoz keluar saat tab terakhir ditutup.' }
    @{ id = 'Q2'; title = 'Tombol close'; steps = @('(Script membuka Whoz lagi.) Munculkan header, klik close.'); expect = 'Whoz keluar.' }
)

function Start-Whoz {
    if (-not (Get-Process whoz -ErrorAction SilentlyContinue)) { Start-Process $Exe | Out-Null; Start-Sleep 3 }
}

function Get-WhozMemoryMB {
    $self = (Get-Process whoz -ErrorAction SilentlyContinue | Measure-Object WorkingSet64 -Sum).Sum
    $engine = Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -like '*WhozBrowser*' } |
        ForEach-Object { (Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue).WorkingSet64 } |
        Measure-Object -Sum
    [math]::Round(($self + $engine.Sum) / 1MB)
}

$wvKey = '\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$webview = @("HKLM:\SOFTWARE\WOW6432Node$wvKey", "HKCU:\Software$wvKey") |
    ForEach-Object { (Get-ItemProperty $_ -ErrorAction SilentlyContinue).pv } | Select-Object -First 1
$dpi = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -ErrorAction SilentlyContinue).AppliedDPI
$commit = try { git -C $root rev-parse --short HEAD 2>$null } catch { $null }
$os = Get-CimInstance Win32_OperatingSystem

# Start from the default layout so C7/C8 behave as described.
New-Item -ItemType Directory "$env:LOCALAPPDATA\WhozBrowser" -Force | Out-Null
Set-Content "$env:LOCALAPPDATA\WhozBrowser\tabs" 'left'

Write-Host "Whoz manual test: $($tests.Count) langkah. Jawab p (pass), f (fail), s (skip), q (berhenti dan simpan)." -ForegroundColor Green
Write-Host "Whoz: $Exe"

$results = @()
$memory = $null
foreach ($t in $tests) {
    Start-Whoz
    if ($t.memory) {
        $tabs = Read-Host "`n[$($t.id)] Berapa tab yang sedang terbuka di Whoz?"
        $memory = "$(Get-WhozMemoryMB) MB working set dengan $tabs tab (whoz.exe + proses msedgewebview2 Whoz)"
        Write-Host "  $memory"
        continue
    }
    Write-Host "`n[$($t.id)] $($t.title)" -ForegroundColor Cyan
    $t.steps | ForEach-Object { Write-Host "  - $_" }
    Write-Host "  Harapan: $($t.expect)" -ForegroundColor DarkGray
    if ($t.clip) { Set-Clipboard $t.clip }
    if ($t.relaunch) {
        Read-Host "  Tekan Enter setelah Whoz tertutup" | Out-Null
        Get-Process whoz -ErrorAction SilentlyContinue | Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
        Start-Whoz
        Write-Host "  Whoz dibuka lagi. Lanjutkan langkah terakhir."
    }
    do { $a = (Read-Host "  Hasil [p/f/s/q]").Trim().ToLower() } until ($a -in 'p', 'f', 's', 'q')
    if ($a -eq 'q') { break }
    $note = Read-Host $(if ($a -eq 'f') { '  Apa yang terjadi?' } else { '  Catatan (Enter untuk lewati)' })
    $results += [pscustomobject]@{ Id = $t.id; Title = $t.title; Result = @{ p = 'PASS'; f = 'FAIL'; s = 'SKIP' }[$a]; Note = $note }
}

$count = { param($r) @($results | Where-Object Result -eq $r).Count }
$lines = @(
    '# Whoz manual test report', ''
    "- Tanggal: $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
    "- Windows: $($os.Caption) $($os.Version)"
    "- WebView2: $webview"
    "- Skala layar: $(if ($dpi) { [math]::Round($dpi / 96 * 100) } else { '?' })%"
    "- Commit: $commit"
    "- whoz.exe: $Exe ($([math]::Round((Get-Item $Exe).Length / 1KB)) KB)"
    "- RAM: $memory"
    "- Ringkasan: PASS $(& $count 'PASS'), FAIL $(& $count 'FAIL'), SKIP $(& $count 'SKIP')", ''
    '| ID | Tes | Hasil | Catatan |', '|---|---|---|---|'
)
$lines += $results | ForEach-Object { "| $($_.Id) | $($_.Title) | $($_.Result) | $($_.Note -replace '\|', '/') |" }

$out = Join-Path (Get-Location) "whoz-test-report-$(Get-Date -Format 'yyyyMMdd-HHmm').md"
$lines | Set-Content $out -Encoding UTF8
$lines -join "`r`n" | Set-Clipboard
Write-Host "`nLaporan disimpan: $out" -ForegroundColor Green
Write-Host 'Laporan juga sudah disalin ke clipboard. Tempel (Ctrl+V) di chat.' -ForegroundColor Green
