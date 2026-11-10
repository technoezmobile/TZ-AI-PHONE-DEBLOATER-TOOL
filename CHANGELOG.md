# Changelog

All notable changes to **TZ AI DEBLOATER TOOL** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.1.0] - 2026-11-10

### First Public GitHub Release

#### Added
- **Standalone Terminal Application (`tz-debloater`)**:
  - Full-featured CLI executable running without any GUI, webview, or Tauri dependency.
  - 11 dedicated commands: `doctor`, `devices`, `users`, `scan`, `candidates`, `plan`, `apply`, `history`, `restore`, `config`, `models`.
  - Machine-readable `--json` output and interactive terminal previews with confirmation prompt (bypassable with `-y`/`--yes`).
  - Strict, documented POSIX-style exit codes: `0` (Success), `1` (Runtime error), `2` (Usage error), `3` (User declined), `4` (Policy violation), `5` (Device offline/not found).
- **Cross-Platform Operating System Support**:
  - **ADB Discovery**: Automated resolution for macOS Homebrew (`/opt/homebrew/bin/adb`), MacPorts (`/usr/local/bin/adb`), Linux system paths (`/usr/bin/adb`, `/snap/bin/adb`), and Android SDK locations.
  - **Secure Credential Vaults**: Native encryption via Windows Credential Manager (DPAPI), Apple Keychain Services (`/usr/bin/security`), and FreeDesktop Secret Service API (`secret-tool`) with restricted POSIX file store fallback (`0o600` permissions).
  - **Headless Environment Fallbacks**: Support for `TZ_AI_API_KEY`, `GEMINI_API_KEY`, `OPENAI_API_KEY`, and `ANTHROPIC_API_KEY` environment variables.
- **Cross-Process Advisory Locking (`CrossProcessLock`)**:
  - File-based advisory locks synchronizing concurrent access to target devices, config files, and the audit journal between desktop and CLI sessions.
  - Automated stale lock reaping based on operating system process verification (`tasklist` on Windows, `kill -0` on Unix) and 300-second timeouts.
- **3-Dimensional Package Classification**:
  - Separates Android package type (`System`, `UpdatedSystem`, `User`), community catalog rating (`Recommended`, `Advanced`, `Expert`, `Unsafe`), and current removal eligibility (`Eligible`, `ReviewOnly`, `Protected`, `AlreadyRemoved`).
  - Catalog "Recommended" ratings can never override runtime protection rules.
- **Debloat Candidates Default View**:
  - Package Explorer defaults to high-confidence debloat candidates for the selected user profile, filtering out protected components and uninstalled apps.
- **Four Distinct AI Assistant Shortcuts**:
  - Dedicated intents: `Review Bloatware` (eligible candidates only), `Review Trackers` (catalog analytics/telemetry), `Uninstall Promos` (sponsor apps targeting per-user removal), and `List Candidates` (read-only full candidate list).
- **Continuous Integration & Automated Releases**:
  - Multi-platform CI workflows (`.github/workflows/ci.yml`) testing on Windows, macOS, and Linux runners.
  - Release automation (`.github/workflows/release.yml`) compiling NSIS, MSI, Portable, DMG, AppImage, and CLI bundles with SHA-256 checksum generation.
- **Comprehensive Public Documentation (`docs/`)**:
  - Added `USER_GUIDE.md`, `CLI.md`, `INSTALL.md`, `TROUBLESHOOTING.md`, `MAINTAINER.md`, `PRIVACY.md`, and `PUBLISHING.md`.

#### Changed
- Bumped project version to `1.1.0` across Rust `Cargo.toml`, Tauri configuration, Node `package.json`, and UI status footers.
- Cleaned root workspace by removing loose decompression byproducts (`classes.dex`) and updated `.gitignore` to ignore `_upstream/`, `*.apk`, `*.dex`, `classes.dex`, `target/`, and `.test-data/`.

---

## [1.0.1] - 2026-11-10

### Windows Release Candidate

#### Added
- Direct ADB Package Manager execution backend (`DirectAdbPm`) utilizing `pm disable-user`, `pm uninstall -k --user`, and `cmd package install-existing`.
- Action-aware safety policy dynamically detecting active home screen launcher (`cmd package resolve-activity`) and active keyboard (`ime list -s`) to prevent rendering devices unusable.
- Expandable diagnostics drawer displaying raw command arguments, execution backend, exit codes, and stdout/stderr.
- Local audit journal with pre-operation JSON snapshots and rollback capabilities.
- 81 automated tests verifying mock ADB execution, process concurrency, credential vault isolation, and streaming cancellation.

#### Fixed
- Fixed Windows display scaling and high-DPI resize defects where windows snapped or maximized at 125% and 150% scaling left half-screen empty space.
- Replaced fixed sidebar reservations with fluid CSS flex layouts and responsive navigation rail.
- Fixed chat composer overflow and clipping on narrow window dimensions.
- Corrected package state handling to prevent requested uninstalls from falling back to disable actions.

