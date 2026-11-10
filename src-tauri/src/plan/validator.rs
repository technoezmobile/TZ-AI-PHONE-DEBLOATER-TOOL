use crate::adb::command::is_valid_package_name;
use crate::adb::operation::ActionType;
use crate::ai::client::AiClient;
use crate::ai::types::{ActionPlan, ShortcutIntent};
use crate::dataset::get_catalog;
use crate::policy::SafetyPolicy;
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub executable_count: usize,
    pub protected_blocked_count: usize,
    pub missing_package_count: usize,
    pub unsafe_package_count: usize,
}

pub struct PlanValidator;

impl PlanValidator {
    /// Check whether a plan is invalidated due to device or user profile change
    pub fn check_invalidation(
        plan: &mut ActionPlan,
        current_serial: &str,
        current_user_id: u32,
    ) -> bool {
        if plan.target_device_serial != current_serial || plan.target_user_id != current_user_id {
            plan.approved_by_user = false;
            plan.is_valid = false;
            plan.invalidation_reason = Some("Target mismatch".into());
            true
        } else {
            false
        }
    }

    /// Validates an ActionPlan against the currently active device and user profile.
    /// Recomputes recommendation categories from trusted local catalog and enforces safety invariants.
    pub fn validate_plan(
        plan: &mut ActionPlan,
        current_serial: &str,
        current_user_id: u32,
        installed_packages: &HashSet<String>,
        active_launcher: Option<String>,
        active_ime: Option<String>,
        intent: Option<ShortcutIntent>,
    ) -> PlanValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut executable_count = 0;
        let mut protected_blocked_count = 0;
        let mut missing_package_count = 0;
        let mut unsafe_package_count = 0;

        // 1. Validate device targeting match
        if plan.target_device_serial != current_serial {
            errors.push(format!(
                "Target device mismatch! Plan was created for '{}', but current connected device is '{}'.",
                plan.target_device_serial, current_serial
            ));
        }

        // 2. Validate Android user/profile targeting match
        if plan.target_user_id != current_user_id {
            errors.push(format!(
                "Target profile mismatch! Plan was created for User {}, but current selected profile is User {}.",
                plan.target_user_id, current_user_id
            ));
        }

        let mut policy = SafetyPolicy::new();
        policy.set_active_roles(active_launcher, active_ime);
        let catalog = get_catalog();

