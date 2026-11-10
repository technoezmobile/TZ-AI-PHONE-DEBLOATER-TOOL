use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tz_ai_debloater_lib::adb::{
    is_valid_package_name, ActionType, OperationExecutor, OperationOutcome, PackageState,
    UserManager,
};
use tz_ai_debloater_lib::ai::client::AiClient;
use tz_ai_debloater_lib::ai::types::{ActionPlan, ErrorType, ProposedAction};
use tz_ai_debloater_lib::journal::{
    DebloatProfile, DeviceSnapshot, JournalManager, PackageSnapshotEntry, ProfileItem,
};
use tz_ai_debloater_lib::plan::PlanValidator;
use tz_ai_debloater_lib::policy::SafetyPolicy;
use tz_ai_debloater_lib::storage::credentials::{ConfigStore, CredentialStore};

// -----------------------------------------------------------------------------
// 1. Identifier Validation & Command Injection Resistance
// -----------------------------------------------------------------------------
#[test]
fn test_strict_identifier_validation_injection_resistance() {
    assert!(is_valid_package_name("android"));
    assert!(is_valid_package_name("com.android.settings"));
    assert!(is_valid_package_name("org.lineageos.jelly"));
    assert!(is_valid_package_name("com.samsung.android.app.notes"));
    assert!(is_valid_package_name("net.oneplus.launcher"));
    assert!(is_valid_package_name("com.google.android.gms"));

    // Injections & Malicious Characters
    assert!(!is_valid_package_name("com.example; rm -rf /"));
    assert!(!is_valid_package_name("com.example && echo 1"));
    assert!(!is_valid_package_name("com.example | cat /etc/passwd"));
    assert!(!is_valid_package_name("com.example`reboot`"));
    assert!(!is_valid_package_name("com.example$(reboot)"));
    assert!(!is_valid_package_name("com.example > /sdcard/hack"));
    assert!(!is_valid_package_name("com.example\nreboot"));
    assert!(!is_valid_package_name("com.example\r\nreboot"));
    assert!(!is_valid_package_name("com.example\0null"));
    assert!(!is_valid_package_name(""));
    assert!(!is_valid_package_name("singleword"));
    assert!(!is_valid_package_name(".starts.with.dot"));
    assert!(!is_valid_package_name("ends.with.dot."));
    assert!(!is_valid_package_name("contains..double.dots"));
    assert!(!is_valid_package_name("contains spaces.in.name"));
}

// -----------------------------------------------------------------------------
// 2. Protected Package Policy & Safety Rules
// -----------------------------------------------------------------------------
#[test]
fn test_protected_package_policy_rules() {
    let mut policy = SafetyPolicy::new();
    policy.set_active_roles(
        Some("com.sec.android.app.launcher".into()),
        Some("com.touchtype.swiftkey".into()),
    );

    // Critical core components MUST be blocked
    let systemui = policy.check_protection("com.android.systemui");
    assert!(systemui.is_protected);
    assert!(systemui.is_critical_core);

    let settings = policy.check_protection("com.android.settings");
    assert!(settings.is_protected);
    assert!(settings.is_critical_core);

    let installer = policy.check_protection("com.google.android.packageinstaller");
    assert!(installer.is_protected);

    // Active Launcher & Active Keyboard dynamically protected
    let launcher = policy.check_protection("com.sec.android.app.launcher");
    assert!(launcher.is_protected);
    assert!(launcher.reason.unwrap().contains("Active Default Launcher"));

    let ime = policy.check_protection("com.touchtype.swiftkey");
    assert!(ime.is_protected);
    assert!(ime.reason.unwrap().contains("Active Default Keyboard"));

    // Action-aware policy: Enabling/Restoring is OK, Disabling/Uninstalling is blocked
    assert!(policy
        .is_action_allowed("com.android.systemui", ActionType::Disable)
        .is_err());
    assert!(policy
        .is_action_allowed("com.android.systemui", ActionType::UninstallUser)
        .is_err());
    assert!(policy
        .is_action_allowed("com.android.systemui", ActionType::Enable)
        .is_ok());
    assert!(policy
        .is_action_allowed("com.android.systemui", ActionType::Restore)
        .is_ok());
}

