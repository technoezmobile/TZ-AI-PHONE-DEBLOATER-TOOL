# TZ AI DEBLOATER TOOL

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-teal.svg)](./LICENSE)
[![Release: v1.1.0](https://img.shields.io/badge/Release-v1.1.0-emerald.svg)](https://github.com/technoezmobile/TZ-AI-PHONE-DEBLOATER-TOOL/releases/tag/v1.1.0)
[![CI: Multi-Platform](https://img.shields.io/badge/CI-Multi--Platform-blue.svg)](./.github/workflows/ci.yml)
[![Design: Unslop](https://img.shields.io/badge/Design-Unslop-zinc.svg)](https://github.com/heydamir)

**TZ AI DEBLOATER TOOL** is an open-source, conservative Android maintenance workstation and package manager engineered by **TZ Labs**. It establishes direct communication with Android devices via USB debugging or Android 11+ wireless ADB, empowering power users, system administrators, and technicians to inspect, disable, uninstall, and recover packages.

It features both a fluid desktop graphical application and a headless standalone terminal CLI (`tz-debloater`), backed by a deterministic, action-aware safety policy, an atomic local audit journal, and multi-provider AI planning.

---

## Multi-Platform Verification Status Matrix

Every release package is evaluated across six rigorous operational dimensions. To maintain absolute transparency and integrity, packages verified on native hardware are clearly distinguished from pending remote CI builds:

| Platform & Target | Architecture | Build Status | Install Status | Launch Status | ADB Status | Signing / Notarization |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Windows 10 / 11** | `x86_64` | **Verified Native** | **Verified Native** (Setup, MSI, Portable) | **Verified Fluid** (100%, 125%, 150% DPI) | **Verified Hardware** (Poco X3 Pro & Mock ADB) | Unsigned Community Binary (SmartScreen documented) |
| **macOS Apple Silicon** | `aarch64` (M1–M4) | **Verified CI Runner** | **Verified Bundle** (DMG & Mach-O CLI) | **Verified Native** | **Supported** (Android platform-tools) | Unsigned (Gatekeeper `xattr -cr` documented) |
| **macOS Intel** | `x86_64` | **Verified CI Runner** | **Verified Bundle** (DMG & Mach-O CLI) | **Verified Native** | **Supported** (Android platform-tools) | Unsigned (Gatekeeper `xattr -cr` documented) |
| **Linux Ubuntu / Debian** | `x86_64` | **Verified CI Runner** | **Verified Package** (.deb & AppImage) | **Verified Native** | **Supported** (udev rules documented) | Unsigned `.deb` / AppImage (udev rules documented) |
| **Linux Standalone CLI** | `x86_64` | **Verified CI Runner** | **Verified Binary** (tar.gz archive) | **Verified Native** | **Supported** (POSIX compliant) | Static ELF binary (POSIX compliant) |

> [!NOTE]
> All release artifacts across Windows, macOS (Apple Silicon & Intel), and Linux were compiled from clean, audited release sources via GitHub Actions native runners and verified against authoritative SHA-256 checksums in [`SHA256SUMS.txt`](./release_artifacts/github-release/v1.1.0/SHA256SUMS.txt).

---

## Official Downloads (Release v1.1.0)

All 11 official release artifacts are cryptographically verified with SHA-256 checksums available in [`SHA256SUMS.txt`](./release_artifacts/github-release/v1.1.0/SHA256SUMS.txt):

### Windows Packages (Verified Native Binaries)

| File Name | Format | Target Platform | Status | Description |
| :--- | :--- | :--- | :--- | :--- |
| [`TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-Setup.exe) | NSIS Installer | Windows 10/11 x64 | **Verified** | **Recommended Setup**: Per-user installation, automatic desktop/Start shortcuts, uninstaller. |
| [`TZ-AI-Debloater-Tool-v1.1.0-x64.msi`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.msi) | WiX MSI | Windows 10/11 x64 | **Verified** | **Enterprise Deployment**: Silent automated rollouts via Group Policy (GPO), Intune, or SCCM. |
| [`TZ-AI-Debloater-Tool-v1.1.0-portable.exe`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-portable.exe) | Standalone EXE | Windows 10/11 x64 | **Verified** | **Zero-Install Portable**: Run directly from technician flash drives without installation. |
| [`tz-debloater-v1.1.0-windows-x64.zip`](./release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-windows-x64.zip) | Zip Archive | Windows 10/11 x64 | **Verified** | **Standalone CLI Binary**: Headless terminal binary (`tz-debloater.exe`) for scripts and automation. |

### macOS Packages (Verified Apple Silicon & Intel)

| File Name | Format | Target Platform | Status | Description |
| :--- | :--- | :--- | :--- | :--- |
| [`TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg) | Apple Disk Image | macOS Apple Silicon (M1–M4) | **Verified** | Drag-and-drop installer for Apple Silicon M1/M2/M3/M4 Macs. |
| [`tz-debloater-v1.1.0-macos-aarch64.tar.gz`](./release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-aarch64.tar.gz) | Tar Gzip Archive | macOS Apple Silicon | **Verified** | Standalone Mach-O CLI binary for Apple Silicon terminals. |
| [`TZ-AI-Debloater-Tool-v1.1.0-x64.dmg`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.dmg) | Apple Disk Image | macOS Intel x86_64 | **Verified** | Drag-and-drop installer for legacy Intel Macs. |
| [`tz-debloater-v1.1.0-macos-x64.tar.gz`](./release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-x64.tar.gz) | Tar Gzip Archive | macOS Intel x86_64 | **Verified** | Standalone Mach-O CLI binary for Intel Mac terminals. |

### Linux Packages (Verified x86_64 Packages & CLI)

| File Name | Format | Target Platform | Status | Description |
| :--- | :--- | :--- | :--- | :--- |
| [`TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage`](./release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage) | Linux AppImage | Linux x86_64 | **Verified** | Universal single-file executable for Ubuntu, Fedora, Arch, and Debian. |
| [`tz-ai-debloater_1.1.0_amd64.deb`](./release_artifacts/github-release/v1.1.0/tz-ai-debloater_1.1.0_amd64.deb) | Debian Package | Debian / Ubuntu x64 | **Verified** | Standard Debian package with system menu entry and desktop shortcuts. |
| [`tz-debloater-v1.1.0-linux-x64.tar.gz`](./release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-linux-x64.tar.gz) | Tar Gzip Archive | Linux x86_64 | **Verified** | Standalone ELF CLI binary for headless servers and automation. |

---

## Quickstart

### 1. Graphical Desktop Application
1. Download and run **`TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`**.
2. Connect your Android phone with **USB Debugging** enabled (see [User Guide](./docs/USER_GUIDE.md)).
3. Browse the default **Debloat Candidates** view in Package Explorer.
4. Select packages to modify, review the preflight safety check, and click **Apply**.

### 2. Standalone Terminal CLI (`tz-debloater`)
```bash
# 1. Inspect environment and connected devices
tz-debloater doctor

# 2. List connected devices and Android user profiles
tz-debloater devices
tz-debloater users

# 3. View recommended debloat candidates
tz-debloater candidates

# 4. Generate an action plan for telemetry trackers
tz-debloater plan --intent trackers

# 5. Apply modifications safely with automatic confirmation
tz-debloater apply --packages "com.facebook.katana" --action disable

# 6. Undo the last operation at any time
tz-debloater restore --latest -y
```

For complete CLI documentation, flags, and exit codes, see [`docs/CLI.md`](./docs/CLI.md).

---

## Key Capabilities & Safety Architecture

### 1. 3-Dimensional Package Classification
Unlike primitive debloaters that mix safety and category into a single badge, TZ AI DEBLOATER TOOL evaluates every package along three independent axes:
- **Android Package Type**: Runtime metadata from the Android OS (`System`, `UpdatedSystem`, `User`).
- **Community Catalog Rating**: Static consensus across 1,500+ analyzed packages (`Recommended`, `Advanced`, `Expert`, `Unsafe`, `Unlisted`).
- **Current Removal Eligibility**: Real-time policy decision (`Eligible`, `ReviewOnly`, `Protected`, `AlreadyRemoved`).

> [!IMPORTANT]
> A catalog rating of "Recommended" **never overrides** runtime device protections. If a package is acting as your active launcher or keyboard, removal is strictly blocked.

### 2. Four Distinct AI Assistant Shortcuts
The AI assistant includes 4 dedicated single-click scopes:
- **Review Bloatware**: Strictly inspects verified bloatware candidates with high-confidence evidence.
- **Review Trackers**: Focuses strictly on telemetry, analytics, and advertising SDKs.
- **Uninstall Promos**: Targets third-party partner and sponsor applications for per-user removal.
- **List Candidates**: **Read-only** view displaying all catalog debloat candidates without proposing actions.

### 3. Action-Aware Safety Policy
- **Dynamic Role Protection**: Automatically detects active home screen launchers (`cmd package resolve-activity`) and keyboards (`ime list -s`) to prevent bricking device navigation.
- **Selective Blocking**: Destructive actions (`Disable`, `UninstallUser`) are strictly blocked on essential services, while restorative actions (`Enable`, `Restore`) remain open.
- **No Accidental Disables**: Removal requests execute as `ActionType::UninstallUser` (`pm uninstall -k --user <uid>`), preventing unwanted fallback to disablement.

### 4. Cross-Process Advisory Locking
An atomic file-locking engine (`CrossProcessLock`) coordinates access between the desktop application and CLI sessions, preventing conflicting writes to devices, config files, or the audit journal. Stale locks from crashed processes are automatically reaped.

### 5. OS-Backed Cryptographic Keyrings
API keys for AI integrations (Gemini, OpenAI, Anthropic) are stored encrypted in native operating system keyrings:
- Windows: Microsoft Credential Manager via DPAPI.
- macOS: Apple Keychain Services.
- Linux: FreeDesktop Secret Service API (`secret-tool`) or restricted `0o600` file store.
- Keys are **never stored in plaintext `config.json`**.

---

## Documentation Index

Comprehensive guides are available in the [`docs/`](./docs/) directory:

- [**User Guide (`docs/USER_GUIDE.md`)**](./docs/USER_GUIDE.md): End-to-end walkthrough of USB/wireless ADB, multi-user profiles, batch actions, and settings.
- [**CLI Manual (`docs/CLI.md`)**](./docs/CLI.md): Complete reference for `tz-debloater` subcommands, flags, POSIX exit codes, and scripting examples.
- [**Installation Guide (`docs/INSTALL.md`)**](./docs/INSTALL.md): Instructions for Windows, macOS Gatekeeper clearance, Linux udev rules, and verification.
- [**Troubleshooting Guide (`docs/TROUBLESHOOTING.md`)**](./docs/TROUBLESHOOTING.md): Fixes for Xiaomi security restrictions, Knox MDM blocks, unauthorized devices, and emergency Safe Mode recovery.
- [**Maintainer Guide (`docs/MAINTAINER.md`)**](./docs/MAINTAINER.md): Developer toolchains, directory layout, test suites, and release packaging workflows.
- [**Privacy & Data Policy (`docs/PRIVACY.md`)**](./docs/PRIVACY.md): Zero-telemetry audit, credential vault mechanics, and AI data minimization controls.
- [**Publishing Guide (`PUBLISHING.md`)**](./PUBLISHING.md): Step-by-step instructions for repository owners to push releases and deploy website previews.
- [**Changelog (`CHANGELOG.md`)**](./CHANGELOG.md): Historical log of all releases and technical improvements.

---

## Upstream Lineage & Third-Party Credits

TZ AI DEBLOATER TOOL inspects and acknowledges the following open-source projects:

- **unslop Design System**:
  - Author: Damir Sokolovsky (`@heydamir`)
  - License: Creative Commons Attribution 4.0 International (CC BY 4.0)
- **FreeLLMAPI Gateway**:
  - Author: Tashfeen Ahmed (`https://github.com/tashfeenahmed/freellmapi`)
  - License: MIT License
- **Universal Android Debloater Next Generation (UAD-ng)**:
  - Repository: `https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation`
  - License: GNU General Public License v3.0 (GPL-3.0)
- **Canta**:
  - Repository: `https://github.com/samolego/Canta`
  - License: GNU Lesser General Public License v3.0 (LGPL-3.0)

For full licensing details, see [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md) and [`LICENSE`](./LICENSE).

---

## Safety Disclaimer

> [!WARNING]
> Non-root package operations operate at the per-user profile layer. They do NOT delete original system APKs from the read-only `/system` partition. While system packages can generally be re-enabled or re-installed (`cmd package install-existing`), disabling critical components can cause system instability, bootloops, or require a factory reset. Always maintain offline backups of critical data before modifying system packages.

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**.
