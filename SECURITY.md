# Security Policy

**TZ AI DEBLOATER TOOL** takes security seriously. As a tool designed to interact with low-level Android debug bridges and handle API credentials, we adhere to strict defensive programming and local security practices.

---

## Supported Versions

| Version | Supported |
| :--- | :---: |
| `1.1.x` | :white_check_mark: |
| `< 1.1.0` | :x: |

We recommend always running the latest release to ensure you have the most up-to-date safety rules and vulnerability protections.

---

## Security Principles & Protections

1. **OS-Backed Credential Vaults**:
   API keys for AI integrations are stored in native operating system keyrings:
   - Windows: Microsoft Credential Manager (`advapi32.dll`) encrypted via DPAPI.
   - macOS: Apple Keychain Services (`/usr/bin/security`).
   - Linux: FreeDesktop Secret Service API (`secret-tool`) or restricted `0o600` file store.
   - Keys are **never serialized to plaintext configuration files**.
2. **Command Injection Prevention**:
   All package identifiers and device serials are validated against strict alphanumeric and reverse-DNS regex rules (`^[a-zA-Z0-9_.]+$`). Subprocesses use direct argument vector execution rather than raw shell string concatenation (`cmd /c` or `sh -c`), neutralizing argument injection attacks.
3. **Endpoint Whitelisting & Loopback Security**:
   Outbound network traffic from the AI Assistant is strictly bounded:
   - Loopback endpoints (`localhost`, `127.0.0.1`, `::1`) are permitted for local inference servers.
   - Remote endpoints require explicit user approval before device metadata is transmitted.
   - Cross-origin redirects that attempt to leak authentication headers are rejected.
4. **Backend-Owned Approval Gates**:
   Plan approvals are cryptographically bound to monotonic revisions and active device serials in the Rust backend. Approvals cannot be replayed or modified in the frontend.

---

## Reporting a Vulnerability

If you discover a security vulnerability in this project:

1. **Do not open a public issue.**
2. Send an email to **security@tzlabs.in** or submit a confidential report through GitHub's [Security Advisory feature](https://github.com/tz-labs/tz-ai-debloater-tool/security/advisories/new).
3. Please include:
   - A detailed description of the vulnerability.
   - Steps to reproduce or proof-of-concept code.
   - Potential impact on connected devices or user workstations.

We will acknowledge receipt within 48 hours and work with you to verify and resolve the issue before public disclosure.