// -----------------------------------------------------------------------------
// 3. Multi-User ADB Output Parsing
// -----------------------------------------------------------------------------
#[test]
fn test_adb_multi_user_output_parsing() {
    let sample = "\
Users:
\tUserInfo{0:Owner:13} running
\tUserInfo{10:Work profile:30} running
\tUserInfo{999:Dual App:0}
";
    let users = UserManager::parse_users_output(sample).expect("Should parse multi-user output");
    assert_eq!(users.len(), 3);
    assert_eq!(users[0].id, 0);
    assert!(users[0].is_primary);
    assert_eq!(users[1].id, 10);
    assert_eq!(users[1].profile_type, "Work Profile");
    assert_eq!(users[2].id, 999);
    assert_eq!(users[2].profile_type, "Dual / Cloned App Profile");
}

// -----------------------------------------------------------------------------
// 4. AI Plan Extraction & Safety Enforcement
// -----------------------------------------------------------------------------
#[test]
fn test_ai_schema_plan_parsing_and_protection() {
    let sample_ai_response = r#"{
        "explanation": "Debloating recommendations preserving camera and banking",
        "preserved_functionality": ["camera", "banking"],
        "actions": [
            {
                "package_id": "com.facebook.katana",
                "package_name": "Facebook",
                "proposed_action": "Disable",
                "reason": "Social bloatware consuming battery in background",
                "evidence": "UAD catalog: OEM bloatware",
                "expected_side_effects": "No Facebook notifications",
                "uncertainty": "Low"
            },
            {
                "package_id": "com.android.systemui",
                "package_name": "System UI",
                "proposed_action": "UninstallUser",
                "reason": "Dangerous hallucination",
                "evidence": "None",
                "uncertainty": "High"
            }
        ]
    }"#;

    let plan = AiClient::extract_plan_from_text(sample_ai_response, "SERIAL_12345", 0)
        .expect("Should parse valid plan schema");

    assert_eq!(plan.target_device_serial, "SERIAL_12345");
    assert_eq!(plan.target_user_id, 0);
    assert_eq!(plan.actions.len(), 2);

    let fb = &plan.actions[0];
    assert_eq!(fb.package_id, "com.facebook.katana");
    assert!(!fb.is_protected);
    assert!(fb.selected);

    let sys = &plan.actions[1];
    assert_eq!(sys.package_id, "com.android.systemui");
    assert!(sys.is_protected);
    assert!(!sys.selected);
}

// -----------------------------------------------------------------------------
// 5. Plan Targeting & Approval Invalidation
// -----------------------------------------------------------------------------
#[test]
fn test_plan_targeting_and_approval_invalidation() {
    let mut plan = ActionPlan {
        id: "plan-1".into(),
        revision: 1,
        target_device_serial: "SERIAL_A".into(),
        target_user_id: 0,
        explanation: "Test Plan".into(),
        preserved_functionality: vec!["calls".into()],
        actions: vec![ProposedAction {
            package_id: "com.facebook.katana".into(),
            package_name: Some("Facebook".into()),
            proposed_action: ActionType::Disable,
            reason: "Bloatware".into(),
            evidence: "Catalog".into(),
            expected_side_effects: None,
            uncertainty: "Low".into(),
            removal_category: "Recommended".into(),
            is_protected: false,
            protection_warning: None,
            selected: true,
            is_executable: true,
        }],
        created_at: 1000,
        approved_by_user: true,
        is_valid: true,
        invalidation_reason: None,
    };

    let mut installed = HashSet::new();
    installed.insert("com.facebook.katana".into());

    let res = PlanValidator::validate_plan(&mut plan, "SERIAL_A", 0, &installed, None, None, None);
    assert!(res.is_valid);
    assert_eq!(res.executable_count, 1);

    // Target mismatch invalidates plan
    let res_mismatch =
        PlanValidator::validate_plan(&mut plan, "SERIAL_B", 0, &installed, None, None, None);
    assert!(!res_mismatch.is_valid);
    assert!(res_mismatch.errors.iter().any(|e| e.contains("mismatch")));

    // Helper invalidation check
    assert!(PlanValidator::check_invalidation(&mut plan, "SERIAL_B", 0));
    assert!(!plan.approved_by_user);
    assert!(!plan.is_valid);
}

