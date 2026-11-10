# Third-Party Software Notices and Attribution

**TZ AI DEBLOATER TOOL** incorporates, adapts, or references components, design systems, and datasets from the following open-source projects under their respective licenses:

---

## 1. unslop Design System
- **Author:** Damir Sokolovsky (`@heydamir`)
- **Repository / Reference:** `unslop` design tokens, surfaces, hairline borders, and typography guidelines
- **License:** Creative Commons Attribution 4.0 International (CC BY 4.0)
- **Copyright:** (C) 2024-2026 Damir Sokolovsky
- **Usage in TZ AI DEBLOATER TOOL:**
  - Visual identity, graphite tonal ramp (`#0f0f10`, `#141415`, `#19191b`, `#232325`), hairline borders (`#27272a`), muted teal accent (`#14b8a6`), tabular numbers, and calm, professional information hierarchy.

---

## 2. FreeLLMAPI Gateway & Routing Architecture
- **Author:** Tashfeen Ahmed
- **Project URL:** https://github.com/tashfeenahmed/freellmapi
- **License:** MIT License
- **Copyright:** (C) 2024-2026 Tashfeen Ahmed
- **Usage in TZ AI DEBLOATER TOOL:**
  - Multi-provider AI routing architecture, provider failover logic, and model routing algorithms adapted into the internal **TZ AI Gateway** engine with Windows Credential Store integration, health tracking, and streaming cancellation.

---

## 3. Universal Android Debloater Next Generation (UAD-ng)
- **Project URL:** https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation
- **Commit Inspected:** `9fcad87bf78c52c4270504bc8880139d95a27a29`
- **License:** GNU General Public License v3.0 (GPL-3.0)
- **Copyright:** (C) 2021-2026 Universal Debloater Alliance & contributors
- **Usage in TZ AI DEBLOATER TOOL:**
  - Package recommendation catalog and metadata dataset (`uad_lists.json` containing 5,399 package entries, classification lists, removal recommendations, descriptions, dependencies, and needed-by relations).
  - Strongly typed ADB interaction design, multi-user resolution logic, and cross-user behavior detection patterns.

---

## 4. Universal Android Debloater (Original)
- **Project URL:** https://github.com/0x192/universal-android-debloater
- **Commit Inspected:** `11f27c671cba278d71296cdef4c5a5dba06add5e`
- **License:** GNU General Public License v3.0 (GPL-3.0)
- **Copyright:** (C) 2020-2023 0x192 & contributors
- **Usage in TZ AI DEBLOATER TOOL:**
  - Historical architectural lineage and package categorization concepts.

---

## 5. Canta
- **Project URL:** https://github.com/samolego/Canta
- **Commit Inspected:** `e3ed0c9bf90ee6f8bfb7a92ba1e140327cda480f`
- **License:** GNU Lesser General Public License v3.0 (LGPL-3.0)
- **Copyright:** (C) 2023-2026 samolego & contributors
- **Usage in TZ AI DEBLOATER TOOL:**
  - Desktop helper execution architecture (`TzHelperBackend`), protocol framing, and `app_process` command invocation patterns adapted under LGPL-3.0 with device/user scoping and zero-dependency protocol decoding.
  - Package review workflows, non-root debloating risk warnings, and batch review interaction patterns.

---

## 6. Tauri Framework
- **Project URL:** https://github.com/tauri-apps/tauri
- **License:** Apache License 2.0 / MIT
- **Copyright:** (C) 2019-2026 Tauri Programme within The Commons Conservancy

---

## 7. Lucide Icons
- **Project URL:** https://github.com/lucide-icons/lucide
- **License:** ISC License
- **Copyright:** (C) 2020-2026 Lucide Contributors
