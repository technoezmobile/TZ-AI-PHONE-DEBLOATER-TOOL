# Maintainer & Developer Guide

This document details the build systems, directory architecture, test suites, and packaging workflows for maintainers and contributors working on **TZ AI DEBLOATER TOOL**.

---

## 1. Toolchain & Development Environment

To compile both the desktop application and the standalone terminal CLI from source, the following tools are required:

- **Rust**: Version 1.78 or newer (`rustup default stable`).
  - Windows: `x86_64-pc-windows-msvc` target (Visual Studio C++ Build Tools 2022).
  - macOS: Xcode Command Line Tools (`xcode-select --install`).
  - Linux: `build-essential`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`.
- **Node.js**: Version 20.x or 22.x LTS with `npm` 10+.
- **Tauri v2 CLI**: Installed via devDependencies (`npm run tauri`).
- **Windows Installers (Windows Builds Only)**:
  - **NSIS**: Version 3.x (used for `Setup.exe`).
  - **WiX Toolset**: Version 3.14 (used for `.msi` creation). Ensure `candle.exe` and `light.exe` are in your `PATH`.

---

## 2. Repository Layout

```
E:\AI DEBLOATER\
├── .github\
│   ├── ISSUE_TEMPLATE\          # GitHub issue templates (bug, feature)
│   ├── workflows\
│   │   ├── ci.yml               # Multi-platform CI (Windows, macOS, Linux)
│   │   └── release.yml          # Automated release builder and asset publisher
│   └── pull_request_template.md
├── docs\                        # Public documentation manuals
│   ├── USER_GUIDE.md            # Comprehensive user guide
│   ├── CLI.md                   # Command-line manual for tz-debloater
│   ├── INSTALL.md               # Multi-platform installation guide
│   ├── TROUBLESHOOTING.md       # Diagnostic and recovery manual
│   ├── MAINTAINER.md            # This file
│   └── PRIVACY.md               # Technical privacy & data handling audit
├── release_artifacts\           # Output directory for release bundles & checksums
├── site\                        # Static website & documentation preview
├── src\                         # Frontend React + TypeScript application
│   ├── components\              # Unslop UI components (PackageExplorer, Settings, etc.)
│   ├── App.tsx                  # Main fluid shell & responsive rail navigation
│   └── index.css                # Graphite design tokens, CSS variables, typography
├── src-tauri\                   # Rust backend
│   ├── Cargo.toml               # Library and binary targets definition
│   ├── tauri.conf.json          # Tauri v2 configuration (version, windows, bundle)
│   ├── src\
│   │   ├── bin\
│   │   │   ├── tz_debloater.rs  # Standalone CLI binary (zero-webview entrypoint)
│   │   │   └── mock_adb.rs      # High-fidelity mock ADB simulator for test suites
│   │   ├── lib.rs               # Core shared library & Tauri IPC commands
│   │   ├── main.rs              # Tauri GUI application entrypoint
│   │   ├── adb\                 # ADB discovery, execution, PM parsing, users
│   │   ├── ai\                  # Multi-protocol LLM adapters, streaming, routing
│   │   ├── dataset\             # Community catalog (1,500+ analyzed packages)
│   │   ├── execution\           # Atomic mutation boundary with lock enforcement
│   │   ├── journal\             # Snapshots, SQLite/JSON audit journal, recovery
│   │   ├── policy\              # Action-aware safety rules & role detectors
│   │   └── storage\             # Cross-platform keyring & concurrency locks
│   └── tests\                   # Integration test suites (mock ADB, credentials, etc.)
└── package.json                 # Node dependencies and scripts
```

---

## 3. Running Test Suites

All tests must pass cleanly before cutting any release or merging pull requests:

### 1. JavaScript Contract Tests
```bash
npm test
```
Verifies contract assertions and screenshot runners (`tests/*.test.mjs`).

### 2. Frontend Typecheck & Production Build
```bash
npm run build
```
Executes `tsc` (TypeScript compiler) followed by `vite build`. Ensures 0 type errors and generates production bundle in `dist/`.

### 3. Rust Unit and Integration Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```
Runs all 81+ automated tests covering:
- Mock ADB command execution and argument injection resistance.
- Action-aware safety policies (blocking launchers, keyboards, critical system services).
- Cross-process OS credential storage isolation.
- Cross-process advisory lock file creation and recovery.
- Multi-user parsing and XML/text stream parsing.

---

## 4. Release Build Commands

### 1. Build Standalone CLI (`tz-debloater`)
```bash
cargo build --manifest-path src-tauri/Cargo.toml --release --bin tz-debloater
```
The compiled binary is generated at `src-tauri/target/release/tz-debloater.exe` (or `tz-debloater` on Unix).

### 2. Build Tauri Desktop Bundles (NSIS, MSI, Portable)
```bash
npm run tauri -- build --ci --bundles nsis,msi
```
Output installers are generated in `src-tauri/target/release/bundle/`.

### 3. Generate Checksums
All release artifacts must be accompanied by an authoritative `SHA256SUMS.txt`:
```powershell
Get-FileHash -Algorithm SHA256 .\release_artifacts\github-release\v1.1.0\* |
  ForEach-Object { "$($_.Hash.ToLower())  $(Split-Path $_.Path -Leaf)" } |
  Out-File -Encoding utf8 .\release_artifacts\github-release\v1.1.0\SHA256SUMS.txt
```

---

## 5. Version Bumping Checklist

When bumping versions (e.g. `v1.1.0` -> `v1.2.0`):
1. `package.json`: `"version": "x.y.z"`
2. `src-tauri/tauri.conf.json`: `"version": "x.y.z"`
3. `src-tauri/Cargo.toml`: `version = "x.y.z"`
4. `src-tauri/src/bin/tz_debloater.rs`: `const VERSION: &str = "x.y.z";`
5. `src/App.tsx` and `src/components/AboutNotices.tsx`: In-app version labels.
6. `CHANGELOG.md`: New version header with dated changes.
7. `site/`: Headers and download links.

