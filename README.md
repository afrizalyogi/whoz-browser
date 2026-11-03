<div align="center">

# Whoz

**A Windows browser that gets out of the way.**

The page fills the whole window. Window buttons and tabs stay hidden until your mouse touches the edge.

[Download for Windows](https://github.com/afrizalyogi/whoz-browser/releases/latest) · [Features](#features) · [Compare](#how-it-compares) · [Shortcuts](#keyboard-shortcuts) · [Build](#build-from-source)

![Whoz demo: idle window, header on hover, vertical tabs, tabs in the header, address bar](docs/assets/demo.gif)

</div>

## Why Whoz

Most browsers spend 80–130 pixels at the top of the screen on a tab strip, an address bar, and a bookmarks bar, all of them visible all the time. On a laptop that is 10% of the screen you look at all day.

Whoz shows the web page edge to edge, apart from a 4 px hover strip along the top and left. Move the mouse to the top edge and the window buttons appear. Move it to the left edge and your tabs slide in. Move away and they leave.

It is a 535 KB program, written in about 800 lines of Rust and HTML, running on the Microsoft Edge WebView2 engine that already ships with Windows 10 and 11. Websites render exactly as they do in Edge and Chrome.

## Features

| | |
|---|---|
| **Edge-to-edge pages** | No toolbar, tab strip or title bar. The window is the page, plus a 4 px hover strip on the top and left edges. |
| **Auto-hide header** | Hover the top edge for minimize, maximize/restore and close, plus address bar, new tab and layout buttons. Drag the header to move the window; double-click to maximize. |
| **Vertical tabs** | Hover the left edge for a tab list with full page titles. Click to switch; `×` or middle-click to close. |
| **Or tabs on top** | One click moves the tabs into the header instead. Whoz remembers your choice. |
| **Address bar on demand** | `Ctrl+L` opens a floating bar. Type a URL or search words, or filter your open tabs and jump to one. |
| **Popups become tabs** | `target=_blank` links and `window.open` (Google and GitHub sign-in included) open as tabs and keep `window.opener`. When the popup closes itself you return to the tab that opened it. |
| **Hidden tabs use less memory** | Tabs you are not looking at are told to release memory (WebView2 `MemoryUsageTargetLevel = Low`). |
| **Keyboard first** | Every action has a shortcut. See [below](#keyboard-shortcuts). |
| **Separate profile** | Cookies and logins live in `%LOCALAPPDATA%\WhozBrowser\profile`, apart from Edge and Chrome. |
| **Unpacked extensions** | Put extension folders in `%LOCALAPPDATA%\WhozBrowser\extensions\`. |
| **Tiny and portable** | One 535 KB `whoz.exe` (263 KB zipped). No installer required, no admin rights. |

<table>
<tr>
<td><img src="docs/assets/idle.png" alt="Idle: page only"><br><sub>Idle: only the page</sub></td>
<td><img src="docs/assets/header.png" alt="Header revealed on hover"><br><sub>Top edge: window buttons</sub></td>
</tr>
<tr>
<td><img src="docs/assets/tabs-left.png" alt="Vertical tabs on the left"><br><sub>Left edge: vertical tabs</sub></td>
<td><img src="docs/assets/address-bar.png" alt="Address bar overlay"><br><sub>Ctrl+L: address bar and tab filter</sub></td>
</tr>
</table>

## How it compares

Checked against each browser's own documentation and release notes in **November 2026**. Sizes were measured on one Windows 11 x64 machine (Chrome 156, WebView2 156).

| | **Whoz** | Chrome | Edge | Firefox | Zen |
|---|---|---|---|---|---|
| Engine | Chromium (Edge WebView2) | Chromium | Chromium | Gecko | Gecko |
| Page fills the whole window by default | ✅ | ❌ | ❌ | ❌ | Optional (Compact Mode) |
| Window buttons hidden until hover | ✅ | ❌ | ❌ | ❌ | ✅ in Compact Mode ([docs](https://docs.zen-browser.app/user-manual/compact-mode)) |
| Vertical tabs | ✅ auto-hide | ✅ since Apr 2026 ([Google](https://blog.google/products-and-platforms/products/chrome/new-chrome-productivity-features/)) | ✅ since 2021, can hide the title bar ([Winaero](https://winaero.com/hide-title-bar-with-vertical-tabs-in-microsoft-edge/)) | ✅ since 136, expand on hover since 138 ([Mozilla](https://connect.mozilla.org/t5/discussions/sidebar-and-vertical-tabs-launch-in-release-136/m-p/89652)) | ✅ |
| Download size | **263 KB** | ~1 GB installed | ~1.8 GB installed | — | — |
| Account sync | ❌ | ✅ | ✅ | ✅ | ✅ |
| Extension store | ❌ (unpacked only) | ✅ Chrome Web Store | ✅ Edge Add-ons | ✅ AMO | ✅ AMO |
| Bookmarks, history UI, downloads UI | ❌ | ✅ | ✅ | ✅ | ✅ |
| Platforms | Windows | Windows, macOS, Linux | Windows, macOS, Linux | Windows, macOS, Linux | Windows, macOS, Linux |
| Open source | ✅ MIT / Apache-2.0 | Chromium only | ❌ | ✅ | ✅ |

**About the size row.** Whoz is small because it reuses WebView2, which Windows already installed (about 870 MB on the test machine). Whoz itself adds 535 KB. It does not use less memory per page than Edge; it runs the same engine.

### Who should use Whoz

- You want the most screen for the page and can live without a visible tab strip.
- You use the keyboard and want a browser with no menus to learn.
- You want a second, separate browser for one web app or one account.

### Who should stay with Chrome, Edge, Firefox or Zen

- You rely on account sync, an extension store, bookmarks or a download manager.
- You use macOS or Linux.
- You want a mature browser with security teams and years of hardening behind it. Whoz is a small shell; the engine underneath is updated by Microsoft with Windows, but the shell is a young project.

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+L`, `Alt+D`, `F6`, `Ctrl+Space` | Address bar and tab filter |
| `Ctrl+T` | New tab |
| `Ctrl+W`, `Ctrl+F4` | Close tab (closing the last tab quits) |
| `Ctrl+Tab`, `Ctrl+Shift+Tab`, `Ctrl+PgDn`, `Ctrl+PgUp` | Next / previous tab |
| `Ctrl+1` … `Ctrl+8`, `Ctrl+9` | Tab N / last tab |
| `Alt+←`, `Alt+→`, `F5`, `Ctrl+F`, `Ctrl+P`, `Ctrl +`/`-`/`0` | Back, forward, reload, find, print, zoom |
| `F11` | Full screen |
| `Alt+F4` | Quit |
| `Alt` + drag anywhere | Move the window |
| `Win+↑`, `Win+←`, `Win+→` | Maximize, snap |

In the address bar, text with a dot (`example.com`) opens as a site and anything else searches Google. Start with `?` to force a search (`? node.js`).

## Install

1. Download `whoz-browser-windows-x64.zip` from [Releases](https://github.com/afrizalyogi/whoz-browser/releases/latest) and unzip it.
2. Run `whoz.exe`. To add Whoz to the Start menu, run `install.ps1` (per user, no admin rights).

`whoz.exe https://example.com` opens that URL instead of the home page.

**Requirements:** Windows 10 or 11, x64, with the Microsoft Edge WebView2 Runtime. Windows 11 includes it; on Windows 10 get it from [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/) if Whoz reports it missing.

## Build from source

Needs [Rust](https://rustup.rs) with the MSVC toolchain.

```powershell
git clone https://github.com/afrizalyogi/whoz-browser
cd whoz-browser
cargo run --release      # build and run
cargo test               # URL parsing, escaping and shortcut tests
.\package.ps1            # dist\whoz-browser\ + zip
```

### How it works

```
tao window (frameless)
├── tab WebViews ........ one per tab, only the active one visible
└── ui.html WebView ..... header + tab list + address bar, transparent,
                          clipped with SetWindowRgn to the parts on screen
```

The UI overlay is a normal web page clipped to small rectangles. While it is hidden only a 4 px strip on the top and left edges receives the mouse, so clicks everywhere else go straight to the page. `ui.html` reports which rectangles are visible; `src/main.rs` applies them and handles tabs, windows and shortcuts. Shortcuts come from WebView2's `AcceleratorKeyPressed` event, so they work no matter which web page has focus.

| File | Lines | What |
|---|---|---|
| [`src/main.rs`](src/main.rs) | ~640 | Window, tabs, shortcuts, URL parsing, tests |
| [`src/ui.html`](src/ui.html) | ~150 | Header, tab list, address bar |

## Roadmap

Ideas, not promises. Open an issue to vote or discuss.

- Session restore (reopen tabs after a restart)
- Download progress indicator
- Choice of search engine and home page
- Drag to reorder tabs
- Keyboard shortcut to toggle the sidebar pinned open

## Contributing

Bug reports, ideas and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first; the short version is: keep it small, run `cargo test` and `cargo clippy --all-targets -- -D warnings`, and describe how you tested.

Security issues: please follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Made by [PT Aryatama Teknologi Digital](https://github.com/afrizalyogi). Built on [tao](https://github.com/tauri-apps/tao) and [wry](https://github.com/tauri-apps/wry) from the Tauri project.
