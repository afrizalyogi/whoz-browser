# Contributing to Whoz

Thanks for helping. Whoz stays useful by staying small, so the bar for new code is "does this make the browser better for most people without adding visible UI?"

## Before you start

- **Bug?** Open an issue with your Windows version, WebView2 version (`edge://version` in Edge shows a matching Chromium build), steps to reproduce, and what you expected.
- **Feature?** Open an issue first and describe the problem it solves. Features that add always-visible UI are unlikely to be accepted.
- **Small fix** (typo, obvious bug)? Send a pull request directly.

## Development

```powershell
cargo run                 # debug build, keeps a console for eprintln! output
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release     # the binary you ship
```

Code lives in two files:

- `src/main.rs`: window, tabs, shortcuts, URL parsing. Keep new logic small and add a unit test for anything that parses or decides (see the `tests` module).
- `src/ui.html`: header, tab list and address bar. Plain HTML/CSS/JS, no build step, no dependencies.

The overlay is clipped with `SetWindowRgn` to the rectangles `ui.html` reports in its `rgn` message. If you add a visible element, add its rectangle in `clip()`, or it will not show and will not receive clicks.

## Pull requests

- One change per pull request.
- `cargo test` and `cargo clippy --all-targets -- -D warnings` pass.
- Say how you tested it by hand. `scripts\manual-test.ps1` walks through every check and writes a report you can paste into the pull request.
- Screenshots or a short GIF for anything visual.

By contributing you agree your work is dual-licensed under MIT and Apache-2.0, like the rest of the project.
