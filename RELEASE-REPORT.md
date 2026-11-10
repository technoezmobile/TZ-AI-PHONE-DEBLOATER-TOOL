# TZ AI DEBLOATER TOOL 1.0.1 Release Report

This release was repaired, verified, redesigned with the **Unslop** design system, and packaged directly in `E:\AI DEBLOATER` on **10 November 2026**.

## 1. Release Artifacts & Recommendations

The production release binaries and Windows installers are compiled and packaged in `release_artifacts/` (unsigned distribution binaries):

| Artifact | File Type | Size | SHA-256 Hash | Recommendation |
| :--- | :--- | :--- | :--- | :--- |
| **`TZ-AI-Debloater-Tool-Setup.exe`** | NSIS Installer | 3.57 MiB (3,743,264 bytes) | `269642B33AFEECB95CFA18A936F86AB9756D10C339C259EFD7F1A6F453DA818A` | **Recommended**: Standard desktop installation for Windows workstations (automatic per-user install, Start Menu & Desktop shortcuts, clean uninstaller). |
| **`TZ-AI-Debloater-Tool-x64.msi`** | WiX MSI Installer | 5.23 MiB (5,484,544 bytes) | `56847DB8AB2B6E452172BE8F231B09D9E13979AE298DF495A2F1CB7CDC5F1AA2` | Enterprise GPO / Active Directory / Intune automated system deployments. |
| **`TZ-AI-Debloater-Tool.exe`** | Standalone Executable | 15.18 MiB (15,921,664 bytes) | `2259CC5E6B02886BB200AE56208A423074E050CF3643B480A54A9C97964925BA` | Portable zero-install execution from technician flash drives. |

All binaries are verified as clean local distribution packages. The local test environment installation (`C:\Users\singh\AppData\Local\TZ AI DEBLOATER TOOL\tz-ai-debloater-tool.exe`, 15,921,664 bytes) is synchronized and verified.

---

## 2. Package Classification, Selection & Preflight Overhaul

### 1. Separation of Package Type, Catalog Rating, and Removal Eligibility
Three distinct dimensions now govern package management across backend and UI:
1. **Android Package Type** (`AndroidPackageType`):
   - `System`: Installed in system image partition (`/system/`, `/product/`, `/vendor/`).
   - `UpdatedSystem`: System application updated via Google Play or OEM updater, detected by APK residing in `/data/` or `/data/app/` via `pm list packages -s -f`.
   - `User`: Standard third-party user application.
2. **Catalog Rating** (`removal_recommendation`):
   - Curated community rating from UAD-ng (`Recommended`, `Advanced`, `Expert`, `Unsafe`, `Unlisted`).
   - "Miscellaneous" preserved strictly as a catalog category; unlisted apps receive `"Unlisted"` (never `"Misc"`).
3. **Current Removal Eligibility** (`RemovalEligibility`):
   - `Eligible`: Curated candidate with sufficient evidence for removal on the current ROM.
   - `ReviewOnly`: Uncertain vendor packages, built-in utilities, or user apps needing explicit user review.
   - `Protected`: Critical system components, active default launcher, active keyboard, or confirmed critical vendor services.
   - `AlreadyRemoved`: Packages uninstalled for the target user (`PackageState::Uninstalled`).
   - **Absolute Safety Rule**: A catalog "Recommended" rating **never overrides device protection rules** or implies safety on the connected ROM. If a package is protected, `removal_eligibility` is strictly `Protected`.

### 2. Default "Debloat candidates" View & Explicit View Tabs
Package Explorer now opens by default to the **Debloat candidates** view:
- **`Debloat candidates` (Default)**:
  - Displays exclusively currently installed packages (`Enabled` or `Disabled`, strictly excluding `Uninstalled`) for the active Android user that have verified evidence and eligibility for debloating.
- **`All packages`**:
  - Full inventory view for comprehensive device inspection.
- **`System & vendor`**:
  - Focuses on OEM, MediaTek/Qualcomm, and Android system services.
- **`Removed for this user`**:
  - Displays uninstalled packages (`PackageState::Uninstalled`) with direct **Restore package** action buttons (`cmd package install-existing`). Uninstalled packages are strictly excluded from new removal plans.

### 3. MediaTek Vendor Services & Built-in ROM Utilities Safety
- **Confirmed Critical MediaTek Services**:
  - `com.mediatek.telephony`, `com.mediatek.ims`, `com.mediatek.wfo.impl`, `com.mediatek.engineermode`, `com.mediatek.carrierconfig`, etc., are hard-blocked as critical protected packages in `SafetyPolicy`.
- **Uncertain MediaTek Packages**:
  - Other MediaTek vendor packages (e.g. `com.mediatek.miravision`, `com.mediatek.duraspeed`) are classified as `ReviewOnly` and excluded from default debloat candidates and automated AI plans.
