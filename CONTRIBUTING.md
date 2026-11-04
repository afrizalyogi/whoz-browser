# Contributing to Whoz

Thanks for helping. Whoz stays useful by staying small, so the bar for new code is "does this make the browser better for most people without adding visible UI?"

## Before you start

- **Bug?** Open an issue with your Windows version, WebView2 version (`edge://version` in Edge shows a matching Chromium build), steps to reproduce, and what you expected.
- **Feature?** Open an issue first and describe the problem it solves. Features that add always-visible UI are unlikely to be accepted.
- **Small fix** (typo, obvious bug)? Send a pull request directly.

## Two editions

- **Whoz** (repository root): a Chromium build. Changes are patches in `patches/`, listed in `patches/series`. Build with `python build.py` (see the README for requirements) or run the **Whoz (Chromium)** workflow on your fork. Any change meant to save memory or CPU needs before/after numbers from `bench\bench.ps1` in the pull request.
- **Whoz Lite** (`lite/`): the WebView2 edition in Rust. The rest of this guide covers it; run the commands below inside `lite/`.

## Development (Whoz Lite)

```powershell
cargo run                 # debug build, keeps a console for eprintln! output
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release     # the binary you ship
```

Code lives in a few files:

- `src/main.rs`: window, tabs, toolbar commands, extensions, downloads, the `whoz://` page API, shortcuts and URL parsing.
- `src/store.rs`: settings, bookmarks, history, downloads and folder sync (plain JSON files).
- `src/ext.rs`: Chrome Web Store download, CRX unpacking, manifest reading.
- `src/win.rs`: small Win32 helpers.
- `src/ui.html`: toolbar, tabs, address bar, menus and popovers. Plain HTML/CSS/JS, no build step.
- `src/pages.html`: `whoz://settings`, `history`, `bookmarks`, `downloads`, `extensions`.

Add a unit test for anything that parses, merges or decides (see the `tests` modules).

The overlay is clipped with `SetWindowRgn` to the shapes `ui.html` reports in its `rgn` message. A new floating element needs the `pop` class (or its own entry in `clip()`), or it will not show and will not receive clicks.

## Releases

Whoz Lite: CI runs clippy, tests and `package.ps1` on every change to `lite/`. To publish a release, bump `version` in `lite/Cargo.toml` and merge to `main`: CI creates the `lite-v<version>` tag and a GitHub release with the zip, `SHA256SUMS.txt` and generated notes. A push that keeps the same version publishes nothing.

Whoz: the **Whoz (Chromium)** workflow is started by hand and takes many hours. With *Publish a pre-release* ticked it publishes `v<version>` as an unsigned pre-release.

## Pull requests

- One change per pull request.
- `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
- Say how you tested it by hand. `scripts\manual-test.ps1` walks through every check and writes a report you can paste into the pull request.
- Screenshots or a short GIF for anything visual.

By contributing you agree your work is licensed under the GNU General Public License v3.0 or later, like the rest of the project.
