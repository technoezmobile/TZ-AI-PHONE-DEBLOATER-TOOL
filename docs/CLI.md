# TZ Debloater CLI Manual (`tz-debloater`)

`tz-debloater` is a high-performance, standalone terminal application compiled from the TZ AI DEBLOATER TOOL core engine. It executes in any console environment (PowerShell, Command Prompt, bash, zsh, SSH, or CI/CD pipelines) without requiring a graphical session or running Tauri desktop instance.

---

## Table of Contents
1. [Installation & Binary Location](#1-installation--binary-location)
2. [Documented Exit Codes](#2-documented-exit-codes)
3. [Global Options](#3-global-options)
4. [Command Reference](#4-command-reference)
   - [doctor](#doctor)
   - [devices](#devices)
   - [users](#users)
   - [scan / packages](#scan--packages)
   - [candidates](#candidates)
   - [plan](#plan)
   - [apply](#apply)
   - [history](#history)
   - [restore](#restore)
   - [config](#config)
   - [models](#models)
5. [Environment Variables](#5-environment-variables)
6. [Automation & Scripting Examples](#6-automation--scripting-examples)

---

## 1. Installation & Binary Location

- **Windows**: Included in release zip `tz-debloater-v1.1.0-windows-x64.zip` and placed at `%LOCALAPPDATA%\Programs\TZ_AI_Debloater\tz-debloater.exe` when installed via Setup.
- **macOS**: Included in macOS release tarball or installed in `/usr/local/bin/tz-debloater`.
- **Linux**: Standalone ELF executable placed in `/usr/local/bin/tz-debloater`.

To verify your installation:
```bash
tz-debloater --version
```
Output:
```
tz-debloater v1.1.0 (TZ Labs - Intelligent Android Maintenance)
```

---

## 2. Documented Exit Codes

`tz-debloater` adheres to strict POSIX-style exit codes for seamless scripting and error checking:

| Exit Code | Meaning | Description |
| :---: | :--- | :--- |
| **`0`** | **Success** | The requested command completed successfully. |
| **`1`** | **Runtime Error** | Execution failed on device or during package mutation (e.g. ADB error, IO failure). |
| **`2`** | **Usage Error** | Invalid command-line arguments, unknown flags, or missing required values. |
| **`3`** | **Cancelled** | Operation aborted because the user declined the interactive confirmation prompt. |
| **`4`** | **Policy Violation** | Operation blocked by SafetyPolicy (e.g. attempting to uninstall active launcher, keyboard, or critical system service). |
| **`5`** | **Device Offline** | No ready Android device found, specified serial not detected, or ADB executable unavailable. |

---

## 3. Global Options

These flags apply across all subcommands:

- `-d, --device <serial>`: Target a specific device by its ADB serial number. If omitted and exactly one ready device is connected, that device is automatically selected.
- `-u, --user <id>`: Target a specific Android user/profile ID (default: `0`).
- `-y, --yes`: Skip interactive confirmation prompts and proceed automatically.
- `--json`: Format all output as machine-readable JSON on stdout.
- `-v, --version`: Print version information and exit.
- `-h, --help`: Print comprehensive help information and exit.

---

## 4. Command Reference

### `doctor`
Inspects your operating system environment, ADB discovery, native keyring status, local configuration, and connected devices.

```bash
# Pretty-printed diagnostics
tz-debloater doctor

# Machine-readable JSON output
tz-debloater doctor --json
```

**Example Output:**
```
TZ AI DEBLOATER TOOL - System Doctor (v1.1.0)
==================================================
Host OS:             windows (x86_64)
Keyring Store:       Windows Credential Manager (Native DPAPI)
AI Data Consent:     Granted
Active AI Profile:   preset-gemini
Approved Endpoints:  3 host(s)
Audit Journal:       105 entries recorded
Safety Snapshots:    108 saved
--------------------------------------------------
ADB Status:          Available
ADB Path:            C:\Users\user\AppData\Local\TZ_AI_Debloater\platform-tools\adb.exe
ADB Version:         Android Debug Bridge version 1.0.41
ADB Detection:       Application Managed Platform Tools
--------------------------------------------------
Connected Devices:   1 total (1 ready)
  - 28141FDH20088Z   Xiaomi       23049PCD8G      Android 14   [device]
==================================================
```

---

### `devices`
Lists all attached Android devices, model names, Android versions, and connection states.

```bash
tz-debloater devices
tz-debloater devices --json
```

---

### `users`
Lists all Android user profiles available on the target device.

```bash
tz-debloater users
tz-debloater users -d <serial>
```

---

### `scan` / `packages`
Scans all packages installed on the selected device and user profile with flexible filtering.

```bash
# Default view: shows debloat candidates for User 0
tz-debloater scan

# View all packages
tz-debloater scan --view all

# View system and vendor packages
tz-debloater scan --view system

# View packages uninstalled for this user
tz-debloater scan --view removed

# Search by keyword
tz-debloater scan --search "facebook"

# Filter by rating, type, or state
tz-debloater scan --rating recommended --type system --state enabled

# Target secondary user profile (e.g. Work Profile 10)
tz-debloater scan -u 10
```

---

### `candidates`
Shorthand for `tz-debloater scan --view candidates`. Shows only high-confidence debloat candidates currently installed for the target profile.

```bash
tz-debloater candidates
tz-debloater candidates --json
```

---

### `plan`
Generates a structured action plan. Can be heuristic-driven (catalog-based) or AI-assisted.

```bash
# Plan by shortcut intent
tz-debloater plan --intent bloatware
tz-debloater plan --intent trackers
tz-debloater plan --intent promos

# Plan for explicit packages
tz-debloater plan --packages "com.facebook.katana,com.facebook.system" --action disable

# Output plan as JSON
tz-debloater plan --intent trackers --json > trackers-plan.json
```

---

### `apply`
Applies package state mutations (disable, uninstall, enable, restore) to the connected device.

```bash
# Apply action to comma-separated packages (prompts for confirmation)
tz-debloater apply --packages "com.facebook.katana" --action disable

# Apply without prompt (for scripts / CI)
tz-debloater apply --packages "com.facebook.katana" --action disable -y

# Apply an existing plan JSON file
tz-debloater plan --intent promos --json > promos.json
tz-debloater apply --plan promos.json -y

# Attempting to remove protected components results in exit code 4:
tz-debloater apply --packages "com.android.launcher" --action disable -y
# Output: [POLICY VIOLATION] Action 'Disable' on package 'com.android.launcher' is prohibited...
# ExitCode: 4
```

---

### `history`
Reads the local SQLite/JSON audit journal, displaying past package operations and outcomes.

```bash
# Show last 10 entries
tz-debloater history --limit 10

# Filter by device
tz-debloater history -d <serial>
```

---

### `restore`
Restores previously modified packages to their prior verified state.

```bash
# Restore specific packages
tz-debloater restore --packages "com.facebook.katana" -y

# Undo the most recent operation recorded in the journal
tz-debloater restore --latest -y
```

---

### `config`
Inspects and modifies configuration profiles, credentials, and approved endpoints.

```bash
# View configuration
tz-debloater config list

# Set active AI connection profile
tz-debloater config set-active preset-gemini

# Store API key in native OS keyring
tz-debloater config set-key preset-gemini "AIzaSy..."

# Remove API key from keyring
tz-debloater config remove-key preset-gemini

# Approve an external endpoint destination
tz-debloater config approve-host "https://custom-gateway.internal"
```

---

### `models`
Discovers supported models for the active connection profile.

```bash
tz-debloater models
tz-debloater models --json
```

---

## 5. Environment Variables

In automated environments or headless servers where a desktop keyring daemon is unavailable, `tz-debloater` respects the following environment variables:

- `TZ_AI_API_KEY`: Universal API key fallback for the active connection.
- `GEMINI_API_KEY`: API key for Google Gemini connection profile.
- `OPENAI_API_KEY`: API key for OpenAI connection profile.
- `ANTHROPIC_API_KEY`: API key for Anthropic connection profile.
- `TZ_DEBLOATER_TEST_DATA`: Overrides the local data folder path (useful for hermetic test suites).

---

## 6. Automation & Scripting Examples

### PowerShell: Safe OEM Debloating Pipeline
```powershell
# 1. Verify environment and device connection
tz-debloater doctor --json | ConvertFrom-Json | ForEach-Object {
    if ($_.devices.ready -eq 0) {
        Write-Error "No ready device attached!"
        exit 1
    }
}

# 2. Query debloat candidates as JSON
$candidates = tz-debloater candidates --json | ConvertFrom-Json

# 3. Apply disable to all recommended OEM bloatware safely
$pkgList = ($candidates | Where-Object { $_.removal_recommendation -eq "Recommended" }).package_id -join ","
if ($pkgList) {
    Write-Host "Disabling: $pkgList"
    tz-debloater apply --packages $pkgList --action disable -y
}
```

### Bash: Automated Tracker Cleanup
```bash
#!/usr/bin/env bash
set -euo pipefail

# Generate tracker cleanup plan
tz-debloater plan --intent trackers --json > /tmp/trackers.json

# Apply plan with automated verification
tz-debloater apply --plan /tmp/trackers.json -y
echo "Trackers successfully debloated!"
```

