use crate::adb::operation::ActionType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionResult {
    pub is_protected: bool,
    pub is_critical_core: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SafetyPolicy {
    critical_packages: HashSet<&'static str>,
    critical_prefixes: Vec<&'static str>,
    pub active_launcher: Option<String>,
    pub active_ime: Option<String>,
}

impl Default for SafetyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetyPolicy {
    pub fn new() -> Self {
        let mut critical_packages = HashSet::new();
        // AOSP & Android OS Core
        critical_packages.insert("android");
        critical_packages.insert("android.ext.services");
        critical_packages.insert("android.ext.shared");
        critical_packages.insert("com.android.systemui");
        critical_packages.insert("com.android.settings");
        critical_packages.insert("com.android.keyguard");
        critical_packages.insert("com.android.phone");
        critical_packages.insert("com.android.server.telecom");
        critical_packages.insert("com.android.emergency");
        critical_packages.insert("com.google.android.emergency");

        // Permission & Package Installation
        critical_packages.insert("com.android.packageinstaller");
        critical_packages.insert("com.google.android.packageinstaller");
        critical_packages.insert("com.android.permissioncontroller");
        critical_packages.insert("com.google.android.permissioncontroller");

        // Essential Core Storage & Telephony Providers
        critical_packages.insert("com.android.providers.settings");
        critical_packages.insert("com.android.providers.telephony");

        // Confirmed Critical MediaTek & Vendor Telephony/Network Services
        critical_packages.insert("com.mediatek.telephony");
        critical_packages.insert("com.mediatek.ims");
        critical_packages.insert("com.mediatek.wfo.impl");
        critical_packages.insert("com.mediatek.engineermode");
        critical_packages.insert("com.mediatek.capctrl");
        critical_packages.insert("com.mediatek.framework.res");
        critical_packages.insert("com.mediatek.carrierconfig");
        critical_packages.insert("com.mediatek.simprocessor");
        critical_packages.insert("com.mediatek.cellbroadcastreceiver");
        critical_packages.insert("com.mediatek.location.lppe.main");
        critical_packages.insert("com.mediatek.location.mtknps");
        critical_packages.insert("com.mediatek.mdmlsample");
        critical_packages.insert("com.qualcomm.qti.telephony");
        critical_packages.insert("com.sec.android.app.telephonyui");

        // Essential Keyboards
        critical_packages.insert("com.google.android.inputmethod.latin"); // Gboard
        critical_packages.insert("com.android.inputmethod.latin");

        let critical_prefixes = vec!["com.android.systemui.", "com.android.permissioncontroller."];

        Self {
            critical_packages,
            critical_prefixes,
            active_launcher: None,
            active_ime: None,
        }
    }

    pub fn set_active_roles(&mut self, launcher: Option<String>, ime: Option<String>) {
        self.active_launcher = launcher;
        self.active_ime = ime;
    }

    /// Check if a package is protected under the conservative safety policy
    pub fn check_protection(&self, package_id: &str) -> ProtectionResult {
        // 1. Dynamic active launcher protection
        if let Some(ref launcher) = self.active_launcher {
            if package_id == launcher {
                return ProtectionResult {
                    is_protected: true,
                    is_critical_core: true,
                    reason: Some(format!(
                        "Active Default Launcher '{package_id}'. Disabling or removing the primary launcher will leave the home screen blank."
                    )),
                };
            }
        }

        // 2. Dynamic active keyboard / IME protection
        if let Some(ref ime) = self.active_ime {
            if package_id == ime {
                return ProtectionResult {
                    is_protected: true,
                    is_critical_core: true,
                    reason: Some(format!(
                        "Active Default Keyboard / Input Method '{package_id}'. Disabling this will prevent typing on device."
                    )),
                };
            }
        }

        // 3. Static critical core OS packages
        if self.critical_packages.contains(package_id) {
            return ProtectionResult {
                is_protected: true,
                is_critical_core: true,
                reason: Some(format!(
                    "Critical Android OS Core component '{package_id}'. Modifying this package may brick the device or cause an immediate bootloop."
                )),
            };
        }

        for prefix in &self.critical_prefixes {
            if package_id.starts_with(prefix) {
                return ProtectionResult {
                    is_protected: true,
                    is_critical_core: true,
                    reason: Some(format!(
                        "Essential system interface component matching protected prefix '{prefix}'."
                    )),
                };
            }
        }

        ProtectionResult {
            is_protected: false,
            is_critical_core: false,
            reason: None,
        }
    }

    /// Legacy helper for action safety checking
    pub fn validate_action_safety(
        &self,
        package_id: &str,
        removal_category: &str,
        user_override: bool,
    ) -> Result<(), String> {
        let prot = self.check_protection(package_id);
        if prot.is_critical_core {
            return Err(format!(
                "BLOCKED BY SAFETY POLICY: '{package_id}' is a vital OS package"
            ));
        }
        if removal_category.eq_ignore_ascii_case("unsafe") && !user_override {
            return Err(format!(
                "Package '{package_id}' is unsafe and requires explicit confirmation"
            ));
        }
        Ok(())
    }

    /// Action-aware policy check:
    /// Restoring or Enabling an essential package is allowed (for recovery);
    /// Disabling or Uninstalling an essential package is blocked!
    pub fn is_action_allowed(&self, package_id: &str, action: ActionType) -> Result<(), String> {
        let prot = self.check_protection(package_id);

        if prot.is_protected {
            match action {
                ActionType::Enable | ActionType::Restore => {
                    // Allowed: restoring or re-enabling a protected package is safe and helps recover devices
                    Ok(())
                }
                ActionType::Disable | ActionType::UninstallUser => {
                    Err(format!(
                        "BLOCKED BY SAFETY POLICY: '{}' is a protected component ({:?}). It cannot be disabled or uninstalled.",
                        package_id,
                        prot.reason.unwrap_or_else(|| "Vital system service".into())
                    ))
                }
            }
        } else {
            Ok(())
        }
    }

    /// Static helper for quick check
    pub fn is_package_protected(package_id: &str) -> (bool, Option<String>) {
        let policy = Self::new();
        let res = policy.check_protection(package_id);
        (res.is_protected, res.reason)
    }

    /// Identify ROM built-in utilities (MIUI Compass, Notes, Calculator, etc.)
    /// These are kept out of default debloat candidates and AI shortcut plans,
    /// requiring a separate explicit review for any supported action on them.
    pub fn is_built_in_utility(package_id: &str) -> bool {
        matches!(
            package_id,
            "com.miui.compass"
                | "com.miui.notes"
                | "com.miui.calculator"
                | "com.miui.weather2"
                | "com.miui.screenrecorder"
                | "com.miui.soundrecorder"
                | "com.xiaomi.scanner"
                | "com.miui.fm"
                | "com.sec.android.app.popupcalculator"
                | "com.samsung.android.app.notes"
                | "com.google.android.calculator"
                | "com.google.android.deskclock"
        )
    }

    /// Check if a package is a confirmed critical vendor service (telephony, RIL, network, radio)
    pub fn is_confirmed_critical_vendor_service(package_id: &str) -> bool {
        matches!(
            package_id,
            "com.mediatek.telephony"
                | "com.mediatek.ims"
                | "com.mediatek.wfo.impl"
                | "com.mediatek.engineermode"
                | "com.mediatek.capctrl"
                | "com.mediatek.framework.res"
                | "com.mediatek.carrierconfig"
                | "com.mediatek.simprocessor"
                | "com.mediatek.cellbroadcastreceiver"
                | "com.mediatek.location.lppe.main"
                | "com.mediatek.location.mtknps"
                | "com.mediatek.mdmlsample"
                | "com.qualcomm.qti.telephony"
                | "com.sec.android.app.telephonyui"
        )
    }

    /// MediaTek vendor packages with uncertain roles must be treated as review-only,
    /// rather than inferred safe or critical from package prefix alone.
    pub fn is_uncertain_vendor_package(package_id: &str) -> bool {
        (package_id.starts_with("com.mediatek.") || package_id.starts_with("vendor.mediatek."))
            && !Self::is_confirmed_critical_vendor_service(package_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_packages_are_protected() {
        let policy = SafetyPolicy::new();
        assert!(policy.check_protection("com.android.systemui").is_protected);
        assert!(policy.check_protection("com.android.settings").is_protected);
        assert!(
            policy
                .check_protection("com.google.android.packageinstaller")
                .is_protected
        );
        assert!(
            policy
                .check_protection("com.google.android.permissioncontroller")
                .is_protected
        );
        assert!(policy.check_protection("android").is_protected);
    }

    #[test]
    fn test_active_launcher_and_ime_protection() {
        let mut policy = SafetyPolicy::new();
        policy.set_active_roles(
            Some("com.sec.android.app.launcher".into()),
            Some("com.touchtype.swiftkey".into()),
        );

        let launcher_prot = policy.check_protection("com.sec.android.app.launcher");
        assert!(launcher_prot.is_protected);
        assert!(launcher_prot
            .reason
            .unwrap()
            .contains("Active Default Launcher"));

        let ime_prot = policy.check_protection("com.touchtype.swiftkey");
        assert!(ime_prot.is_protected);
        assert!(ime_prot.reason.unwrap().contains("Active Default Keyboard"));
    }

    #[test]
    fn test_action_aware_policy() {
        let policy = SafetyPolicy::new();

        // Disabling or uninstalling SystemUI must be blocked
        assert!(policy
            .is_action_allowed("com.android.systemui", ActionType::Disable)
            .is_err());
        assert!(policy
            .is_action_allowed("com.android.systemui", ActionType::UninstallUser)
            .is_err());

        // Enabling or restoring SystemUI must be permitted for recovery
        assert!(policy
            .is_action_allowed("com.android.systemui", ActionType::Enable)
            .is_ok());
        assert!(policy
            .is_action_allowed("com.android.systemui", ActionType::Restore)
            .is_ok());

        // Normal third-party packages can be disabled or uninstalled
        assert!(policy
            .is_action_allowed("com.facebook.katana", ActionType::Disable)
            .is_ok());
        assert!(policy
            .is_action_allowed("com.facebook.katana", ActionType::UninstallUser)
            .is_ok());
    }

    #[test]
    fn test_mediatek_critical_services_and_uncertain_packages() {
        let policy = SafetyPolicy::new();

        // Confirmed critical services must be blocked by policy
        assert!(SafetyPolicy::is_confirmed_critical_vendor_service("com.mediatek.telephony"));
        assert!(SafetyPolicy::is_confirmed_critical_vendor_service("com.mediatek.ims"));
        assert!(SafetyPolicy::is_confirmed_critical_vendor_service("com.mediatek.wfo.impl"));
        assert!(policy
            .is_action_allowed("com.mediatek.telephony", ActionType::UninstallUser)
            .is_err());
        assert!(policy
            .is_action_allowed("com.mediatek.ims", ActionType::Disable)
            .is_err());

        // Uncertain vendor packages are review-only, not confirmed critical
        assert!(SafetyPolicy::is_uncertain_vendor_package("com.mediatek.miravision"));
        assert!(!SafetyPolicy::is_confirmed_critical_vendor_service("com.mediatek.miravision"));
    }

    #[test]
    fn test_built_in_utilities_identified() {
        assert!(SafetyPolicy::is_built_in_utility("com.miui.compass"));
        assert!(SafetyPolicy::is_built_in_utility("com.miui.notes"));
        assert!(SafetyPolicy::is_built_in_utility("com.miui.calculator"));
        assert!(SafetyPolicy::is_built_in_utility("com.sec.android.app.popupcalculator"));
        assert!(!SafetyPolicy::is_built_in_utility("com.facebook.katana"));
    }
}
