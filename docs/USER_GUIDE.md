# TZ AI DEBLOATER TOOL — User Guide

Welcome to **TZ AI DEBLOATER TOOL**, an intelligent, safety-conscious workstation engineered for Android power users, system administrators, and technicians.

This guide provides end-to-end instructions for connecting devices, inspecting package inventories, understanding package classifications, executing safe manual or AI-assisted debloating operations, and recovering packages.

---

## Table of Contents
1. [Connecting Your Android Device](#1-connecting-your-android-device)
   - [USB Debugging](#usb-debugging)
   - [Wireless ADB (Android 11+)](#wireless-adb-android-11)
2. [Multi-User Profiles](#2-multi-user-profiles)
3. [Understanding Package Classification](#3-understanding-package-classification)
   - [1. Android Package Type](#1-android-package-type)
   - [2. Community Catalog Rating](#2-community-catalog-rating)
   - [3. Current Removal Eligibility](#3-current-removal-eligibility)
4. [Browsing & Filtering Packages](#4-browsing--filtering-packages)
   - [Default Debloat Candidates View](#default-debloat-candidates-view)
   - [Other Standard Views](#other-standard-views)
5. [The Four AI Assistant Shortcuts](#5-the-four-ai-assistant-shortcuts)
6. [Manual Package Management](#6-manual-package-management)
   - [Disable vs Uninstall-User vs Restore](#disable-vs-uninstall-user-vs-restore)
   - [Batch Selection & Preflight Validation](#batch-selection--preflight-validation)
7. [Audit Journal & Recovery](#7-audit-journal--recovery)
8. [Configuring AI Providers](#8-configuring-ai-providers)
9. [Keyboard Shortcuts](#9-keyboard-shortcuts)
10. [Safety Guarantees & Privacy](#10-safety-guarantees--privacy)

---

## 1. Connecting Your Android Device

### USB Debugging
1. On your Android device, navigate to **Settings > About Phone**.
2. Tap **Build Number** 7 times consecutively until the prompt displays *"You are now a developer!"*.
3. Go to **Settings > System > Developer Options** (or **Additional Settings > Developer Options**).
4. Enable **USB Debugging**.
5. Connect your device to your workstation via a USB data cable.
6. A prompt will appear on your phone: *"Allow USB debugging?"*. Check *"Always allow from this computer"* and tap **Allow**.
7. In TZ AI DEBLOATER TOOL, click **Rescan Devices**. Your device will appear in the connection header with a green `Ready` indicator.

> [!TIP]
> **Xiaomi / Redmi / POCO Devices (MIUI & HyperOS):**
> If you wish to disable packages via ADB, MIUI requires enabling **USB debugging (Security settings)** in Developer Options. This requires a signed-in Mi Account and an inserted SIM card. However, **Per-User Uninstallation** (`pm uninstall -k --user 0`) works without this toggle!

### Wireless ADB (Android 11+)
1. Ensure your computer and Android device are connected to the same Wi-Fi network.
2. In **Developer Options**, enable **Wireless Debugging** and tap on it.
3. Tap **Pair device with pairing code**. Note the IP address, port, and 6-digit Wi-Fi pairing code.
4. In the tool, navigate to **Devices > Wireless Pairing**, enter the host, pairing port, and code, then click **Pair Device**.
5. Once paired, enter the connection port from the main Wireless Debugging screen and click **Connect**.

---

## 2. Multi-User Profiles

Android supports multiple user spaces on a single physical device:
- **User 0 (Owner / Primary User)**: The primary device owner and main application environment.
- **Work Profile (e.g. User 10)**: Created by Android Enterprise or MDM (Google Workspace, Intune).
- **Cloned Apps / Dual Apps / Second Space (e.g. User 999)**: Created by OEM dual-app features.

In TZ AI DEBLOATER TOOL, you can select which profile you are targeting via the user dropdown in the top header. Package changes and uninstallations will only affect the selected user profile, leaving other profiles intact.

---

## 3. Understanding Package Classification

Unlike primitive tools that combine safety, package location, and user intent into a single confusing badge, TZ AI DEBLOATER TOOL separates classification into **three distinct dimensions**:

### 1. Android Package Type (Runtime Inspection)
Determined directly from Android's package manager runtime metadata:
- **System**: An immutable system application pre-installed in `/system`, `/vendor`, or `/product`.
- **Updated System**: A system application that has received updates from Google Play or an APK installer, located in `/data/app` with base files in `/system`.
- **User**: A standard third-party application installed in `/data/app`.

### 2. Community Catalog Rating (Static Knowledge Base)
Sourced from the verified community debloating database (1,500+ analyzed packages):
- **Recommended**: Safe to remove or disable for virtually all users with no impact on core functionality.
- **Advanced**: Safe to remove, but removes optional features (e.g. printing framework, live wallpapers, cloud backup).
- **Expert**: Removal requires deep technical understanding; may break secondary OEM services.
- **Unsafe**: Critical component (telephony, emergency dialer, Bluetooth stack, display server).
- **Unlisted**: Package is not yet classified in the community database.

> [!IMPORTANT]
> A catalog rating of "Recommended" **never overrides** runtime safety rules. If a package is acting as your currently active home screen launcher or keyboard, it is automatically protected.

### 3. Current Removal Eligibility (Backend Policy Decision)
The backend evaluates the package dynamically to assign one of four eligibility states:
- **Eligible**: Currently installed for this user and verified safe for removal.
- **Review Only**: Package requires explicit review before modification (e.g. uncertain MediaTek vendor packages, OEM utilities like Compass or Notes).
- **Protected**: Core operating system dependency, active launcher, active input method, or critical telephony component. Removal is strictly blocked in the backend.
- **Already Removed**: The package has already been uninstalled or disabled for the active user profile.

---

## 4. Browsing & Filtering Packages

### Default Debloat Candidates View
When opening the **Package Explorer**, the tool defaults to the **Debloat Candidates** view. This view displays only packages that:
1. Are currently installed and active for the selected Android user.
2. Have verified evidence recommending removal (Recommended rating in catalog).
3. Are NOT protected by system policy.

This prevents user overwhelm and ensures you only see actionable, high-confidence candidates.

### Other Standard Views
- **All Packages**: Complete inventory of every package detected on the device.
- **System & Vendor**: Filters for packages installed in system or vendor partitions.
- **Removed for this User**: Shows packages previously uninstalled or disabled for this profile, allowing instant one-click restoration.

---

## 5. The Four AI Assistant Shortcuts

The AI Assistant includes four dedicated single-click shortcuts. Each shortcut enforces a distinct scope and purpose:

| Shortcut | Scope & Behavior | Action Type |
| :--- | :--- | :--- |
| **Review Bloatware** | Inspects verified bloatware candidates. Excludes built-in utilities (Notes, Compass) and uncertain vendor services. | `Disable` |
| **Review Trackers** | Scans strictly for telemetry, analytics, and advertising packages identified in the catalog. | `Disable` |
| **Uninstall Promos** | Identifies pre-installed promotional, sponsor, and partner apps (e.g. third-party games, shopping apps). | `UninstallUser` |
| **List Candidates** | **Read-Only**: Lists all catalog debloat candidates present on the device without proposing mutations. | *Read-Only* |

---

## 6. Manual Package Management

### Disable vs Uninstall-User vs Restore
- **Disable (`pm disable-user --user <uid> <package>`)**:
  Freezes the application. It disappears from the launcher, cannot run in the background, and receives no CPU cycles. Easily re-enabled at any time.
- **Uninstall for User (`pm uninstall -k --user <uid> <package>`)**:
  Uninstalls the application for the specified user profile while preserving original system files in `/system`. Frees user data space and completely removes the app from the profile.
- **Restore / Install Existing (`cmd package install-existing --user <uid> <package>`)**:
  Reinstalls a previously uninstalled system package for the user profile with zero data loss to the underlying system partition.
- **Enable (`pm enable --user <uid> <package>`)**:
  Unfreezes a previously disabled application.

### Batch Selection & Preflight Validation
You can select multiple packages using checkboxes in the Package Explorer. Clicking **Batch Action** opens a modal displaying:
- Total requested actions.
- Preflight validation breakdown: eligible packages vs blocked packages.
- Clear warnings if any selected package is protected.
- Monotonic approval gate to prevent stale executions.

---

## 7. Audit Journal & Recovery

Every operation performed by TZ AI DEBLOATER TOOL is logged to an immutable local audit journal:
- **Pre-Execution Snapshots**: A complete JSON snapshot of package states is captured immediately before any modification.
- **Verification Pass**: Immediately after executing a command, the tool rescans the package state to verify that the change actually took effect.
- **One-Click Recovery**: In the **History & Recovery** tab, you can view all past actions and click **Restore** to automatically revert packages to their previous state.

---

## 8. Configuring AI Providers

TZ AI DEBLOATER TOOL connects to various LLM backends in **Settings > AI Connection**:
1. **Google Gemini (Direct)**: Native Gemini API (`v1beta`). Paste your Gemini API key from Google AI Studio.
2. **OpenAI (Direct)**: Standard OpenAI chat completions.
3. **Anthropic (Direct)**: Claude Messages API.
4. **TZ AI Gateway**: Local proxy or self-hosted router (e.g. FreeLLMAPI).

All API keys are encrypted using native OS keyrings (Windows DPAPI, macOS Keychain, Linux Secret Service). Keys are never stored in plaintext `config.json`.

---

## 9. Keyboard Shortcuts

- `Tab` / `Shift+Tab`: Navigate between buttons, search fields, and table rows.
- `Space`: Toggle checkbox selection on the focused package row.
- `Enter`: Confirm action dialog or submit AI chat prompt.
- `Esc`: Close open modals or cancel plan review.
- `Ctrl+F` (or `Cmd+F` on macOS): Focus the package search bar.

---

## 10. Safety Guarantees & Privacy

- **Zero Telemetry**: No user analytics, no diagnostic pings, no telemetry sent to TZ Labs.
- **Non-Root Safety**: Commands operate through standard Android user-space ADB (`pm` / `cmd package`). They do not modify root partitions or trigger SafetyNet / Play Integrity tripping.
- **Offline Capable**: All manual debloating, package searching, catalog classification, and recovery functions work 100% offline without an internet connection.

