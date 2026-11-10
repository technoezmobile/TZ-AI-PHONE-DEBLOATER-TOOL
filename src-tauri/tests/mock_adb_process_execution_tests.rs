use std::path::PathBuf;
use std::time::Duration;
use tz_ai_debloater_lib::adb::{
    ActionType, AdbExecutor, DeviceManager, OperationExecutor, UserManager,
};

fn get_mock_adb_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mock_adb"))
}

#[test]
fn test_real_mock_adb_process_execution() {
    let adb_path = get_mock_adb_path();
    assert!(
        adb_path.exists(),
        "mock_adb.exe must be built before running test: {}",
        adb_path.display()
    );

    let executor = AdbExecutor::new(&adb_path);
    let res = executor
        .run_raw(&["version"])
        .expect("Must run mock ADB binary");
    assert!(res.success);
    assert!(res.stdout.contains("Android Debug Bridge version 1.0.41"));
}

#[test]
fn test_real_mock_adb_devices_and_properties() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    let devices = DeviceManager::list_devices(&executor).expect("Must list devices from mock adb");
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].serial, "MOCK_SERIAL_123");
    assert_eq!(devices[0].model, "Pixel 8");
    assert_eq!(devices[0].brand, "Google");
    assert_eq!(devices[0].android_version, "14");
    assert_eq!(devices[0].android_sdk, 34);
}

#[test]
fn test_real_mock_adb_timeout_and_process_reaping() {
    let adb_path = get_mock_adb_path();
    // Configure strict 1-second timeout
    let executor = AdbExecutor::new(&adb_path).with_timeout(Duration::from_millis(800));

    let start = std::time::Instant::now();
    // sleep_long in mock_adb sleeps for 10 seconds
    let res = executor.run_raw(&["sleep_long"]);
    let elapsed = start.elapsed();

    // Must return an error due to timeout
    assert!(res.is_err(), "Command should have timed out");
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("timed out"),
        "Error should explain timeout: {}",
        err_msg
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "Timeout must be enforced well before 10s: took {:?}",
        elapsed
    );
}

#[test]
fn test_real_mock_adb_no_uninstall_fallback_dropping_k() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // mock_adb returns failure for "pm uninstall -k"
    // and returns "UNINSTALL_WITHOUT_K_CALLED" if fallback without -k is invoked
    let outcome = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_123",
        0,
        "com.facebook.katana",
        ActionType::UninstallUser,
    )
    .expect("execute_action should complete");

    // The operation must NOT succeed because -k failed
    assert_eq!(outcome.success, false);
    // Verified that it did NOT call uninstall without -k!
    assert!(!outcome.message.contains("UNINSTALL_WITHOUT_K_CALLED"));
}

#[test]
fn test_real_mock_adb_force_stop_targets_user_profile() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // Disable action also triggers force-stop: mock_adb fails with exit code 2 if --user is missing
    let outcome = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_123",
        0,
        "com.facebook.katana",
        ActionType::Disable,
    )
    .expect("execute_action should execute");

    assert!(outcome.success);
}

#[test]
fn test_real_mock_adb_malformed_user_output_does_not_assume_user_0() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // MALFORMED triggers corrupt output in mock_adb
    let res = UserManager::list_users(&executor, "MALFORMED");
    assert!(
        res.is_err(),
        "Must reject malformed user discovery output rather than defaulting to user 0"
    );
    let err = res.unwrap_err();
    assert!(err.contains("No Android user profiles could be enumerated"));
}

#[test]
fn test_simultaneous_stderr_and_stdout_do_not_deadlock() {
    let e = AdbExecutor::new(get_mock_adb_path()).with_timeout(Duration::from_secs(3));
    let r = e.run_raw(&["stderr_flood"]).unwrap();
    assert!(r.success);
    assert!(r.stdout.contains("drained"));
    assert!(r.stderr.len() > 65536);
}

// -----------------------------------------------------------------------------
// Hardened Package Discovery Tests (Android errors with exit code 0)
// -----------------------------------------------------------------------------

#[test]
fn test_real_mock_adb_rejects_security_exception_with_exit_code_zero() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // SECURITY_ERROR flag in serial causes mock_adb to return exit 0 but output SecurityException
    let res = tz_ai_debloater_lib::adb::PackageManager::list_packages_with_flags(
        &executor,
        "SECURITY_ERROR",
        10,
        Some("-u"),
    );

    assert!(
        res.is_err(),
        "Must reject SecurityException even when exit code is 0"
    );
    let err = res.unwrap_err();
    assert!(
        err.contains("SecurityException") || err.contains("Permission Denial"),
        "Error message must mention SecurityException: {err}"
    );
}

#[test]
fn test_real_mock_adb_rejects_manager_error_with_exit_code_zero() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // MANAGER_ERROR flag in serial causes mock_adb to return exit 0 with Error: Could not access package manager
    let res = tz_ai_debloater_lib::adb::PackageManager::list_packages_with_flags(
        &executor,
        "MANAGER_ERROR",
        0,
        Some("-u"),
    );

    assert!(res.is_err(), "Must reject manager error with exit code 0");
    let err = res.unwrap_err();
    assert!(err.contains("Could not access package manager"));
}

#[test]
fn test_real_mock_adb_rejects_malformed_package_output() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    let res = tz_ai_debloater_lib::adb::PackageManager::list_packages_with_flags(
        &executor,
        "MALFORMED_PACKAGES",
        0,
        Some("-u"),
    );

    assert!(res.is_err(), "Must reject malformed non-package output");
    let err = res.unwrap_err();
    assert!(err.contains("Malformed package list output"));
}

// -----------------------------------------------------------------------------
// Recovery State Separation: Installation vs Enabled State Tests
// -----------------------------------------------------------------------------

#[test]
fn test_real_mock_adb_recovery_previously_disabled() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // Step 1: Restore system app (verifying installation, not enforcing Enabled)
    let outcome_restore = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_REC_DIS",
        0,
        "com.facebook.katana",
        ActionType::Restore,
    )
    .expect("Restore execution should succeed");

    assert!(outcome_restore.success);
    assert!(outcome_restore.verified, "Restore must verify installation");

    // Step 2: Disable system app to restore desired Disabled state
    let outcome_disable = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_REC_DIS",
        0,
        "com.facebook.katana",
        ActionType::Disable,
    )
    .expect("Disable execution should succeed");

    assert!(outcome_disable.success);
    assert!(outcome_disable.verified);
    assert_eq!(
        outcome_disable.resulting_state,
        Some(tz_ai_debloater_lib::adb::PackageState::Disabled)
    );
}

#[test]
fn test_real_mock_adb_recovery_previously_enabled() {
    let adb_path = get_mock_adb_path();
    let executor = AdbExecutor::new(&adb_path);

    // Step 1: Restore system app
    let outcome_restore = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_REC_EN",
        0,
        "com.facebook.katana",
        ActionType::Restore,
    )
    .expect("Restore execution should succeed");

    assert!(outcome_restore.success);
    assert!(outcome_restore.verified);

    // Step 2: Enable to verify requested final Enabled state
    let outcome_enable = OperationExecutor::execute_action(
        &executor,
        "MOCK_SERIAL_REC_EN",
        0,
        "com.facebook.katana",
        ActionType::Enable,
    )
    .expect("Enable execution should succeed");

    assert!(outcome_enable.success);
    assert!(outcome_enable.verified);
    assert_eq!(
        outcome_enable.resulting_state,
        Some(tz_ai_debloater_lib::adb::PackageState::Enabled)
    );
}