        // 3. Validate and sanitize each proposed action
        let mut seen = HashSet::new();
        for action in &mut plan.actions {
            if !action.selected {
                continue;
            }
            if !seen.insert(action.package_id.clone()) {
                errors.push(format!(
                    "Duplicate selected action for {}",
                    action.package_id
                ));
                action.is_executable = false;
                continue;
            }
            // A. Validate identifier syntax
            if !is_valid_package_name(&action.package_id) {
                errors.push(format!(
                    "Invalid package identifier syntax: '{}'.",
                    action.package_id
                ));
                action.selected = false;
                action.is_executable = false;
                continue;
            }

            // B. Recompute recommendation category from trusted local metadata
            if let Some(meta) = catalog.get(&action.package_id) {
                action.removal_category = meta.removal.clone();
            } else {
                action.removal_category = "Unlisted".to_string();
            }

            // C. Validate package presence on target device
            // Missing packages or already-removed packages MUST NOT remain executable!
            if !installed_packages.contains(&action.package_id) {
                errors.push(format!(
                    "Package '{}' is not currently installed for User {}. It cannot be executed.",
                    action.package_id, current_user_id
                ));
                action.selected = false;
                action.is_executable = false;
                missing_package_count += 1;
                continue;
            }

            // D. Enforce intent scope if an explicit shortcut intent is active
            if let Some(int) = intent {
                match int {
                    ShortcutIntent::ReviewTrackers => {
                        if !AiClient::is_tracker_or_analytics(&action.package_id) {
                            errors.push(format!(
                                "Package '{}' is not a tracking or analytics service (outside Review Trackers intent scope).",
                                action.package_id
                            ));
                            action.selected = false;
                            action.is_executable = false;
                            continue;
                        }
                    }
                    ShortcutIntent::UninstallPromos => {
                        if action.proposed_action != ActionType::UninstallUser {
                            errors.push(format!(
                                "Action '{:?}' on package '{}' is rejected: Only UninstallUser is allowed under Uninstall Promos intent.",
                                action.proposed_action, action.package_id
                            ));
                            action.selected = false;
                            action.is_executable = false;
                            continue;
                        }
                        if !AiClient::is_promotional(&action.package_id) {
                            errors.push(format!(
                                "Package '{}' is not an eligible promotional package (outside Uninstall Promos intent scope).",
                                action.package_id
                            ));
                            action.selected = false;
                            action.is_executable = false;
                            continue;
                        }
                    }
                    ShortcutIntent::ReviewBloatware => {
                        // Bloatware intent allows recommended debloat packages
                        if SafetyPolicy::is_built_in_utility(&action.package_id) {
                            errors.push(format!(
                                "Package '{}' is a built-in utility and is excluded from automated debloat plans. It requires separate explicit review.",
                                action.package_id
                            ));
                            action.selected = false;
                            action.is_executable = false;
                            continue;
                        }
                        if SafetyPolicy::is_uncertain_vendor_package(&action.package_id) {
                            errors.push(format!(
                                "Package '{}' is a MediaTek vendor package with uncertain role and is excluded from automated debloat plans.",
                                action.package_id
                            ));
                            action.selected = false;
                            action.is_executable = false;
                            continue;
                        }
                    }
                }
            } else {
                // Non-shortcut / custom plans: warn on built-in utilities and uncertain vendor packages
                if SafetyPolicy::is_built_in_utility(&action.package_id) {
                    warnings.push(format!(
                        "REVIEW REQUIRED: '{}' is a ROM built-in utility. Modifying it requires separate explicit review.",
                        action.package_id
                    ));
                }
                if SafetyPolicy::is_uncertain_vendor_package(&action.package_id) {
                    warnings.push(format!(
                        "REVIEW REQUIRED: '{}' is a MediaTek vendor component with uncertain role. A prefix or catalog note alone does not prove safety.",
                        action.package_id
                    ));
                }
            }

            // E. Action-aware safety policy check
            if let Err(msg) = policy.is_action_allowed(&action.package_id, action.proposed_action) {
                errors.push(msg);
                action.selected = false;
                action.is_executable = false;
                action.is_protected = true;
                protected_blocked_count += 1;
                continue;
            }

            // F. Handle Unsafe and Unlisted packages conservatively:
            // Do NOT allow automatic selection of unknown or unsafe packages!
            if action.removal_category.eq_ignore_ascii_case("unsafe") {
                warnings.push(format!(
                    "CAUTION: '{}' is categorized as Unsafe in the catalog. It is unselected by default.",
                    action.package_id
                ));
                action.selected = false; // Block automatic selection
                unsafe_package_count += 1;
            } else if action.removal_category.eq_ignore_ascii_case("unlisted") {
                warnings.push(format!(
                    "Package '{}' is unlisted in community datasets. Automatic execution is withheld for review.",
                    action.package_id
                ));
                action.selected = false; // Block automatic selection
            }

            if action.uncertainty.eq_ignore_ascii_case("high") {
                warnings.push(format!(
                    "High uncertainty flagged for '{}': verify side effects before confirming.",
                    action.package_id
                ));
            }

            action.is_executable = true;
            if action.selected {
                executable_count += 1;
            }
        }

        let is_valid = errors.is_empty();
        plan.is_valid = is_valid;
        if !is_valid {
            plan.invalidation_reason = Some(errors.join("; "));
            plan.approved_by_user = false;
        }