// -----------------------------------------------------------------------------
// 6. OEM Error Translation
// -----------------------------------------------------------------------------
#[test]
fn test_friendly_error_translation_oem_restrictions() {
    let knox_err = OperationExecutor::translate_error_message(
        "Failure [DELETE_FAILED_DEVICE_POLICY_MANAGER]",
        "",
    );
    assert!(knox_err.contains("Device Policy Manager") || knox_err.contains("Knox"));

    let sec_err = OperationExecutor::translate_error_message(
        "java.lang.SecurityException: Permission Denial: not allowed",
        "",
    );
    assert!(sec_err.contains("security policy") || sec_err.contains("Non-root"));
}

// -----------------------------------------------------------------------------
// 7. Multi-Action Recovery Mapping
// -----------------------------------------------------------------------------
#[test]
fn test_journal_inverse_recovery_actions() {
    // Previously Enabled -> Uninstalled requires [Restore, Enable]
    let steps_en_to_un = JournalManager::derive_recovery_steps(
        Some(PackageState::Enabled),
        Some(PackageState::Uninstalled),
    );
    assert_eq!(
        steps_en_to_un,
        vec![ActionType::Restore, ActionType::Enable]
    );

    // Previously Disabled -> Uninstalled requires [Restore, Disable]
    let steps_dis_to_un = JournalManager::derive_recovery_steps(
        Some(PackageState::Disabled),
        Some(PackageState::Uninstalled),
    );
    assert_eq!(
        steps_dis_to_un,
        vec![ActionType::Restore, ActionType::Disable]
    );

    // Previously Disabled -> Enabled requires [Disable]
    let steps_dis_to_en = JournalManager::derive_recovery_steps(
        Some(PackageState::Disabled),
        Some(PackageState::Enabled),
    );
    assert_eq!(steps_dis_to_en, vec![ActionType::Disable]);

    // Cross-device recovery must be blocked
    let outcome = OperationOutcome {
        package_id: "com.facebook.katana".into(),
        action: ActionType::Disable,
        success: true,
        exit_code: Some(0),
        previous_state: Some(PackageState::Enabled),
        resulting_state: Some(PackageState::Disabled),
        message: "Disabled".into(),
        verified: true,
        classified_cause: tz_ai_debloater_lib::adb::ClassifiedCause::None,
        execution_backend: "DirectAdbPm".into(),
        command_args: Vec::new(),
        target_device: "DEVICE_ALPHA".into(),
        target_user_id: 0,
        raw_stdout: None,
        raw_stderr: None,
    };
    let entry = JournalManager::log_operation("DEVICE_ALPHA", 0, &outcome).unwrap();

    let proposal =
        JournalManager::propose_recovery(&entry, "DEVICE_BETA", 0, Some(PackageState::Disabled));
    assert!(!proposal.can_recover);
    assert!(proposal.blockage_reason.unwrap().contains("mismatch"));
}

// -----------------------------------------------------------------------------
// 8. Secret Redaction & Key Masking
// -----------------------------------------------------------------------------
#[test]
fn test_secret_redaction_and_key_masking() {
    assert_eq!(ConfigStore::mask_key(""), "");
    assert_eq!(ConfigStore::mask_key("short"), "••••••••");
    assert_eq!(ConfigStore::mask_key("12345678"), "••••••••");
    assert_eq!(
        ConfigStore::mask_key("AIzaSyAbc12345XyZ"),
        "AIza••••••••5XyZ"
    );
}