- **Built-in Utilities**:
  - MIUI Compass (`com.miui.compass`), Notes (`com.miui.notes`), Calculator (`com.miui.calculator`), Weather, Screen Recorder, etc., are identified via `SafetyPolicy::is_built_in_utility` and classified as `ReviewOnly`. They do not appear in default candidate lists or automated AI plans, requiring separate explicit review.

### 4. Cross-Filter Batch Selection & Hidden Count Warning
- When packages are selected and subsequent filter or tab changes hide some of the selected packages, the batch toolbar displays:
  `"{N} packages selected ({M} hidden by current view/filter)"`.
- Prevents accidental execution of hidden selections while allowing deliberate cross-category selections.

### 5. Backend Preflight Revalidation (`validate_batch_selection`)
- Before executing any batch action, the frontend calls `validate_batch_selection` in Tauri.
- The backend queries fresh device package state and revalidates every entry against device protection policy.
- An interactive confirmation modal displays:
  - Exact Package ID, Type, Current State, Action, Target User, and Eligibility.
  - Excluded packages clearly show the exact refusal reason (e.g., *"Confirmed critical telephony/modem/radio vendor service"*, *"Protected component: Active Default Launcher"*, *"Package is already uninstalled for this user"*).
  - Only confirmed eligible items are executed when confirmed.

---

## 3. Real-Device Regression Repairs & UI Polish

### 1. Clipped Controls and Responsive Chat Rendering
- **Settings Profile Tabs & Credential Actions**:
  - Wrapped with flexbox (`flex-wrap: wrap`) preventing horizontal overflow.
  - Action buttons (`Test entered key`, `Test saved key`, `Remove`) wrap underneath cleanly.
  - Automatically collapses to single-column stacking below 1080px.
- **AI Assistant Layout Integrity**:
  - Clear chat, model picker, composer, and Send button remain sticky and reachable across all window sizes (from 760px to 1920px).
  - Four quick shortcuts: `Review bloatware`, `Review trackers`, `Uninstall promos`, `List candidates` with visible labels and `flex-wrap: wrap`.
  - Long package IDs and prompt inputs wrapped using `word-break: break-word` and `overflow-wrap: anywhere`.
- **Markdown Tables in Chat**:
  - Replaced raw text Markdown pipe display with responsive semantic HTML `<table>` elements.
  - Status badges: `✓ Success` (teal), `ℹ Already Removed` (zinc), `✕ Failed` (rose).
  - Loading indicator while responses stream.

### 2. Default Gemini Direct to `gemini-3.5-flash`
- Default model for all new Gemini Direct profiles updated to `gemini-3.5-flash`.
- Bidirectional synchronization between chat model selector and Settings.
- Live Google Gemini API verification verified (HTTP 200, 1117ms latency).

---

## 4. POCO Verification Screenshots

Captured and stored in both `screenshots/` and artifact directories:

1. **POCO Debloat Candidates (`screenshots/poco_debloat_candidates.png`)**:
   - Shows default "Debloat candidates" tab.
   - Contains only eligible debloat packages (`com.miui.msa.global`, `com.facebook.appmanager`, `com.xiaomi.discover`, `com.xiaomi.glgm`, `com.mipay.wallet.in`).
   - MediaTek services, MIUI Compass/Notes, and uninstalled packages are excluded.

2. **POCO System & Vendor (`screenshots/poco_system_vendor.png`)**:
   - Shows "System & vendor" view.
   - Highlights MediaTek services (`com.mediatek.telephony` [Protected], `com.mediatek.miravision` [Review-Only]), MIUI utilities (`com.miui.compass` [Built-in utility], `com.miui.notes` [Updated System]), and active launcher (`com.miui.home` [Protected]).

3. **POCO Removed for this User (`screenshots/poco_removed_user.png`)**:
   - Shows "Removed for this user" tab.
   - Displays `com.miui.analytics` and `com.google.android.apps.tachyon` with state `Uninstalled`, badge `Removed`, and direct **Restore package** buttons.

4. **POCO Batch Preflight Revalidation (`screenshots/poco_batch_preflight.png`)**:
   - Shows preflight confirmation modal before batch execution.
   - Displays exact table of Package ID, Type, Current State, Action, Target User, and Status with exclusion reasons.

---

## 5. Test Suite & Verification Results

```powershell
npm test                                                  # 2 passed, 0 failed
npm run build                                             # 2071 modules transformed, 0 errors
cargo test --manifest-path src-tauri/Cargo.toml           # 77 passed, 0 failed
node scripts/capture_poco_verification.mjs                # 4 screenshots verified & captured
npm run tauri -- build --ci --bundles nsis,msi           # Production NSIS & MSI builds verified
```