        PlanValidationResult {
            is_valid,
            errors,
            warnings,
            executable_count,
            protected_blocked_count,
            missing_package_count,
            unsafe_package_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb::operation::ActionType;
    use crate::ai::types::ProposedAction;

    fn make_test_plan(serial: &str, user_id: u32, packages: Vec<(&str, ActionType)>) -> ActionPlan {
        let actions = packages
            .into_iter()
            .map(|(pkg, act)| ProposedAction {
                package_id: pkg.to_string(),
                package_name: None,
                proposed_action: act,
                reason: "Test".to_string(),
                evidence: "Catalog".to_string(),
                expected_side_effects: None,
                uncertainty: "Low".to_string(),
                removal_category: "Recommended".to_string(),
                is_protected: false,
                protection_warning: None,
                selected: true,
                is_executable: true,
            })
            .collect();

        ActionPlan {
            id: "plan-1".to_string(),
            revision: 1,
            target_device_serial: serial.to_string(),
            target_user_id: user_id,
            explanation: "Test debloat".to_string(),
            preserved_functionality: vec!["camera".into()],
            actions,
            created_at: 1000,
            approved_by_user: true,
            is_valid: true,
            invalidation_reason: None,
        }
    }

    #[test]
    fn test_valid_plan_passes() {
        let mut installed = HashSet::new();
        installed.insert("com.facebook.katana".to_string());

        let mut plan = make_test_plan(
            "serial123",
            0,
            vec![("com.facebook.katana", ActionType::Disable)],
        );
        let result =
            PlanValidator::validate_plan(&mut plan, "serial123", 0, &installed, None, None, None);

        assert!(result.is_valid);
        assert_eq!(result.executable_count, 1);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_device_mismatch_blocks_plan() {
        let mut installed = HashSet::new();
        installed.insert("com.facebook.katana".to_string());

        let mut plan = make_test_plan(
            "serial_A",
            0,
            vec![("com.facebook.katana", ActionType::Disable)],
        );
        let result = PlanValidator::validate_plan(&mut plan, "serial_B", 0, &installed, None, None, None);

        assert!(!result.is_valid);
        assert!(result.errors[0].contains("Target device mismatch"));
    }

    #[test]
    fn test_missing_package_blocked() {
        let installed = HashSet::new(); // empty

        let mut plan = make_test_plan(
            "serial123",
            0,
            vec![("com.nonexistent.app", ActionType::Disable)],
        );
        let result =
            PlanValidator::validate_plan(&mut plan, "serial123", 0, &installed, None, None, None);

        assert!(!result.is_valid);
        assert!(result.errors[0].contains("not currently installed for User 0"));
        assert_eq!(plan.actions[0].is_executable, false);
    }

    #[test]
    fn test_protected_package_blocked() {
        let mut installed = HashSet::new();
        installed.insert("com.android.systemui".to_string());

        let mut plan = make_test_plan(
            "serial123",
            0,
            vec![("com.android.systemui", ActionType::Disable)],
        );
        let result =
            PlanValidator::validate_plan(&mut plan, "serial123", 0, &installed, None, None, None);

        assert!(!result.is_valid);
        assert!(result.errors[0].contains("BLOCKED BY SAFETY POLICY"));
        assert_eq!(plan.actions[0].is_executable, false);
    }

    #[test]
    fn test_tracker_intent_enforces_scope() {
        let mut installed = HashSet::new();
        installed.insert("com.facebook.katana".to_string());
        installed.insert("com.miui.analytics".to_string());

        // Katana is not a tracker, should be blocked under ReviewTrackers intent
        let mut plan = make_test_plan(
            "serial123",
            0,
            vec![
                ("com.facebook.katana", ActionType::UninstallUser),
                ("com.miui.analytics", ActionType::UninstallUser),
            ],
        );
        let result = PlanValidator::validate_plan(
            &mut plan,
            "serial123",
            0,
            &installed,
            None,
            None,
            Some(ShortcutIntent::ReviewTrackers),
        );

        assert!(!result.is_valid);
        assert_eq!(plan.actions[0].is_executable, false); // Katana blocked
        assert!(result.errors.iter().any(|e| e.contains("outside Review Trackers intent scope")));
    }

    #[test]
    fn test_promo_intent_enforces_action_type() {
        let mut installed = HashSet::new();
        installed.insert("com.facebook.katana".to_string());

        // Disable action on promo should be rejected under UninstallPromos intent
        let mut plan = make_test_plan(
            "serial123",
            0,
            vec![("com.facebook.katana", ActionType::Disable)],
        );
        let result = PlanValidator::validate_plan(
            &mut plan,
            "serial123",
            0,
            &installed,
            None,
            None,
            Some(ShortcutIntent::UninstallPromos),
        );

        assert!(!result.is_valid);
        assert_eq!(plan.actions[0].is_executable, false);
        assert!(result.errors.iter().any(|e| e.contains("Only UninstallUser is allowed")));
    }
}
