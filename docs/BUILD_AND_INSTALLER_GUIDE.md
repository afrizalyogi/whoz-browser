# Whoz Browser: Build, Packaging & Installer Guide

> Complete instructions for compiling Whoz Browser from source, running tests, assembling portable releases, and creating Windows installers.

---

## 1. Prerequisites

- **Operating System:** Windows 10 / Windows 11 (x64 / ARM64)
- **Rust Toolchain:** Rust 1.80+ (recommended: `stable-x86_64-pc-windows-msvc`)
- **Visual Studio C++ Build Tools:** VS 2022 or higher with MSVC & Windows SDK installed
- **Chromium Base:** Official Chrome or Chromium Content Shell available on machine

---

## 2. Compiling From Source

Clone or navigate to the project directory:

```powershell
cd D:\projects\pt-aryatama-teknologi-digital\afrizalyogi\whoz-browser
```

### Development Build & Run
```powershell
cargo run --bin whoz
```

### Optimized Production Release Build
```powershell
cargo build --release --bin whoz
```

The compiled executable will be generated at:
```
target\release\whoz.exe (~1.5 MB)
```

---

## 3. Automated Packaging (Portable Release)

Run the included automated packaging script:

```powershell
.\package.ps1
```

This script performs:
1. Production release compilation via Cargo.
2. Assembly of `dist\whoz-browser\` directory containing:
   - `whoz.exe` (Main browser executable)
   - `whoz-browser.exe` (Convenience alias)
   - `config.json` (Runtime memory & optimization tuning)
   - `whoz.VisualElementsManifest.xml` (Windows Start Menu integration)
   - `launch.bat` (Direct portable launcher)
   - `data\` & `extensions\` directories.
3. Creation of Windows installer script `dist\install-whoz-browser.ps1`.

---

## 4. Running the Windows Installer

To install Whoz Browser for the current user:

```powershell
powershell -ExecutionPolicy Bypass -File .\dist\install-whoz-browser.ps1
```

This will:
- Install Whoz Browser into `%LOCALAPPDATA%\Programs\WhozBrowser`
- Create a Desktop shortcut (`Whoz Browser.lnk`)
- Register Start Menu tile with full visual branding
- Enable portable user profile storage.

---

## 5. Direct Execution (Portable Mode)

You can run Whoz Browser immediately without installation:

```powershell
.\dist\whoz-browser\whoz.exe
# OR
.\dist\whoz-browser\whoz-browser.exe
# OR
.\dist\whoz-browser\launch.bat
```
