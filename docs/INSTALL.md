# Installation Guide

**TZ AI DEBLOATER TOOL** is available across Windows, macOS, and Linux as both a rich desktop workstation and a standalone terminal CLI.

---

## Operating System Support & Verification Matrix

Every release package is evaluated across six operational criteria. Windows packages are verified on native hardware; macOS and Linux packages are generated via GitHub Actions native runners:

| Platform & Target | Architecture | Build Status | Install Status | Launch Status | ADB Status | Signing / Notarization |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Windows 10 / 11** | `x86_64` | **Verified Native** | **Verified Native** (Setup, MSI, Portable) | **Verified Fluid** (100%, 125%, 150% DPI) | **Verified Hardware** (Poco X3 Pro & Mock ADB) | Unsigned Community Binary (SmartScreen bypass documented) |
| **macOS Apple Silicon** | `aarch64` (M1–M4) | **Verified CI Runner** | **Verified Bundle** (DMG & Mach-O CLI) | **Verified Native** | **Supported** (Android platform-tools) | Unsigned (Gatekeeper `xattr -cr` documented) |
| **macOS Intel** | `x86_64` | **Verified CI Runner** | **Verified Bundle** (DMG & Mach-O CLI) | **Verified Native** | **Supported** (Android platform-tools) | Unsigned (Gatekeeper `xattr -cr` documented) |
| **Linux Ubuntu / Debian** | `x86_64` | **Verified CI Runner** | **Verified Package** (.deb & AppImage) | **Verified Native** | **Supported** (udev rules documented) | Unsigned `.deb` / AppImage (udev rules documented) |
| **Linux Standalone CLI** | `x86_64` | **Verified CI Runner** | **Verified Binary** (tar.gz archive) | **Verified Native** | **Supported** (POSIX compliant) | Static ELF binary (POSIX compliant) |

---

## Release Artifacts Inventory (v1.1.0)

All 11 official release packages are compiled, verified, and cataloged in [`release_artifacts/github-release/v1.1.0/`](../release_artifacts/github-release/v1.1.0/):

### Windows Packages (Verified Native Binaries)
- [`TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-Setup.exe) (NSIS Desktop Setup)
- [`TZ-AI-Debloater-Tool-v1.1.0-x64.msi`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.msi) (Enterprise WiX MSI)
- [`TZ-AI-Debloater-Tool-v1.1.0-portable.exe`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-portable.exe) (Zero-Install Standalone Executable)
- [`tz-debloater-v1.1.0-windows-x64.zip`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-windows-x64.zip) (Standalone CLI Archive)

### macOS Packages (Verified Apple Silicon & Intel)
- [`TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg) (Apple Silicon DMG)
- [`tz-debloater-v1.1.0-macos-aarch64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-aarch64.tar.gz) (Apple Silicon CLI)
- [`TZ-AI-Debloater-Tool-v1.1.0-x64.dmg`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.dmg) (Intel Mac DMG)
- [`tz-debloater-v1.1.0-macos-x64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-x64.tar.gz) (Intel Mac CLI)

### Linux Packages (Verified x86_64 Packages & CLI)
- [`TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage) (Linux x86_64 AppImage)
- [`tz-ai-debloater_1.1.0_amd64.deb`](../release_artifacts/github-release/v1.1.0/tz-ai-debloater_1.1.0_amd64.deb) (Debian / Ubuntu Package)
- [`tz-debloater-v1.1.0-linux-x64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-linux-x64.tar.gz) (Linux x86_64 CLI)

---

## Minimum System Requirements

- **Processor**: 1 GHz dual-core 64-bit CPU (x86_64 or Apple Silicon).
- **Memory (RAM)**: 4 GB minimum (8 GB recommended).
- **Disk Space**: 250 MB free space for application, Android platform-tools, and safety journals.
- **Display**: Minimum window resolution of 760 × 540 (fluid scaling up to 4K+ at 100%, 125%, 150%, 200% DPI).
- **Connectivity**: USB port (Type-A or Type-C) with data transfer cable, or local 2.4/5GHz Wi-Fi for Android 11+ wireless debugging.

---

## Windows Installation