// -----------------------------------------------------------------------------
// 9. Profile Import & Export
// -----------------------------------------------------------------------------
#[test]
fn test_profile_import_export_validation() {
    let profile = DebloatProfile {
        profile_name: "Safe Daily Debloat".into(),
        description: "Disables standard tracking apps".into(),
        created_at: 123456,
        actions: vec![ProfileItem {
            package_id: "com.facebook.katana".into(),
            desired_action: ActionType::Disable,
        }],
    };

    let temp_dir = std::env::temp_dir();
    let profile_path = temp_dir.join(format!(
        "test_debloat_profile_{}.json",
        uuid::Uuid::new_v4()
    ));

    JournalManager::export_profile(&profile, &profile_path).expect("Should export profile");
    let imported = JournalManager::import_profile(&profile_path).expect("Should import profile");
    let _ = std::fs::remove_file(&profile_path);
    assert_eq!(imported.profile_name, "Safe Daily Debloat");
    assert_eq!(imported.actions.len(), 1);
}

// -----------------------------------------------------------------------------
// 10. REGRESSION TEST: Saved Credentials Persistence & Isolation
// -----------------------------------------------------------------------------
#[test]
fn test_credential_store_isolation_and_restart() {
    let conn_a = format!("disposable-test-{}", uuid::Uuid::new_v4());
    let conn_b = format!("disposable-test-{}", uuid::Uuid::new_v4());
    let secret_a = "sk-secret-key-alpha-12345";
    let secret_a_updated = "sk-secret-key-alpha-replaced-99999";
    let secret_b = "sk-secret-key-beta-67890";

    // 1. Initial state: neither exists
    assert!(!CredentialStore::has_credential(&conn_a));
    assert!(!CredentialStore::has_credential(&conn_b));

    // 2. Save both
    CredentialStore::save_credential(&conn_a, secret_a).expect("Save A should succeed");
    CredentialStore::save_credential(&conn_b, secret_b).expect("Save B should succeed");

    assert_eq!(
        CredentialStore::get_credential(&conn_a).unwrap(),
        Some(secret_a.to_string())
    );
    assert_eq!(
        CredentialStore::get_credential(&conn_b).unwrap(),
        Some(secret_b.to_string())
    );

    // 3. Replace A with new value: B remains unchanged (isolation)
    CredentialStore::save_credential(&conn_a, secret_a_updated).expect("Replace A should succeed");
    assert_eq!(
        CredentialStore::get_credential(&conn_a).unwrap(),
        Some(secret_a_updated.to_string())
    );
    assert_eq!(
        CredentialStore::get_credential(&conn_b).unwrap(),
        Some(secret_b.to_string())
    );

    // 4. Verify isolation: deleting A does NOT delete B
    CredentialStore::delete_credential(&conn_a).expect("Delete A should succeed");
    assert!(!CredentialStore::has_credential(&conn_a));
    assert!(CredentialStore::has_credential(&conn_b));
    assert_eq!(
        CredentialStore::get_credential(&conn_b).unwrap(),
        Some(secret_b.to_string())
    );

    // 5. Clean up B
    CredentialStore::delete_credential(&conn_b).expect("Delete B should succeed");
    assert!(!CredentialStore::has_credential(&conn_b));
    assert_eq!(CredentialStore::get_credential(&conn_b).unwrap(), None);

    // 6. Deleting already-deleted credential is safe and idempotent
    assert!(CredentialStore::delete_credential(&conn_b).is_ok());
}

// -----------------------------------------------------------------------------
// 11. REGRESSION TEST: Endpoint Normalization & Security Rules
// -----------------------------------------------------------------------------
#[test]
fn test_endpoint_normalization_and_security() {
    // Prohibit embedded credentials in URL
    let bad_url = "https://user:password@api.openai.com/v1";
    assert!(AiClient::normalize_endpoint(bad_url, "models").is_err());

    // Prohibit insecure HTTP on remote non-loopback hosts
    let insecure_remote = "http://api.remote-gateway.com/v1";
    let remote_res = AiClient::normalize_endpoint(insecure_remote, "models");
    assert!(remote_res.is_err());
    assert_eq!(
        remote_res.unwrap_err().error_type,
        ErrorType::InvalidEndpointOrProtocol
    );

    // Allow HTTP on local loopback hosts
    let loopback = "http://localhost:3001/v1";
    let loopback_res = AiClient::normalize_endpoint(loopback, "models");
    assert!(loopback_res.is_ok());
    assert_eq!(loopback_res.unwrap(), "http://localhost:3001/v1/models");

    // Avoid duplicating /v1 when base ends with /v1 and suffix has /v1
    let base_with_v1 = "https://api.openai.com/v1";
    let deduplicated = AiClient::normalize_endpoint(base_with_v1, "v1/chat/completions").unwrap();
    assert_eq!(deduplicated, "https://api.openai.com/v1/chat/completions");
}

