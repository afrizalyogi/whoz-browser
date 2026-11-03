# Security policy

Whoz is a thin shell around the Microsoft Edge WebView2 runtime. Two kinds of issues:

- **Engine bugs** (rendering, JavaScript, sandbox escapes): these are in WebView2. Report them to Microsoft through the [Microsoft Security Response Center](https://msrc.microsoft.com/). WebView2 updates itself with Windows.
- **Whoz bugs** (for example a web page able to trigger Whoz commands, navigate tabs, or read another tab's data): report them privately through [GitHub Security Advisories](https://github.com/afrizalyogi/whoz-browser/security/advisories/new). Please do not open a public issue.

We aim to reply within 7 days and to ship a fix for confirmed issues as soon as we can.

## What Whoz does to limit risk

- Web pages can send exactly one message to Whoz (`drag`, to move the window). Every other command is accepted only from the built-in `ui.html` view.
- Text inserted into the UI (page titles, URLs) is escaped before it reaches JavaScript.
- `javascript:` typed in the address bar is searched, not executed.
