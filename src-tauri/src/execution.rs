//! One policy and persistence boundary for manual, plan, and recovery mutations.
use crate::adb::{
    ActionType, AdbExecutor, DeviceManager, OperationExecutor, OperationOutcome, PackageManager,
    UserManager,
};
use crate::journal::{DeviceSnapshot, JournalManager, PackageSnapshotEntry};
use crate::policy::SafetyPolicy;

pub fn run_recorded(
    executor: &AdbExecutor,
    serial: &str,
    user: u32,
    package: &str,
    action: ActionType,
) -> Result<OperationOutcome, String> {
    let _device_lock = crate::storage::CrossProcessLock::lock_device(serial)?;

    if !crate::adb::is_valid_package_name(package) {
        return Err("Invalid package identifier".into());
    }
    if !UserManager::list_users(executor, serial)?
        .iter()
        .any(|u| u.id == user)
    {
        return Err("Selected Android profile does not exist or is inaccessible".into());
    }
    let packages = PackageManager::scan_device_packages(executor, serial, user)?;
    let previous = packages
        .iter()
        .find(|p| p.package_id == package)
        .ok_or("Package is absent from target inventory")?;

    // Gracefully handle redundant requests where package is already in desired state
    if action == ActionType::UninstallUser && previous.state == crate::adb::PackageState::Uninstalled {
        return Ok(OperationOutcome {
            package_id: package.into(),
            action,
            success: true,
            verified: true,
            exit_code: Some(0),
            previous_state: Some(previous.state),
            resulting_state: Some(previous.state),
            message: format!(
                "Package '{package}' is already uninstalled for User {user} (redundant uninstall skipped)."
            ),
            classified_cause: crate::adb::ClassifiedCause::None,
            execution_backend: "DirectAdbPm".to_string(),
            command_args: Vec::new(),
            target_device: serial.into(),
            target_user_id: user,
            raw_stdout: Some("Already uninstalled".into()),
            raw_stderr: None,
        });
    }

    if action == ActionType::Disable && previous.state == crate::adb::PackageState::Disabled {
        return Ok(OperationOutcome {
            package_id: package.into(),
            action,
            success: true,
            verified: true,
            exit_code: Some(0),
            previous_state: Some(previous.state),
            resulting_state: Some(previous.state),
            message: format!(
                "Package '{package}' is already disabled for User {user} (redundant disable skipped)."
            ),
            classified_cause: crate::adb::ClassifiedCause::None,
            execution_backend: "DirectAdbPm".to_string(),
            command_args: Vec::new(),
            target_device: serial.into(),
            target_user_id: user,
            raw_stdout: Some("Already disabled".into()),
            raw_stderr: None,
        });
    }

    let destructive = matches!(action, ActionType::Disable | ActionType::UninstallUser);
    let launcher = DeviceManager::get_active_launcher_for_user(executor, serial, user);
    let ime = DeviceManager::get_active_ime_for_user(executor, serial, user);
    if destructive && (launcher.is_none() || ime.is_none()) {
        return Err("Cannot verify the selected profile's launcher and keyboard. Removal is blocked; enabling/restoring remains available.".into());
    }
    let mut policy = SafetyPolicy::new();
    policy.set_active_roles(launcher, ime);
    policy.is_action_allowed(package, action)?;
    if destructive
        && previous
            .removal_recommendation
            .eq_ignore_ascii_case("unsafe")
    {
        return Err(
            "This package is marked Unsafe in the local catalog. Removal is blocked.".into(),
        );
    }
    JournalManager::save_snapshot(&DeviceSnapshot {
        snapshot_id: uuid::Uuid::new_v4().to_string(),
        device_serial: serial.into(),
        device_model: "Android device".into(),
        user_id: user,
        timestamp: chrono::Utc::now().timestamp(),
        note: format!("Before {} {}", action.display_name(), package),
        package_states: packages
            .iter()
            .map(|p| PackageSnapshotEntry {
                package_id: p.package_id.clone(),
                state: p.state,
                is_system: p.is_system,
            })
            .collect(),
    })?;
    let pending = OperationOutcome {
        package_id: package.into(),
        action,
        success: false,
        verified: false,
        exit_code: None,
        previous_state: Some(previous.state),
        resulting_state: None,
        message:
            "[Pending] Operation intent saved; if interrupted, inspect the device before retrying"
                .into(),
        classified_cause: crate::adb::ClassifiedCause::None,
        execution_backend: "DirectAdbPm".to_string(),
        command_args: Vec::new(),
        target_device: serial.into(),
        target_user_id: user,
        raw_stdout: None,
        raw_stderr: None,
    };
    JournalManager::log_operation(serial, user, &pending)?;
    let outcome = match OperationExecutor::execute_action(executor, serial, user, package, action) {
        Ok(outcome) => outcome,
        Err(error) => OperationOutcome {
            message: format!("Outcome uncertain: {error}. Reconnect and rescan before retrying."),
            ..pending
        },
    };
    JournalManager::log_operation(serial, user, &outcome).map_err(|e| format!("Device action completed but its journal could not be committed: {e}. Stop and inspect the device."))?;
    Ok(outcome)
}
