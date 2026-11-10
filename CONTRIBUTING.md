# Contributing to TZ AI DEBLOATER TOOL

Thank you for your interest in contributing to **TZ AI DEBLOATER TOOL**! We welcome community contributions, bug reports, feature suggestions, and catalog enhancements.

---

## Code of Conduct
We are committed to providing a respectful, harassment-free environment for all contributors. Please be kind, constructive, and respectful in all interactions.

---

## Architecture Principles to Preserve

All contributions must respect the core engineering principles of this project:

1. **Safety First**:
   Never allow an automated proposal or catalog entry to disable or uninstall a critical operating system dependency. Active launchers, default keyboards, emergency services, and core telephony components must remain protected in the Rust backend.
2. **Zero Telemetry**:
   No code may introduce analytics, tracking pixels, usage telemetry, phone-home beacons, or external reporting services.
3. **Local-First & Offline Capable**:
   Core package management, searching, catalog browsing, and recovery must work 100% offline without requiring internet access or cloud services.
4. **Action-Aware Semantics**:
   Preserve the strict separation between `Disable`, `UninstallUser`, `Enable`, and `Restore`. Never silently downgrade or convert a user's requested uninstall into a disable action.
5. **Unslop Design System**:
   Keep the user interface focused on technical precision: dark graphite surfaces, single teal accent (`#14b8a6`), monospace package identifiers, hairline borders, and zero decorative fluff (no rainbow gradient tiles, no emoji icons, no glowing cards).

---

## Pull Request Guidelines

1. **Fork and Branch**:
   - Create a feature branch off `main`:
     ```bash
     git checkout -b feat/your-feature-name
     ```
2. **Verification Requirements**:
   Before submitting your PR, ensure all three test suites pass cleanly:
   ```bash
   # 1. Contract tests
   npm test

   # 2. TypeScript and Vite build
   npm run build

   # 3. Rust unit and integration tests (81+ tests)
   cargo test --manifest-path src-tauri/Cargo.toml
   ```
3. **Atomic Commits**:
   Write clear, concise commit messages explaining the *why* behind your changes.
4. **Documentation**:
   If adding or changing CLI flags, subcommands, or configuration options, update the corresponding files in `docs/` (`CLI.md`, `USER_GUIDE.md`, etc.).

---

## Suggesting Catalog Additions

To suggest additions or corrections to the community package catalog:
- Package catalog entries are maintained in `src-tauri/src/dataset/`.
- Provide the exact Android package identifier (`com.example.app`).
- Include verified evidence: package category, OEM/carrier source, and justification for the proposed rating (`Recommended`, `Advanced`, `Expert`, or `Unsafe`).