### 1. NSIS Desktop Setup (`.exe`) — Recommended
1. Download **`TZ-AI-Debloater-Tool-v1.1.0-Setup.exe`** from [GitHub Releases](https://github.com/tz-labs/tz-ai-debloater-tool/releases).
2. Double-click the installer.
   > **Note on Windows SmartScreen**: As an open-source, unsigned community binary, Windows SmartScreen may present a *"Windows protected your PC"* notification. Click **More info** and select **Run anyway**.
3. Complete the installation wizard. Shortcuts are created in the Start Menu and Desktop.

### 2. Enterprise WiX MSI (`.msi`)
For system administrators managing enterprise environments or deploying via Group Policy (GPO), Microsoft Intune, or SCCM:
```cmd
msiexec /i TZ-AI-Debloater-Tool-v1.1.0-x64.msi /quiet /qn /norestart
```

### 3. Portable Standalone (`.exe`)
For field technicians running from a USB flash drive without administrative rights:
1. Download **`TZ-AI-Debloater-Tool-v1.1.0-portable.exe`**.
2. Run directly from any folder or removable storage. All runtime dependencies are bundled.

### 4. Standalone Terminal CLI (`tz-debloater`)
1. Download **`tz-debloater-v1.1.0-windows-x64.zip`**.
2. Extract `tz-debloater.exe` into a folder included in your system `PATH` (such as `C:\Windows\System32` or `%LOCALAPPDATA%\Programs\bin`).
3. Open PowerShell or Command Prompt and run `tz-debloater doctor`.

---

## macOS Installation

### 1. Desktop App (Apple Silicon & Intel)
1. Download the corresponding DMG for your architecture:
   - Apple Silicon (M1/M2/M3/M4): [`TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-aarch64.dmg)
   - Intel Macs: [`TZ-AI-Debloater-Tool-v1.1.0-x64.dmg`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.dmg)
2. Open the DMG file and drag **TZ AI DEBLOATER TOOL** into your **Applications** folder.
3. Because the release is open-source and unsigned with an Apple Developer certificate, macOS Gatekeeper may display a warning on first launch. To permit launch:
   - Right-click the app in `/Applications` and select **Open**, then click **Open** in the dialog.
   - Alternatively, open Terminal and clear the quarantine attribute:
     ```bash
     xattr -cr "/Applications/TZ AI DEBLOATER TOOL.app"
     ```

### 2. Standalone Terminal CLI
1. Download the CLI tarball:
   - Apple Silicon: [`tz-debloater-v1.1.0-macos-aarch64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-aarch64.tar.gz)
   - Intel Mac: [`tz-debloater-v1.1.0-macos-x64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-macos-x64.tar.gz)
2. Extract and copy to your system path:
   ```bash
   tar -xzvf tz-debloater-v1.1.0-macos-*.tar.gz
   sudo cp tz-debloater /usr/local/bin/tz-debloater
   sudo chmod +x /usr/local/bin/tz-debloater
   tz-debloater doctor
   ```

---

## Linux Installation

### 1. AppImage
The AppImage runs on any modern Linux distribution without installation:
1. Download [`TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage`](../release_artifacts/github-release/v1.1.0/TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage).
2. Make it executable and run:
   ```bash
   chmod +x TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage
   ./TZ-AI-Debloater-Tool-v1.1.0-x64.AppImage
   ```

### 2. Debian / Ubuntu Package (`.deb`)
1. Download [`tz-ai-debloater_1.1.0_amd64.deb`](../release_artifacts/github-release/v1.1.0/tz-ai-debloater_1.1.0_amd64.deb).
2. Install via dpkg:
   ```bash
   sudo dpkg -i tz-ai-debloater_1.1.0_amd64.deb
   sudo apt-get install -f  # Resolves any missing webkit2gtk dependencies if required
   ```

### 3. Standalone Terminal CLI
1. Download [`tz-debloater-v1.1.0-linux-x64.tar.gz`](../release_artifacts/github-release/v1.1.0/tz-debloater-v1.1.0-linux-x64.tar.gz).
2. Extract and copy to `/usr/local/bin`:
   ```bash
   tar -xzvf tz-debloater-v1.1.0-linux-x64.tar.gz
   sudo cp tz-debloater /usr/local/bin/tz-debloater
   sudo chmod +x /usr/local/bin/tz-debloater
   tz-debloater doctor
   ```

### 3. Android Udev Rules (Non-Root ADB Access)
On Linux systems, non-root users require udev permissions to communicate with USB Android devices:
```bash
# Debian / Ubuntu
sudo apt install android-sdk-platform-tools-common

# Arch Linux
sudo pacman -S android-udev

# Manual rule configuration:
sudo curl -s https://raw.githubusercontent.com/M0Rf30/android-udev-rules/main/51-android.rules -o /etc/udev/rules.d/51-android.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

---

## Verification & Hash Integrity

Before executing any downloaded binaries, compare the computed SHA-256 hash against the official values published in `SHA256SUMS.txt`:

### Windows (PowerShell)
```powershell
Get-FileHash -Algorithm SHA256 .\TZ-AI-Debloater-Tool-v1.1.0-Setup.exe
```

### macOS / Linux
```bash
shasum -a 256 TZ-AI-Debloater-v1.1.0*
```

