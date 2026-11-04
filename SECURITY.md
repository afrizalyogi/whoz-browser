# Security policy

This policy covers both editions. **Whoz** is a Chromium build: engine bugs that also affect Chrome go to the [Chromium security team](https://www.chromium.org/Home/chromium-security/reporting-security-bugs/); bugs in our patches come to us (below). **Whoz Lite** is a thin shell around the Microsoft Edge WebView2 runtime. For Whoz Lite there are two kinds of issues:

- **Engine bugs** (rendering, JavaScript, sandbox escapes): these are in WebView2. Report them to Microsoft through the [Microsoft Security Response Center](https://msrc.microsoft.com/). WebView2 updates itself with Windows.
- **Whoz bugs** (for example a web page able to trigger Whoz commands, navigate tabs, or read another tab's data): report them privately through [GitHub Security Advisories](https://github.com/afrizalyogi/whoz-browser/security/advisories/new). Please do not open a public issue.

We aim to reply within 7 days and to ship a fix for confirmed issues as soon as we can.

## What Whoz Lite does to limit risk

- Web pages can send exactly one message to Whoz (`drag`, to move the window). Toolbar commands are accepted only from the built-in `ui.html` view.
- `whoz://` pages (settings, history, bookmarks, downloads, extensions) talk to Whoz through `https://whoz.localhost/api/`. Requests need an `x-whoz` header, which forces a CORS preflight that Whoz never approves, and a matching `Origin`, so other sites cannot call it. Whoz pages refuse to be framed.
- Text inserted into the UI (page titles, URLs) is escaped before it reaches JavaScript or HTML.
- `javascript:` typed in the address bar is searched, not executed.
- Extensions are downloaded only from Google's Chrome Web Store update server over HTTPS, or loaded from a folder you pick. Extensions you install can read and change the pages they ask for, as in any browser; install only ones you trust.
- Sync writes `whoz-sync.json` (bookmarks, history, settings) unencrypted to the folder you choose. Pick a folder only you can read. Passwords and cookies are never synced.