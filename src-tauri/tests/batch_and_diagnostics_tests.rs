use tz_ai_debloater_lib::{
    adb::{ActionType, ClassifiedCause, OperationExecutor, PackageState},
    ai::types::{ActionPlan, ProposedAction},
};

#[test]
fn test_classify_xiaomi_miui_permission_denial() {
    let stdout = "java.lang.SecurityException: Permission Denial: attempt to change component state from pid=12345, uid=2000, requires android.permission.CHANGE_COMPONENT_ENABLED_STATE";
    let (cause, message) = OperationExecutor::classify_error(stdout, "", Some(1), ActionType::Disable);

    match cause {
        ClassifiedCause::MissingPrivileges { details, remedy } => {
            assert!(details.contains("CHANGE_COMPONENT_ENABLED_STATE"));
            assert!(remedy.contains("USB debugging (Security settings)"));
            assert!(remedy.contains("Uninstall (User)"));
        }
        _ => panic!("Expected MissingPrivileges, got: {:?}", cause),
    }

    assert!(message.contains("USB debugging (Security settings)"));
    assert!(message.contains("Uninstall for User"));
}

#[test]
fn test_classify_knox_and_device_policy_restrictions() {
    let stdout = "Failure [DELETE_FAILED_DEVICE_POLICY_MANAGER]";
    let (cause, message) = OperationExecutor::classify_error(stdout, "", Some(1), ActionType::UninstallUser);

    match cause {
        ClassifiedCause::DevicePolicyRestricted { details } => {
            assert!(details.contains("DELETE_FAILED_DEVICE_POLICY_MANAGER"));
        }
        _ => panic!("Expected DevicePolicyRestricted, got: {:?}", cause),
    }
    assert!(message.contains("Device Policy Manager"));
}

#[test]
fn test_classify_protected_system_package() {
    let stdout = "Error: Cannot disable a protected package: com.android.phone";
    let (cause, message) = OperationExecutor::classify_error(stdout, "", Some(1), ActionType::Disable);

    match cause {
        ClassifiedCause::ApplicationPolicyBlock { details } => {
            assert!(details.contains("protected"));
        }
        _ => panic!("Expected ApplicationPolicyBlock, got: {:?}", cause),
    }
    assert!(message.contains("protects this package"));
}

#[test]
fn test_classify_unknown_or_missing_package() {
    let stdout = "Failure [not installed for 0]";
    let (cause, message) = OperationExecutor::classify_error(stdout, "", Some(1), ActionType::UninstallUser);

    match cause {
        ClassifiedCause::PackageNotFound { .. } => {}
        _ => panic!("Expected PackageNotFound, got: {:?}", cause),
    }
    assert!(message.contains("not installed for this user profile"));
}

#[test]
fn test_classify_device_offline() {
    let stderr = "error: device offline";
    let (cause, message) = OperationExecutor::classify_error("", stderr, Some(1), ActionType::Disable);

    assert_eq!(cause, ClassifiedCause::DeviceOffline);
    assert!(message.contains("Device went offline"));
}

#[test]
fn test_action_type_wanted_states() {
    assert_eq!(ActionType::UninstallUser.wanted_state(), Some(PackageState::Uninstalled));
    assert_eq!(ActionType::Disable.wanted_state(), Some(PackageState::Disabled));
    assert_eq!(ActionType::Enable.wanted_state(), Some(PackageState::Enabled));
    // Restore verifies presence, does not mandate enabled vs disabled
    assert_eq!(ActionType::Restore.wanted_state(), None);
}

#[test]
fn test_retry_plan_filtering() {
    let plan = ActionPlan {
        id: "plan-original-123".into(),
        revision: 1,
        target_device_serial: "SERIAL123".into(),
        target_user_id: 0,
        explanation: "Original plan".into(),
        preserved_functionality: vec!["Calls".into()],
        actions: vec![
            ProposedAction {
                package_id: "com.pkg.succeeded".into(),
                package_name: None,
                proposed_action: ActionType::UninstallUser,
                reason: "bloat".into(),
                evidence: "vendor".into(),
                expected_side_effects: None,
                uncertainty: "low".into(),
                removal_category: "Recommended".into(),
                is_protected: false,
                protection_warning: None,
                selected: true,
                is_executable: true,
            },
            ProposedAction {
                package_id: "com.pkg.failed".into(),
                package_name: None,
                proposed_action: ActionType::Disable,
                reason: "bloat".into(),
                evidence: "vendor".into(),
                expected_side_effects: None,
                uncertainty: "low".into(),
                removal_category: "Recommended".into(),
                is_protected: false,
                protection_warning: None,
                selected: true,
                is_executable: true,
            },
            ProposedAction {
                package_id: "com.pkg.skipped".into(),
                package_name: None,
                proposed_action: ActionType::UninstallUser,
                reason: "bloat".into(),
                evidence: "vendor".into(),
                expected_side_effects: None,
                uncertainty: "low".into(),
                removal_category: "Recommended".into(),
                is_protected: false,
                protection_warning: None,
                selected: true,
                is_executable: true,
            },
        ],
        created_at: 1000,
        approved_by_user: true,
        is_valid: true,
        invalidation_reason: None,
    };

    let remaining_ids = vec!["com.pkg.failed".to_string(), "com.pkg.skipped".to_string()];
    let mut retry_actions = Vec::new();
    let id_set: std::collections::HashSet<String> = remaining_ids.into_iter().collect();

    for act in plan.actions {
        if id_set.contains(&act.package_id) {
            retry_actions.push(act);
        }
    }

    assert_eq!(retry_actions.len(), 2);
    assert_eq!(retry_actions[0].package_id, "com.pkg.failed");
    assert_eq!(retry_actions[1].package_id, "com.pkg.skipped");
}

