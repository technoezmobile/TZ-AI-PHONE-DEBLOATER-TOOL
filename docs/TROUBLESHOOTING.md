# Troubleshooting & Diagnostic Guide

This guide provides technical explanations and solutions for real-world scenarios encountered when debloating Android devices.

---

## 1. Device Discovery & Connection Issues

### Device Shows as "Unauthorized"
- **Cause**: The Android device has not yet accepted your computer's RSA debugging key.
- **Solution**:
  1. Unplug the USB cable from the phone.
  2. On the phone, go to **Developer Options > Revoke USB debugging authorizations** and tap **OK**.
  3. Reconnect the USB cable.
  4. Unlock the phone screen. A prompt *"Allow USB debugging?"* will appear.
  5. Check *"Always allow from this computer"* and tap **Allow**.
  6. Click **Rescan Devices** in TZ AI DEBLOATER TOOL.

### Device Shows as "Offline"
- **Cause**: Flaky USB connection, faulty cable, or stalled ADB daemon.
- **Solution**:
  1. Replace the cable (use a known-good data cable, not charge-only).
  2. Ensure the USB connection mode on the phone is set to **File Transfer (MTP)** or **PTP**, rather than "Charge only".
  3. In terminal, restart the ADB daemon:
     ```bash
     adb kill-server
     adb start-server
     ```

### Wireless ADB Disconnected After Reboot
- **Cause**: In Android 11+, the Wireless Debugging port dynamically regenerates whenever Wi-Fi reconnects or the device restarts. The pairing port and connection port are distinct.
- **Solution**:
  1. Pairing only needs to be completed once.
  2. When reconnecting, open **Developer Options > Wireless Debugging** and check the current port number next to the IP address (e.g. `192.168.1.45:39485`).
  3. In TZ AI DEBLOATER TOOL, enter this new port into the **Connect** field (pairing code is not required on subsequent connects).

---

## 2. OEM-Specific Restrictions & Permission Denials

### Xiaomi / Redmi / POCO (MIUI & HyperOS)
#### Symptoms
- Disabling a package fails with:
  `java.lang.SecurityException: Permission Denial: attempt to change component state from pid=...`
- The diagnostic classifies the error as: `OemRestriction` (Xiaomi MIUI Security Settings required).

#### Cause
Xiaomi builds enforce an extra layer of protection on `pm disable-user`. Non-root shell connections cannot disable packages without an explicit developer flag.

#### Solutions
- **Option A (Recommended — Per-User Uninstall)**:
  Instead of `Disable`, use **Uninstall for User** (`ActionType::UninstallUser`). Xiaomi ROMs do NOT block `pm uninstall -k --user 0`. The app will be cleanly removed from your profile and its background services stopped immediately.
- **Option B (Enable Security Settings)**:
  1. Go to **Settings > Additional Settings > Developer Options**.
  2. Scroll down to **USB debugging (Security settings)**.
  3. Toggle it ON. (Xiaomi requires an active SIM card and signed-in Mi Account to toggle this setting).

---

### Samsung Galaxy (Knox / Enterprise MDM)
#### Symptoms
- Package modification fails with:
  `java.lang.SecurityException: Cannot disable a protected package: ... ADMIN_RESTRICTED`
- Diagnostic classifies the error as: `DevicePolicyRestricted`.

#### Cause
The device is enrolled in an enterprise MDM (Mobile Device Management) profile (e.g. Samsung Knox, Microsoft Intune, Google Workspace Work Profile). The enterprise device admin policy has set `DISALLOW_APPS_CONTROL` or explicitly locked down the package.

#### Solution
Packages governed by active MDM device administration policies cannot be removed via non-root ADB without un-enrolling the device or requesting administrative exemption.

---

## 3. Safety Policy Blocks

### "BLOCKED BY SAFETY POLICY: Active Default Launcher"
- **Reason**: The package you requested to disable or remove is currently serving as your active home screen launcher (`cmd package resolve-activity`).
- **Safety Impact**: Disabling your only launcher leaves the phone with a blank black screen upon pressing the Home button or unlocking.
- **Remedy**: If you wish to replace your OEM launcher (e.g. Nova Launcher, Lawnchair):
  1. Install your alternative launcher from Google Play or APK.
  2. Set the new launcher as default in **Settings > Default Apps > Home App**.
  3. Rescan packages in TZ AI DEBLOATER TOOL. The old OEM launcher will no longer be active, allowing safe disablement.

### "BLOCKED BY SAFETY POLICY: Active Input Method (Keyboard)"
- **Reason**: The package is currently registered as your active on-screen keyboard (`ime list -s`).
- **Remedy**: Install and activate an alternative keyboard (e.g. Gboard, SwiftKey, FlorisBoard) before disabling the OEM keyboard.

---

## 4. Emergency Recovery Procedures

### Restoring an Uninstalled Package
System packages uninstalled with `pm uninstall -k --user 0` remain untouched in the `/system` partition. They can be restored at any time:

#### Via Graphical Interface:
1. Open **Package Explorer**.
2. Select the **Removed for this user** view tab.
3. Locate the package and click **Restore**.

#### Via Standalone CLI:
```bash
tz-debloater restore --packages "com.google.android.apps.youtube" -y
```

#### Via Direct Raw ADB:
```bash
adb shell cmd package install-existing --user 0 <package_name>
```

---

### Inadvertent Bootloop Recovery (Safe Mode)
If an unlisted package was removed and causes the device to soft-reboot or freeze on boot:

1. **Boot into Android Safe Mode**:
   - Power off the device completely.
   - Hold the **Power** button until the manufacturer logo appears.
   - Immediately press and hold **Volume Down** until the lock screen loads.
   - The screen will display *"Safe Mode"* in the lower-left corner.
2. In Safe Mode, standard third-party apps are disabled and core Android frameworks initialize with minimal dependencies.
3. Connect your USB cable. ADB remains active in Safe Mode.
4. Run `tz-debloater restore --latest -y` or re-enable the package via `cmd package install-existing`.
5. Reboot normally.

---

## 5. Cross-Process Lock Troubleshooting

### "Target resource is currently locked by another active instance"
- **Cause**: An active operation is running in the desktop app while a CLI command is invoked, or an instance was abruptly killed while writing to the journal.
- **Handling**:
  TZ AI DEBLOATER TOOL automatically checks the operating system process table (`tasklist` on Windows, `kill -0` on Unix). If the PID that created the lock is dead or older than 300 seconds, the stale lock is safely cleared automatically.
- **Manual Cleanup**:
  If needed, you can delete any leftover `.lock` files in:
  - Windows: `%LOCALAPPDATA%\TZ_AI_Debloater\locks\`
  - macOS / Linux: `~/.local/share/TZ_AI_Debloater/locks/`

