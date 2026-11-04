<div align="center">

# Whoz

**A Chromium browser for Windows that keeps the screen for the page and uses less of your PC.**

[Whoz Lite (available now)](lite/README.md) · [Goals](#goals) · [Status](#status) · [How it is built](#how-it-is-built) · [Benchmarks](#benchmarks) · [Build](#build-from-source)

</div>

> **Status: early.** The Chromium edition builds in CI but has no release yet. If you want to use Whoz today, get [Whoz Lite](lite/README.md), the small WebView2 edition.

## Two editions

| | **Whoz** (this folder) | **Whoz Lite** ([`lite/`](lite/README.md)) |
|---|---|---|
| Engine | Its own Chromium build | Edge WebView2, shared with Windows |
| Download | Full browser (~100+ MB expected) | About 0.5 MB |
| Chrome Web Store extensions | Full Chrome extension API (goal) | Partial: no `chrome.tabs`, `chrome.windows`, side panels |
| Sync | Chromium sync protocol to a server you can host (goal) | Folder sync through your cloud drive |
| Memory per page | Tuned to use less than Chrome (goal, measured) | Same as Edge |
| State | In development | Usable, v0.3 |

## Goals

Whoz exists because a browser that uses the same RAM as Chrome is not worth switching to. Every goal is a number, checked by [`bench/bench.ps1`](bench/bench.ps1) before a release:

1. **Less memory than Chrome** with 1, 10 and 30 tabs open.
2. **Less idle CPU** with tabs open in the background.
3. **Same JavaScript speed as Chrome** on Speedometer 3.1. Whoz runs the same V8 engine, so the target is "no slower", not "faster".
4. **Full Chrome Web Store extensions.** Whoz is Chromium, so the whole `chrome.*` API is there.
5. **Sync** of bookmarks, history, open tabs, passwords and settings through the Chromium sync protocol. Google Sync itself is [closed to other Chromium browsers since March 2021](https://blog.chromium.org/2021/01/limiting-private-api-availability-in.html), so Whoz will sync to a server you or we host.
6. **The Whoz interface:** the page edge to edge, the toolbar hidden until you point at the top edge, vertical tabs by default.

Where memory savings come from: more aggressive tab discarding and freezing, fewer renderer processes, no Google background services (ungoogled-chromium already removes them), and build flags tuned for size and speed. Each change is kept only if the benchmark shows it helps.

What Whoz will not claim: to be faster than Chrome at running one page. Same engine, same speed. The difference is in what runs around the page.

## Status

| Step | State |
|---|---|
| 1. Benchmark harness ([`bench/`](bench/)) | Done: memory, idle CPU, startup, Speedometer 3.1, median of runs, pass/fail gate |
| 2. Baseline Chromium build in GitHub Actions | Set up; first build pending |
| 3. Resource patches, each one measured | Next |
| 4. Whoz interface (auto-hide toolbar, vertical tabs, branding) | Planned |
| 5. Sync server | Planned |
| 6. Releases with automatic updates | Planned |

Until step 4, builds still carry Helium's name and look.

## How it is built

Whoz starts from [Helium](https://github.com/imputnet/helium) (GPL-3.0), which builds on [ungoogled-chromium](https://github.com/ungoogled-software/ungoogled-chromium) (BSD-3-Clause). The Windows build scripts come from [helium-windows](https://github.com/imputnet/helium-windows).

```
Chromium source (tarball)
  + ungoogled-chromium patches   remove Google services and background traffic
  + Helium patches               privacy, uBlock Origin, UI fixes
  + patches/ in this repo        Windows build fixes, then Whoz changes
  = Whoz
```

| Path | What |
|---|---|
| [`helium-chromium/`](https://github.com/imputnet/helium) | Git submodule: Helium's patches and tools |
| [`patches/`](patches/) | Windows and Whoz patches, applied in the order of `patches/series` |
| [`flags.windows.gn`](flags.windows.gn) | Build flags |
| [`build.py`](build.py) | Downloads Chromium, applies patches, builds |
| [`package.py`](package.py) | Installer and portable zip |
| [`bench/`](bench/) | Benchmarks |
| [`lite/`](lite/) | Whoz Lite |

A full Chromium build takes many hours. The [Whoz (Chromium)](.github/workflows/chromium.yml) workflow runs it as a chain of up to 15 GitHub Actions jobs; each saves the build tree for the next one. It runs only when started by hand.

## Benchmarks

No published numbers yet. Numbers go here once each one is the median of at least three runs on a quiet machine, with the machine, versions and date listed.

Run them yourself:

```powershell
.\bench\bench.ps1 -Browsers Whoz,Chrome,Edge -Runs 3            # results in bench\results\
.\bench\bench.ps1 -Browsers Whoz,Chrome -Gate Chrome             # exit 1 if Whoz uses more memory or is >3% slower
```

The script starts every browser with a fresh temporary profile, never touches your own profiles, and sends no mouse or keyboard input.

## Build from source

You need about 100 GB free disk, 16 GB RAM or more (64 GB is better), Visual Studio 2026 with C++, the Windows 11 SDK, Python 3, Git and 7-Zip. On an 8-core PC a build takes many hours.

```powershell
git clone --recurse-submodules https://github.com/afrizalyogi/whoz-browser
cd whoz-browser
python build.py
python package.py
```

Or fork this repository and run the **Whoz (Chromium)** workflow in GitHub Actions, which is free for public repositories.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: [SECURITY.md](SECURITY.md).

## License

[GNU General Public License v3.0 or later](LICENSE). Parts from ungoogled-chromium are under the [BSD 3-Clause license](LICENSE.ungoogled_chromium); Chromium itself is under its own BSD-style license.

Made by [PT Aryatama Teknologi Digital](https://github.com/afrizalyogi).