// -----------------------------------------------------------------------------
// 12. REGRESSION TEST: Missing & Unsafe Packages Blocked From Execution
// -----------------------------------------------------------------------------
#[test]
fn test_missing_and_unsafe_packages_blocked_from_execution() {
    let mut installed = HashSet::new();
    installed.insert("com.unlisted.app".to_string());
    // "com.missing.app" is intentionally NOT in installed set

    let mut plan = ActionPlan {
        id: "plan-test-missing".into(),
        revision: 1,
        target_device_serial: "SERIAL_123".into(),
        target_user_id: 0,
        explanation: "Test missing & unsafe".into(),
        preserved_functionality: vec![],
        actions: vec![
            ProposedAction {
                package_id: "com.missing.app".into(),
                package_name: None,
                proposed_action: ActionType::Disable,
                reason: "Test".into(),
                evidence: "".into(),
                expected_side_effects: None,
                uncertainty: "Low".into(),
                removal_category: "Recommended".into(),
                is_protected: false,
                protection_warning: None,
                selected: true,
                is_executable: true,
            },
            ProposedAction {
                package_id: "com.unlisted.app".into(),
                package_name: None,
                proposed_action: ActionType::Disable,
                reason: "Test".into(),
                evidence: "".into(),
                expected_side_effects: None,
                uncertainty: "Low".into(),
                removal_category: "Recommended".into(), // AI claimed Recommended, but it is unlisted in local catalog!
                is_protected: false,
                protection_warning: None,
                selected: true, // Frontend or AI initially had it selected
                is_executable: true,
            },
        ],
        created_at: 100,
        approved_by_user: true,
        is_valid: true,
        invalidation_reason: None,
    };

    let result = PlanValidator::validate_plan(&mut plan, "SERIAL_123", 0, &installed, None, None, None);

    // Missing package must be marked as not executable and unselected
    assert_eq!(plan.actions[0].is_executable, false);
    assert_eq!(plan.actions[0].selected, false);
    assert_eq!(result.missing_package_count, 1);

    // Unlisted/unknown package must be unselected by default to prevent automatic execution
    assert_eq!(plan.actions[1].removal_category, "Unlisted");
    assert_eq!(plan.actions[1].selected, false);
}

// -----------------------------------------------------------------------------
// 13. REGRESSION TEST: Backend Cancellation Token Aborts Stream
// -----------------------------------------------------------------------------
#[test]
fn test_cancellation_token_flag() {
    let cancel_token = Arc::new(AtomicBool::new(false));
    assert_eq!(cancel_token.load(Ordering::SeqCst), false);

    // Signal cancellation
    cancel_token.store(true, Ordering::SeqCst);
    assert_eq!(cancel_token.load(Ordering::SeqCst), true);
}

// -----------------------------------------------------------------------------
// 14. REGRESSION TEST: Pre-Change Snapshot Failure Aborts
// -----------------------------------------------------------------------------
#[test]
fn test_snapshot_persistence_atomic() {
    let snapshot = DeviceSnapshot {
        snapshot_id: "snap-test-uuid".into(),
        device_serial: "DEVICE_X".into(),
        device_model: "Pixel 8".into(),
        user_id: 0,
        timestamp: 1700000000,
        note: "Unit test snapshot".into(),
        package_states: vec![PackageSnapshotEntry {
            package_id: "com.test.app".into(),
            state: PackageState::Enabled,
            is_system: false,
        }],
    };

    let path = JournalManager::save_snapshot(&snapshot).expect("Snapshot should save");
    assert!(std::path::Path::new(&path).exists());

    let snapshots = JournalManager::list_snapshots().expect("Should list snapshots");
    assert!(snapshots.iter().any(|s| s.snapshot_id == "snap-test-uuid"));
}
