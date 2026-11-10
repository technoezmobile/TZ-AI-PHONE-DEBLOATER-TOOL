# TZ AI DEBLOATER TOOL (v1.0.1)

Windows desktop maintenance console and package manager for Android devices connected through USB or wireless ADB. Engineered for power users, developers, and technicians. AI assistance is optional; every proposed action plan undergoes deterministic hardware safety policy validation and explicit user review before execution.

See [`RELEASE-REPORT.md`](file:///E:/AI%20DEBLOATER/RELEASE-REPORT.md) for full change logs, test evidence, known limits, and release hashes. Upstream open-source notices are in [`THIRD_PARTY_NOTICES.md`](file:///E:/AI%20DEBLOATER/THIRD_PARTY_NOTICES.md) and [`LICENSE`](file:///E:/AI%20DEBLOATER/LICENSE).

---

## 1. System Requirements

- **Operating System:** Windows 10 64-bit (1809+) or Windows 11 (x86_64 architecture natively supported).
- **Runtime:** Microsoft Edge WebView2 Runtime (pre-installed on Windows 10/11).
- **Target Devices:** Android 7.0 (Nougat) up to Android 15+.
- **Bridge:** USB Debugging or Android 11+ Wireless ADB (Wi-Fi 802.11 b/g/n/ac).
- **Memory & Disk:** 150 MB RAM · 60 MB free disk space.

---

## 2. Quick Start & Setup

1. **Enable Developer Options on Android:**
   - Go to **Settings → About phone** and tap **Build number** (or **OS version**) 7 times.
   - In **Settings → Additional Settings / System → Developer options**, enable **USB debugging**.
   - *Xiaomi / POCO devices:* Also enable **USB debugging (Security settings)** if permission modification prompts appear.
2. **Connect Device:**
   - Plug the phone into your PC with a data-capable USB cable.
   - Accept the RSA key fingerprint dialog on your phone screen (*"Always allow from this computer"*).
3. **Launch the Console:**
   - Launch `TZ-AI-Debloater-Tool.exe` or open the installed application.
   - Connected targets and their user profiles (Owner 0, Work Profile 10, Dual Apps 999) appear automatically.

---

## 3. Core Architecture & Safety Rules

### Separated Classification Dimensions
Package management is governed by three independent attributes:
1. **Android Package Type:** `System` (in `/system/`), `UpdatedSystem` (updated system app in `/data/`), or `User` (third-party app).
2. **Catalog Rating:** Curated community debloat recommendation (`Recommended`, `Advanced`, `Expert`, `Unsafe`, `Unlisted`).
3. **Current Removal Eligibility:** Enforced by the backend safety engine (`Eligible`, `ReviewOnly`, `Protected`, `AlreadyRemoved`).
> **Absolute Safety Rule:** A catalog rating **never overrides device protection rules**. Confirmed hardware services remain permanently `Protected`.

### Hardware & Vendor Safeguards
- **MediaTek Vendor Services:** Critical telephony and modem daemons (`com.mediatek.telephony`, `com.mediatek.ims`, `com.mediatek.wfo.impl`) are locked as `Protected`.
- **Active Home Launcher & Keyboard:** Dynamically detected and protected against removal.
- **Built-in ROM Utilities:** MIUI Compass, Notes, Weather, Calculator, and Screen Recorder are marked `Review-Only` and excluded from automated AI debloat plans.

### Rollback Recovery Journal
Every package state mutation is recorded in a local SQLite database (`%APPDATA%\com.tzlabs.debloater\journal.db`). Restore uninstalled packages with one click using `cmd package install-existing --user <id> <package>`.

---

## 4. Keyboard Shortcuts Guide

| Shortcut | Description |
| :--- | :--- |
| `Ctrl + K` | Open Global Command Palette |
| `Ctrl + 1` | Navigate to Connected Devices Console |
| `Ctrl + 2` | Navigate to Package Explorer |
| `Ctrl + 3` | Navigate to AI Assistant |
| `Ctrl + 4` | Navigate to Action Plan Review |
| `Ctrl + 5` | Navigate to Journal & Recovery |
| `Ctrl + 6` | Navigate to Settings |
| `Ctrl + 7` | Navigate to About & Notices |
| `Ctrl + R` / `F5` | Refresh device package inventory |
| `Ctrl + /` or `?` | Open Keyboard Shortcuts Guide Modal |
| `Escape` | Dismiss open dialogs, drawers, or command palette |
| `Enter` | Send AI prompt in composer / Confirm action |
| `Shift + Enter` | Insert newline in AI prompt composer |

---

## 5. Build from Source

Requires Windows x64, Node.js (v18+), Rust (1.78+), Visual Studio C++ Build Tools, and WebView2.

```powershell
# 1. Install dependencies & verify frontend
npm ci
npm test
npm run build

# 2. Run backend test suite (81 tests)
$env:TZ_DEBLOATER_TEST_DATA = Join-Path (Get-Location) '.test-data'
cargo test --manifest-path src-tauri/Cargo.toml

# 3. Build release installers (NSIS & MSI)
npm run tauri -- build --ci --bundles nsis,msi
```

---

## 6. Static Documentation & Public Site

A static site implementing all 20 web checks (pure semantic HTML without required JS, unique titles and descriptions, canonical tags, JSON-LD, sitemap, robots.txt) is provided in `site/`:
- `site/index.html` — Homepage
- `site/download/index.html` — Release downloads & SHA-256 verification
- `site/docs/index.html` — Comprehensive documentation
- `site/changelog/index.html` — Full v1.0.1 changelog
- `site/privacy/index.html` — Privacy policy & zero-telemetry guarantee
- `site/refer/index.html` — Clean share & badge resources
- `site/404.html` — Custom 404 error page

---

## 7. Verified Support & License

- **Organization:** [TZ Labs](https://tzlabs.in)
- **Official Website:** [`https://tzlabs.in`](https://tzlabs.in)
- **License:** GNU General Public License v3.0 ([GPL-3.0](file:///E:/AI%20DEBLOATER/LICENSE))
