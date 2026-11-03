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
    @{ id = 'A1'; title = 'Start'; steps = @('Lihat window Whoz yang baru terbuka.'); expect = 'Google terbuka penuh. Tidak ada title bar, toolbar, atau tab. Hanya garis tipis di tepi atas dan kiri.' }

    @{ id = 'B1'; title = 'Toolbar muncul saat hover'; steps = @('Arahkan mouse ke tepi paling atas window dan diam sebentar.'); expect = 'Toolbar muncul: back, forward, reload, kolom alamat (gembok + domain tebal), bintang, menu (...), minimize, maximize, close.' }
    @{ id = 'B2'; title = 'Toolbar hilang sendiri'; steps = @('Pindahkan mouse ke tengah halaman, tunggu 1 detik.', 'Klik link di bagian atas halaman (mis. "Gmail").'); expect = 'Toolbar hilang. Link di bawah bekas toolbar bisa diklik.' }
    @{ id = 'B3'; title = 'Maximize / restore / double-click'; steps = @('Klik maximize, lalu restore.', 'Double-click area kosong toolbar dua kali.'); expect = 'Window maximize dan kembali; ikon berubah.' }
    @{ id = 'B4'; title = 'Geser dan resize window'; steps = @('Drag area kosong toolbar. Alt+drag di halaman.', 'Tarik tepi kiri, kanan, bawah, pojok, dan tepi atas.'); expect = 'Window bergeser dan bisa di-resize dari semua tepi. Tulis catatan untuk tepi atas.' }
    @{ id = 'B5'; title = 'Back / forward / reload'; steps = @('Buka example.com lalu klik "More information".', 'Klik back, forward, reload di toolbar.'); expect = 'Tombol aktif/nonaktif sesuai riwayat. Saat loading, reload jadi tombol stop.' }
    @{ id = 'B6'; title = 'Toolbar selalu tampil'; steps = @('Tekan Ctrl+Shift+B.', 'Tekan Ctrl+Shift+B lagi.'); expect = 'Pertama: toolbar dan sidebar tetap tampil, halaman bergeser memberi ruang. Kedua: kembali auto-hide.' }

    @{ id = 'C1'; title = 'Sidebar muncul / hilang'; steps = @('Arahkan mouse ke tepi kiri (bukan dekat atas), diam sebentar.', 'Gerakkan cepat melewati tepi kiri.', 'Pindah ke tengah halaman.'); expect = 'Hover lama: sidebar muncul. Lewat cepat: tidak muncul. Ke tengah: hilang.' }
    @{ id = 'C2'; title = 'Resize sidebar'; steps = @('Tekan Ctrl+Shift+B (toolbar selalu tampil).', 'Drag tepi kanan sidebar ke kanan lalu ke kiri.', 'Tekan Ctrl+Shift+B lagi.'); expect = 'Sidebar ikut lebar mouse (180-480 px), halaman menyesuaikan. Garis biru muncul saat drag.' }
    @{ id = 'C3'; title = 'Tab baru dan favicon'; steps = @('Ctrl+T, ketik: rust lang, Enter.', 'Munculkan sidebar.'); expect = 'Tab baru dengan hasil pencarian. Spinner saat loading, lalu favicon. Tab aktif menonjol.' }
    @{ id = 'C4'; title = 'Pindah tab'; steps = @('Klik tab lain di sidebar.', 'Coba Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+1, Ctrl+9.'); expect = 'Semua cara pindah ke tab yang benar.' }
    @{ id = 'C5'; title = 'Drag reorder tab'; steps = @('Buka 3 tab. Drag satu tab ke atas/bawah di sidebar.'); expect = 'Urutan tab berubah dan tetap setelah dilepas.' }
    @{ id = 'C6'; title = 'Menu klik kanan tab'; steps = @('Klik kanan sebuah tab.', 'Coba Pin, lalu Unpin. Duplicate. Close other tabs.'); expect = 'Pin: tab jadi ikon di atas daftar. Duplicate: salinan tab. Close other: sisa tab itu dan tab pinned.' }
    @{ id = 'C7'; title = 'Tutup dan buka lagi tab'; steps = @('Tutup tab dengan x, klik tengah, Ctrl+W.', 'Tekan Ctrl+Shift+T.'); expect = 'Tab tertutup; Ctrl+Shift+T membuka lagi tab terakhir yang ditutup.' }
    @{ id = 'C8'; title = 'Tab di atas'; steps = @('Menu (...) > Move tabs to the top.', 'Hover tepi atas, lalu tepi kiri.', 'Menu > Move tabs to the side.'); expect = 'Baris tab di atas toolbar. Tepi kiri tidak membuka sidebar. Lalu kembali ke samping.' }
    @{ id = 'C9'; title = 'Session restore'; steps = @('Buka 3 tab, pin satu.', 'Tutup Whoz dengan Alt+F4. Script akan membukanya lagi.'); expect = 'Tab, tab pinned, dan ukuran window kembali seperti semula.'; relaunch = $true }

    @{ id = 'D1'; title = 'Address bar'; steps = @('Klik kolom alamat (atau Ctrl+L).', 'Ketik: example.com, Enter.'); expect = 'Bar mengambang dengan URL terseleksi. Example Domain terbuka di tab yang sama.' }
    @{ id = 'D2'; title = 'Saran address bar'; steps = @('Ctrl+L, ketik sebagian nama situs yang pernah dibuka atau judul tab lain.', 'Panah bawah lalu Enter.'); expect = 'Muncul saran: Search, tab terbuka (Switch to tab), bookmark, history.' }
    @{ id = 'D3'; title = 'Tutup address bar'; steps = @('Ctrl+L lalu Esc. Ctrl+L lalu klik halaman.', 'Coba Alt+D, F6, Ctrl+Space.'); expect = 'Bar tertutup; ketiga shortcut membuka bar.' }

    @{ id = 'E1'; title = 'Bookmark'; steps = @('Buka github.com, tekan Ctrl+D.', 'Ganti nama, isi folder "Dev", Enter.', 'Klik bintang lagi, klik Remove.'); expect = 'Popover bookmark muncul di bawah bintang; bintang biru saat tersimpan, kosong setelah Remove.' }
    @{ id = 'E2'; title = 'Halaman bookmarks'; steps = @('Tambahkan 2 bookmark. Tekan Ctrl+Shift+O.', 'Edit satu, hapus satu. Klik "Import from Chrome".'); expect = 'whoz://bookmarks terbuka, dikelompokkan per folder. Edit/hapus jalan. Import menampilkan jumlah yang diimpor (atau pesan bila Chrome tidak ada).' }
    @{ id = 'E3'; title = 'History'; steps = @('Tekan Ctrl+H. Cari sebuah situs. Hapus satu entri (x).'); expect = 'Riwayat per hari, pencarian jalan, entri hilang setelah x.' }
    @{ id = 'E4'; title = 'Clear browsing data'; steps = @('Ctrl+Shift+Delete. Pilih "Last hour", hanya centang Browsing history, klik Clear data.'); expect = 'Toast "Browsing data cleared". History satu jam terakhir hilang.' }

    @{ id = 'F1'; title = 'Download'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.', 'Lihat tombol download di toolbar, klik.', 'Klik ikon folder pada item.'); expect = 'Toast "Downloading...", tombol download dengan progress, daftar dengan progress bar, Explorer membuka file.'; clip = 'https://www.python.org/ftp/python/3.12.0/python-3.12.0-embed-amd64.zip' }
    @{ id = 'F2'; title = 'Halaman downloads'; steps = @('Tekan Ctrl+J.'); expect = 'whoz://downloads menampilkan file dengan Open, Show in folder, Remove.' }

    @{ id = 'G1'; title = 'Install ekstensi dari Chrome Web Store'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.', 'Klik "Add to Whoz" di kolom alamat, tunggu.'); expect = 'Toast "Installing..." lalu "Added uBlock Origin Lite". Ikon puzzle muncul di toolbar.'; clip = 'https://chromewebstore.google.com/detail/ublock-origin-lite/ddkjiahejlhfcafbddmgiahcphecmpfh' }
    @{ id = 'G2'; title = 'Popup ekstensi'; steps = @('Klik ikon puzzle, pilih uBlock Origin Lite.', 'Klik di halaman.'); expect = 'Popup ekstensi muncul di bawah toolbar dengan ukuran pas. Klik di luar menutupnya. Tulis catatan bila tidak berfungsi.' }
    @{ id = 'G3'; title = 'Efek ekstensi'; steps = @('Buka situs berita yang banyak iklan.'); expect = 'Iklan terblokir. Tulis catatan kalau tidak.' }
    @{ id = 'G4'; title = 'Kelola ekstensi'; steps = @('Menu > Extensions. Matikan toggle, nyalakan lagi. Klik Options.'); expect = 'Toggle mengubah status; Options membuka halaman pengaturan ekstensi di tab baru.' }

    @{ id = 'H1'; title = 'Developer tools'; steps = @('Tekan F12. Tutup. Tekan Ctrl+Shift+I. Menu > Developer tools.'); expect = 'DevTools Chromium terbuka untuk halaman aktif.' }
    @{ id = 'H2'; title = 'Settings'; steps = @('Menu > Settings.', 'Ganti Theme ke Dark lalu System. Ganti search engine ke DuckDuckGo, lalu cari sesuatu di address bar.'); expect = 'Tema toolbar dan halaman berubah langsung. Pencarian memakai DuckDuckGo.' }
    @{ id = 'H3'; title = 'Sync folder'; steps = @('Settings > Sync > Choose folder..., pilih folder OneDrive/Google Drive (atau folder kosong apa saja).', 'Klik Sync now.'); expect = 'Status "Last synced just now". File whoz-sync.json ada di folder itu.' }
    @{ id = 'H4'; title = 'Zoom'; steps = @('Ctrl+Plus dua kali, lihat kolom alamat.', 'Menu > Zoom +/-. Klik angka % di kolom alamat.'); expect = 'Angka % muncul di kolom alamat; klik mengembalikan ke 100%.' }
    @{ id = 'H5'; title = 'Private window'; steps = @('Tekan Ctrl+Shift+N.', 'Buka sebuah situs, tutup window private itu.', 'Kembali ke window utama, Ctrl+H.'); expect = 'Window baru dengan label Private dan warna ungu. Situs itu tidak ada di history.' }
    @{ id = 'H6'; title = 'Window baru'; steps = @('Tekan Ctrl+N, lalu tutup window baru.'); expect = 'Window kedua terbuka di home page; window pertama tetap.' }

    @{ id = 'I1'; title = 'Popup jadi tab'; steps = @('URL sudah disalin. Ctrl+L, Ctrl+V, Enter.', 'Klik tombol "popup", tunggu 2 detik.'); expect = 'Tab baru muncul, lalu tertutup sendiri dan kembali ke tab tombol.'; clip = 'data:text/html,<button onclick="w=open(''about:blank'');setTimeout(()=>w.close(),2000)">popup</button>' }
    @{ id = 'I2'; title = 'F11 dan video fullscreen'; steps = @('F11, hover atas, F11 lagi.', 'YouTube: tekan f di player, lalu Esc.'); expect = 'Fullscreen tanpa toolbar; Esc kembali normal.' }
    @{ id = 'I3'; title = 'Audio dan mute tab'; steps = @('Putar video YouTube, buka tab lain.', 'Munculkan sidebar, klik ikon speaker pada tab YouTube.'); expect = 'Suara jalan di tab tersembunyi; ikon speaker muncul, klik membisukan.' }

    @{ id = 'M1'; title = 'Pemakaian RAM'; memory = $true }

    @{ id = 'Q1'; title = 'Tutup Whoz'; steps = @('Munculkan toolbar, klik close.'); expect = 'Whoz keluar.' }
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

# Start from default settings so the steps match. Your settings and tabs are kept in settings.json.bak.
$data = "$env:LOCALAPPDATA\WhozBrowser"
New-Item -ItemType Directory $data -Force | Out-Null
if (Test-Path "$data\settings.json") { Move-Item "$data\settings.json" "$data\settings.json.bak" -Force }

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
