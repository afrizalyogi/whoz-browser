# Whoz Browser

A minimal Windows browser: the whole window is the web page. The header (minimize, maximize, close) and the tab list stay hidden and slide in only while you hover the window edge.

- Hover the **top edge**: header with window buttons, address bar (search icon), new tab, tab-layout toggle.
- Hover the **left edge**: vertical tab list (and header). Click a tab to switch, `×` or middle-click to close.
- The layout button (four squares) moves tabs between the **left sidebar** and the **header**. The choice is saved.
- Drag the empty header to move the window, double-click it to maximize.

Written in Rust (~400 lines) on [`wry`](https://github.com/tauri-apps/wry), using the Microsoft Edge WebView2 runtime that ships with Windows 10/11 (Chromium engine). `whoz.exe` is about 1.5 MB.

Developed by **PT Aryatama Teknologi Digital** (`afrizalyogi`).

## Shortcuts

| Key | Action |
|---|---|
| `Ctrl+L`, `Alt+D`, `F6`, `Ctrl+Space` | Address bar + tab list (overlay) |
| `Ctrl+T` | New tab (address bar opens empty) |
| `Ctrl+W`, `Ctrl+F4` | Close tab (closing the last tab quits) |
| `Ctrl+Tab` / `Ctrl+Shift+Tab`, `Ctrl+PgDn` / `Ctrl+PgUp` | Next / previous tab |
| `Ctrl+1`..`Ctrl+8`, `Ctrl+9` | Go to tab N / last tab |
| `Alt+←` / `Alt+→`, `F5`, `Ctrl+F`, `Ctrl+P`, `Ctrl +/-/0` | Back/forward, reload, find, print, zoom (built into WebView2) |
| `F11` | Fullscreen |
| `Alt+F4` | Quit |
| `Alt` + drag, or drag the header | Move the window |
| Drag a window edge | Resize |
| `Win+↑` / `Win+←` / `Win+→` | Maximize / snap |

In the overlay: type a URL or search words and press `Enter`; type to filter open tabs, pick one with `↑`/`↓` + `Enter` or a click; `Esc` closes it. Text with a dot (`example.com`) opens as a site, anything else searches Google.

Links that open a new window (`target=_blank`, `window.open`, Google sign-in popups) open as a new tab; closing it returns to the tab that opened it.

## Data

Profile (cookies, logins, history, cache) is stored in `%LOCALAPPDATA%\WhozBrowser\profile`.
Unpacked extensions: put each extension folder in `%LOCALAPPDATA%\WhozBrowser\extensions\` and restart. There is no Chrome Web Store and no Chrome/Google account sync: WebView2 does not support either.

## Run, build, install

```powershell
cargo run --release            # run from source
cargo test                     # URL parsing, escaping and shortcut tests
.\package.ps1                  # dist\whoz-browser\whoz.exe + zip
.\dist\whoz-browser\install.ps1   # per-user install + Start menu shortcut
```

`whoz.exe <url>` opens that URL instead of the home page.

Requirements: Windows 10/11 x64 with the WebView2 Runtime (preinstalled on Windows 11 and current Windows 10). Building needs Rust (MSVC toolchain).
