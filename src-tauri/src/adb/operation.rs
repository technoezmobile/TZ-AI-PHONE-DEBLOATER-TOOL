use super::command::{is_valid_package_name, AdbExecutor};
use super::helper::TzHelperBackend;
use super::package::{PackageManager, PackageState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Disable,
    Enable,
    UninstallUser,
    Restore,
}

impl ActionType {
    pub fn wanted_state(&self) -> Option<PackageState> {
        match self {
            Self::Disable => Some(PackageState::Disabled),
            Self::Enable => Some(PackageState::Enabled),
            Self::UninstallUser => Some(PackageState::Uninstalled),
            // Restore specifically verifies installation (package is installed, either Enabled or Disabled),
            // rather than enforcing a specific enabled/disabled state at the restore phase.
            Self::Restore => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Disable => "Disable",
            Self::Enable => "Enable",
            Self::UninstallUser => "Uninstall (User)",
            Self::Restore => "Restore System App",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassifiedCause {
    None,
    MissingPrivileges {
        details: String,
        remedy: String,
    },
    OemRestriction {
        vendor: String,
        details: String,
    },
    DevicePolicyRestricted {
        details: String,
    },
    UnsupportedOperation {
        details: String,
    },
    InvalidProfileTarget {
        user_id: u32,
        details: String,
    },
    ApplicationPolicyBlock {
        details: String,
    },
    PackageNotFound {
        package_id: String,
    },
    DeviceOffline,
    Unclassified {
        summary: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationOutcome {
    pub package_id: String,
    pub action: ActionType,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub previous_state: Option<PackageState>,
    pub resulting_state: Option<PackageState>,
    pub message: String,
    pub verified: bool,
    #[serde(default = "default_classified_cause")]
    pub classified_cause: ClassifiedCause,
    #[serde(default = "default_execution_backend")]
    pub execution_backend: String,
    #[serde(default)]
    pub command_args: Vec<String>,
    #[serde(default)]
    pub target_device: String,
    #[serde(default)]
    pub target_user_id: u32,
    #[serde(default)]
    pub raw_stdout: Option<String>,
    #[serde(default)]
    pub raw_stderr: Option<String>,
}

fn default_classified_cause() -> ClassifiedCause {
    ClassifiedCause::None
}

fn default_execution_backend() -> String {
    "DirectAdbPm".to_string()
}

pub struct OperationExecutor;

impl OperationExecutor {
    /// Execute an action on a package for a targeted user profile with full verification
    pub fn execute_action(
        executor: &AdbExecutor,
        serial: &str,
        user_id: u32,
        package_id: &str,
        action: ActionType,
    ) -> Result<OperationOutcome, String> {
        if !is_valid_package_name(package_id) {
            return Err(format!("Invalid package identifier: '{package_id}'"));
        }

        // 1. Capture initial state before modification
        let initial_state =
            PackageManager::query_single_package_state(executor, serial, user_id, package_id)?;

        // If package is already in the requested state, verify immediately without redundant execution
        if let Some(wanted) = action.wanted_state() {
            if initial_state == Some(wanted) {
                let status_msg = match action {
                    ActionType::Disable => "Already in requested state (Disabled).",
                    ActionType::Enable => "Already in requested state (Enabled).",
                    ActionType::UninstallUser => "Already in requested state (Uninstalled for user).",
                    _ => "Already in requested state.",
                };
                return Ok(OperationOutcome {
                    package_id: package_id.to_string(),
                    action,
                    success: true,
                    exit_code: Some(0),
                    previous_state: initial_state,
                    resulting_state: initial_state,
                    message: status_msg.to_string(),
                    verified: true,
                    classified_cause: ClassifiedCause::None,
                    execution_backend: "DirectAdbPm".to_string(),
                    command_args: vec!["state_check".into()],
                    target_device: serial.to_string(),
                    target_user_id: user_id,
                    raw_stdout: None,
                    raw_stderr: None,
                });
            }
        }

        // 2. Try Canta-compatible TZ Helper backend first if available and applicable
        if let Some(helper_outcome) =
            TzHelperBackend::try_execute_action(executor, serial, user_id, package_id, action)
        {
            let mut outcome = helper_outcome;
            outcome.previous_state = initial_state;
            return Ok(outcome);
        }

        let user_str = user_id.to_string();

        // 3. Fallback to Direct ADB package management commands
        let (raw_res, action_args) = match action {
            ActionType::Disable => {
                let args = vec![
                    "pm".to_string(),
                    "disable-user".to_string(),
                    "--user".to_string(),
                    user_str.clone(),
                    package_id.to_string(),
                ];
                let res = executor.run_shell(
                    serial,
                    &["pm", "disable-user", "--user", &user_str, package_id],
                )?;
                // Explicitly target user profile on force-stop
                let _ = executor.run_shell(
                    serial,
                    &["am", "force-stop", "--user", &user_str, package_id],
                );
                (res, args)
            }
            ActionType::Enable => {
                let args = vec![
                    "pm".to_string(),
                    "enable".to_string(),
                    "--user".to_string(),
                    user_str.clone(),
                    package_id.to_string(),
                ];
                let res = executor
                    .run_shell(serial, &["pm", "enable", "--user", &user_str, package_id])?;
                (res, args)
            }
            ActionType::UninstallUser => {
                // Keep data with -k strictly; NO automatic fallback that drops -k to delete user data
                let args = vec![
                    "pm".to_string(),
                    "uninstall".to_string(),
                    "-k".to_string(),
                    "--user".to_string(),
                    user_str.clone(),
                    package_id.to_string(),
                ];
                let res = executor.run_shell(
                    serial,
                    &["pm", "uninstall", "-k", "--user", &user_str, package_id],
                )?;
                (res, args)
            }
            ActionType::Restore => {
                let args = vec![
                    "cmd".to_string(),
                    "package".to_string(),
                    "install-existing".to_string(),
                    "--user".to_string(),
                    user_str.clone(),
                    package_id.to_string(),
                ];
                let res = executor.run_shell(
                    serial,
                    &[
                        "cmd",
                        "package",
                        "install-existing",
                        "--user",
                        &user_str,
                        package_id,
                    ],
                )?;
                if !res.success || res.stdout.contains("Failure") {
                    // Fallback for older Android: pm unhide --user <uid> <pkg>
                    let fallback_args = vec![
                        "pm".to_string(),
                        "unhide".to_string(),
                        "--user".to_string(),
                        user_str.clone(),
                        package_id.to_string(),
                    ];
                    let fallback_res = executor
                        .run_shell(serial, &["pm", "unhide", "--user", &user_str, package_id])?;
                    (fallback_res, fallback_args)
                } else {
                    (res, args)
                }
            }
        };

        // 4. Re-verify the package state on the device
        let resulting_state =
            PackageManager::query_single_package_state(executor, serial, user_id, package_id)
                .unwrap_or(None);

        let mut success = raw_res.success;
        let mut verified = false;

        // Check if stdout contains explicit Android failure markers
        if raw_res.stdout.contains("Failure [")
            || raw_res.stdout.contains("Failure\n")
            || raw_res.stdout.contains("Error:")
            || raw_res.stdout.contains("SecurityException")
        {
            success = false;
        }

        // Distinctly separate installation verification from enabled-state verification:
        if action == ActionType::Restore {
            // Restore specifically verifies that the package is now installed (present on device).
            let is_installed = matches!(
                resulting_state,
                Some(PackageState::Enabled) | Some(PackageState::Disabled)
            );
            if success && is_installed {
                verified = true;
            }
        } else if let Some(wanted) = action.wanted_state() {
            // Disable, Enable, UninstallUser verify exact target state transitions
            if resulting_state == Some(wanted) {
                verified = true;
            } else if success && resulting_state.is_some() {
                verified = false;
            }
        }

        // 5. Structure error classification and authoritative status messages
        let (classified_cause, message) = if success && verified {
            let msg = match action {
                ActionType::Disable => "Verified disabled.".to_string(),
                ActionType::Enable => "Verified enabled.".to_string(),
                ActionType::UninstallUser => {
                    // Check if package remains active on any other profile on this device
                    let other_users =
                        Self::check_package_on_other_profiles(executor, serial, user_id, package_id);
                    if !other_users.is_empty() {
                        let users_str = other_users
                            .iter()
                            .map(|u| u.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!(
                            "Verified uninstalled for User {user_id}. (Note: App remains active on secondary profile User {users_str})"
                        )
                    } else {
                        format!("Verified uninstalled for User {user_id}.")
                    }
                }
                ActionType::Restore => "Verified restored.".to_string(),
            };
            (ClassifiedCause::None, msg)
        } else if !success {
            Self::classify_error(&raw_res.stdout, &raw_res.stderr, raw_res.exit_code, action)
        } else {
            (
                ClassifiedCause::Unclassified {
                    summary: "State verification mismatch".into(),
                },
                format!(
                    "Command completed, but requested state could not be verified on device (current state: {:?}).",
                    resulting_state
                ),
            )
        };

        Ok(OperationOutcome {
            package_id: package_id.to_string(),
            action,
            success: success && verified,
            exit_code: raw_res.exit_code,
            previous_state: initial_state,
            resulting_state,
            message,
            verified,
            classified_cause,
            execution_backend: "DirectAdbPm".to_string(),
            command_args: action_args,
            target_device: serial.to_string(),
            target_user_id: user_id,
            raw_stdout: if raw_res.stdout.is_empty() {
                None
            } else {
                Some(raw_res.stdout)
            },
            raw_stderr: if raw_res.stderr.is_empty() {
                None
            } else {
                Some(raw_res.stderr)
            },
        })
    }

    /// Check if a package is still installed on other user profiles on the device
    pub fn check_package_on_other_profiles(
        executor: &AdbExecutor,
        serial: &str,
        current_user_id: u32,
        package_id: &str,
    ) -> Vec<u32> {
        let mut others = Vec::new();
        if let Ok(users) = super::user::UserManager::list_users(executor, serial) {
            for u in users {
                if u.id != current_user_id {
                    if let Ok(state) =
                        PackageManager::query_single_package_state(executor, serial, u.id, package_id)
                    {
                        if matches!(
                            state,
                            Some(PackageState::Enabled) | Some(PackageState::Disabled)
                        ) {
                            others.push(u.id);
                        }
                    }
                }
            }
        }
        others
    }

    /// Accurately classify raw ADB errors into structured causes with actionable remedies
    pub fn classify_error(
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
        _action: ActionType,
    ) -> (ClassifiedCause, String) {
        let combined = format!("{stdout} {stderr}");
        let lower = combined.to_lowercase();

        if lower.contains("device offline") || lower.contains("device not found") {
            (
                ClassifiedCause::DeviceOffline,
                "Device went offline or disconnected during operation. Check USB connection and screen state.".into(),
            )
        } else if combined.contains("DELETE_FAILED_DEVICE_POLICY_MANAGER") {
            (
                ClassifiedCause::DevicePolicyRestricted {
                    details: "DELETE_FAILED_DEVICE_POLICY_MANAGER".into(),
                },
                "Action blocked by Device Policy Manager (Enterprise MDM, Samsung Knox, or Work Profile policy).".into(),
            )
        } else if combined.contains("DELETE_FAILED_OWNER_BLOCKED") {
            (
                ClassifiedCause::DevicePolicyRestricted {
                    details: "DELETE_FAILED_OWNER_BLOCKED".into(),
                },
                "Package removal blocked by profile owner or device administrator.".into(),
            )
        } else if combined.contains("requires android.permission.CHANGE_COMPONENT_ENABLED_STATE")
            || combined.contains("Permission Denial: attempt to change component state")
            || combined.contains("neither user 2000 nor current process has android.permission.CHANGE_COMPONENT_ENABLED_STATE")
        {
            (
                ClassifiedCause::MissingPrivileges {
                    details: "Missing CHANGE_COMPONENT_ENABLED_STATE privilege".into(),
                    remedy: "On Xiaomi/MIUI/HyperOS devices, enabling 'USB debugging (Security settings)' in Developer Options is required to disable system apps. Alternatively, use 'Uninstall (User)' which uninstalls the app for your profile without requiring component state permissions.".into(),
                },
                "Action denied by OEM security policy. Disabling system components via ADB requires enabling 'USB debugging (Security settings)' in Developer Options. Alternatively, use 'Uninstall for User' (pm uninstall -k --user <uid>) which operates cleanly without this privilege.".into(),
            )
        } else if combined.contains("requires android.permission.MANAGE_USERS")
            || combined.contains("Permission Denial: manage_users")
        {
            (
                ClassifiedCause::MissingPrivileges {
                    details: "Requires MANAGE_USERS permission".into(),
                    remedy: "The requested profile cannot be modified by shell user 2000. Ensure the targeted user profile is unlocked and accessible.".into(),
                },
                "Action denied: Target user profile requires MANAGE_USERS privilege not granted to non-root ADB shell.".into(),
            )
        } else if combined.contains("Cannot disable a protected package")
            || combined.contains("protected package")
        {
            (
                ClassifiedCause::ApplicationPolicyBlock {
                    details: "Core system package marked protected from disabling".into(),
                },
                "Application policy block: Android protects this package from being disabled while running as a core system component.".into(),
            )
        } else if combined.contains("inaccessible profile")
            || combined.contains("inaccessible user")
            || combined.contains("user not found")
        {
            (
                ClassifiedCause::InvalidProfileTarget {
                    user_id: 0,
                    details: "User profile inaccessible or not found".into(),
                },
                "Target Android profile does not exist or is currently locked/inaccessible.".into(),
            )
        } else if combined.contains("Unknown package")
            || combined.contains("not installed for")
            || combined.contains("package not found")
        {
            (
                ClassifiedCause::PackageNotFound {
                    package_id: "".into(),
                },
                "Package is not installed for this user profile.".into(),
            )
        } else if combined.contains("SecurityException") || combined.contains("Permission Denial") {
            (
                ClassifiedCause::OemRestriction {
                    vendor: "OEM".into(),
                    details: combined.trim().to_string(),
                },
                format!(
                    "Action denied by Android/OEM security check: {}. Non-root ADB debugging cannot modify this component under current device settings.",
                    combined.lines().next().unwrap_or("SecurityException")
                ),
            )
        } else if combined.contains("Failure [") {
            let marker = combined
                .split("Failure [")
                .nth(1)
                .and_then(|s| s.split(']').next())
                .unwrap_or("UNKNOWN_FAILURE");
            (
                ClassifiedCause::UnsupportedOperation {
                    details: format!("Failure [{marker}]"),
                },
                format!("Android Package Manager reported failure: {marker}."),
            )
        } else if !stderr.is_empty() {
            (
                ClassifiedCause::Unclassified {
                    summary: stderr.trim().to_string(),
                },
                stderr.trim().to_string(),
            )
        } else if !stdout.is_empty() {
            (
                ClassifiedCause::Unclassified {
                    summary: stdout.trim().to_string(),
                },
                stdout.trim().to_string(),
            )
        } else {
            (
                ClassifiedCause::Unclassified {
                    summary: format!("Exit code {:?}", exit_code),
                },
                format!("Operation failed with exit code {:?}.", exit_code),
            )
        }
    }

    /// Translates raw ADB/pm errors to user-friendly explanations (legacy backward-compatible helper)
    pub fn translate_error_message(stdout: &str, stderr: &str) -> String {
        let (_, msg) = Self::classify_error(stdout, stderr, None, ActionType::Disable);
        msg
    }
}
