# Whoz Browser: Chrome Extensions (Manifest V3 / V2) Specification

> Architecture and execution model for running Chrome Web Store extensions inside Whoz Browser with zero idle resource consumption.

---

## 1. Extension Architecture in Whoz Browser

In standard Google Chrome, each extension often spawns its own background process or keeps heavy Service Workers running in V8 memory. In Whoz Browser:

1. **Event-Driven Sleep:** Service Workers are instantiated in an ephemeral V8 context only when an incoming event occurs (`chrome.tabs.onUpdated`, `chrome.webRequest.onBeforeRequest`, `chrome.runtime.onMessage`). Once the promise resolves, the worker is paused/hibernated.
2. **Native Content Script Routing:** URL match parsing is done in compiled Rust (`whoz-extensions::content_scripts`) rather than executing JS pattern matchers in every frame.
3. **Storage Backing:** `chrome.storage.local` and `chrome.storage.sync` map directly to `whoz-sync`, allowing extension settings to be backed up via Google Sync automatically.

---

## 2. Manifest V3 Schema Support

```json
{
  "manifest_version": 3,
  "name": "Extension Name",
  "version": "1.0.0",
  "permissions": [
    "storage",
    "tabs",
    "declarativeNetRequest",
    "contextMenus"
  ],
  "host_permissions": [
    "https://*/*"
  ],
  "background": {
    "service_worker": "background.js",
    "type": "module"
  },
  "content_scripts": [
    {
      "matches": ["*://*.google.com/*"],
      "js": ["content.js"],
      "css": ["style.css"],
      "run_at": "document_idle"
    }
  ]
}
```

---

## 3. High-Priority Chrome APIs Supported

| API Namespace | Implementation in Whoz Browser |
|---|---|
| `chrome.runtime` | Handled by Rust IPC messaging router (`sendMessage`, `onMessage`, `getURL`) |
| `chrome.tabs` | Routed to `whoz-core::tab::TabManager` (`create`, `remove`, `query`, `update`) |
| `chrome.storage` | High-speed local LevelDB / SQLite storage with Google Sync forwarding |
| `chrome.declarativeNetRequest` | Rust regex-based adblocking & rule evaluator (uBlock Origin compatible) |
| `chrome.cookies` | Isolated cookie jar per browser profile |
| `chrome.webRequest` | Blocking & non-blocking network request interceptors |

---

## 4. Loading Extensions in Whoz

Users can install extensions:
1. **Unpacked Folder:** Place unpacked extension folder in `./dist/whoz-browser/extensions/<extension-name>`.
2. **Chrome Web Store (`.crx`):** Drag and drop `.crx` file onto the Whoz Browser window. The internal unpacker extracts zip contents, verifies signature against Google root keys, and registers the extension in `ExtensionManager`.
