# Privacy, Security & Data Handling Policy

**TZ AI DEBLOATER TOOL** is designed from first principles around user privacy, operational transparency, and strict local-first execution.

This document details exactly how data is handled, stored, protected, and transmitted by both the desktop application and the `tz-debloater` CLI.

---

## 1. Core Privacy Commitments

1. **Zero Telemetry**:
   TZ AI DEBLOATER TOOL contains **no analytics tracking, no usage telemetry, no diagnostic beacons, and no crash reporters**. The software never initiates contact with any server owned or controlled by TZ Labs.
2. **Local-First Architecture**:
   All ADB communications, package manager parsing, safety policy validations, and audit journaling occur entirely on your local workstation.
3. **No Account Required**:
   The application requires no user registration, email, license key, or authentication to use its complete suite of Android management features.

---

## 2. Operating System Credential Storage

API keys for external AI providers (Google Gemini, OpenAI, Anthropic, or custom gateways) represent sensitive secrets. They are handled according to strict OS-level cryptographic standards:

### Platform-Specific Vaults
- **Windows**:
  Stored in **Windows Credential Manager** via the Win32 `advapi32.dll` API (`CredWriteW`, `CredReadW`, `CredDeleteW`) under targets prefixed with `TZ_AI_DEBLOATER_`. The underlying payload is encrypted with Microsoft DPAPI (Data Protection API) tied to your Windows user logon session.
- **macOS**:
  Stored in **Apple Keychain Services** using the `/usr/bin/security` generic password subsystem. Keys are encrypted with the macOS system keychain tied to your user account.
- **Linux**:
  Stored using the **FreeDesktop Secret Service API** (supported by GNOME Keyring and KeePassXC). On headless or minimal distributions without a D-Bus secret service daemon, credentials fall back to a restricted local vault with strict POSIX permissions (`0o600` file permissions, `0o700` directory permissions) accessible solely by your Linux user UID.

### Zero Plaintext Serialization
- API keys are **never serialized to `config.json`**.
- The graphical interface only receives a boolean flag (`has_credential: true|false`). Raw secret values are never exposed in UI templates, console logs, or browser DOM states.

---

## 3. AI Data Transmission & Consent Gate

When you choose to configure and use the AI Assistant, data transmission is governed by strict technical controls:

### Explicit Consent Gate
Before any prompt or package list is transmitted to an AI provider:
- The user must explicitly review and accept the **AI Data Transmission & Privacy Notice** in the tool settings.
- If consent is not given, the backend immediately rejects all AI chat and streaming requests with a `PermissionAccess` classified error.

### Destination Host Approval Whitelist
To protect against unauthorized exfiltration or misconfigured custom proxies:
- Loopback connections (`localhost`, `127.0.0.1`, `::1`) are permitted for local LLM servers.
- Any remote or custom HTTP/HTTPS endpoint must be explicitly approved by the user in **Settings > Destination Approval** before the application will transmit device data.

### Data Minimization & Scrubbing
When an AI debloating query is sent:
- **Transmitted Data**:
  - The text of the user prompt.
  - The relevant subset of package names (e.g. only currently installed packages matching the intent).
  - High-level device context: Device Brand, Model name, and Android Version (e.g. `Xiaomi 23049PCD8G, Android 14`).
- **Data Scrubbed & NEVER Transmitted**:
  - Physical hardware serial number (ADB serial is masked or omitted).
  - Phone numbers, IMEI, ICCID, or MAC addresses.
  - Personal accounts (Google account emails, Samsung accounts, etc.).
  - User application data, photos, SMS, messages, or files.

---

## 4. Local Audit Journal & Snapshots

To ensure accountability and reversibility:
- An audit journal (`operation_journal.json`) and pre-execution snapshots (`snapshots/*.json`) are maintained in:
  - Windows: `%LOCALAPPDATA%\TZ_AI_Debloater\`
  - macOS: `~/Library/Application Support/TZ_AI_Debloater/`
  - Linux: `~/.local/share/TZ_AI_Debloater/`
- These files reside exclusively on your local storage drive. They are never uploaded or synchronized across networks.
- Users can clear their history at any time by deleting the local folder or using the in-app history clearing tool.

---

## 5. Network Traffic Summary

| Action | Network Activity | Destination |
| :--- | :--- | :--- |
| **Device Discovery** | Local USB / Local Wi-Fi socket only (`127.0.0.1:5037`) | Android Device |
| **Package Scanning** | Local USB / Local Wi-Fi ADB shell only | Android Device |
| **Manual Debloating** | Local USB / Local Wi-Fi ADB shell only | Android Device |
| **Platform-Tools Download** | One-time manual download (if requested by user) | `dl.google.com` (Official Google CDN) |
| **AI Assistant (Gemini)** | User-prompted AI request with masked package list | `generativelanguage.googleapis.com` |
| **AI Assistant (OpenAI)** | User-prompted AI request with masked package list | `api.openai.com` |
| **AI Assistant (Anthropic)** | User-prompted AI request with masked package list | `api.anthropic.com` |
| **AI Assistant (Local Gateway)** | User-prompted AI request | Local loopback (e.g. `127.0.0.1:3001`) |
| **Everything Else** | **Zero Network Traffic** | None |

