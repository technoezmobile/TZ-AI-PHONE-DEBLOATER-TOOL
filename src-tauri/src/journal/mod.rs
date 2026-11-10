use crate::adb::operation::{ActionType, OperationOutcome};
use crate::adb::package::PackageState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub snapshot_id: String,
    pub device_serial: String,
    pub device_model: String,
    pub user_id: u32,
    pub timestamp: i64,
    pub note: String,
    pub package_states: Vec<PackageSnapshotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSnapshotEntry {
    pub package_id: String,
    pub state: PackageState,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub entry_id: String,
    pub timestamp: i64,
    pub device_serial: String,
    pub user_id: u32,
    pub package_id: String,
    pub action: ActionType,
    pub previous_state: Option<PackageState>,
    pub resulting_state: Option<PackageState>,
    pub success: bool,
    #[serde(default)]
    pub verified: bool,
    pub message: String,
    #[serde(default)]
    pub recovery_steps: Vec<ActionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryProposal {
    pub entry_id: String,
    pub package_id: String,
    pub device_serial: String,
    pub user_id: u32,
    pub target_state: PackageState,
    pub steps: Vec<ActionType>,
    pub explanation: String,
    pub can_recover: bool,
    pub blockage_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebloatProfile {
    pub profile_name: String,
    pub description: String,
    pub created_at: i64,
    pub actions: Vec<ProfileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileItem {
    pub package_id: String,
    pub desired_action: ActionType,
}

static JOURNAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub struct JournalManager;

impl JournalManager {
    fn get_app_dir() -> PathBuf {
        let base = if cfg!(debug_assertions) {
            std::env::var_os("TZ_DEBLOATER_TEST_DATA")
                .map(PathBuf::from)
                .or_else(dirs::data_local_dir)
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."))
        };
        let dir = base.join("TZ_AI_Debloater");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn get_snapshots_dir() -> PathBuf {
        let dir = Self::get_app_dir().join("snapshots");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn get_journal_path() -> PathBuf {
        Self::get_app_dir().join("operation_journal.json")
    }

    /// Save pre-change snapshot atomically with collision-resistant UUID filename
    pub fn save_snapshot(snapshot: &DeviceSnapshot) -> Result<String, String> {
        let clean_serial = snapshot
            .device_serial
            .replace([':', '.', '/', '\\', ' '], "_");
        let filename = format!(
            "snapshot_{}_{}_{}_{}.json",
            clean_serial,
            snapshot.user_id,
            snapshot.timestamp,
            uuid::Uuid::new_v4().to_string()
        );
        let path = Self::get_snapshots_dir().join(&filename);
        let temp_path = path.with_extension("tmp");

        let json = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;

        fs::write(&temp_path, json)
            .map_err(|e| format!("Failed to write snapshot to temp file: {e}"))?;
        fs::rename(&temp_path, &path)
            .map_err(|e| format!("Failed to commit snapshot atomically: {e}"))?;

        Ok(path.to_string_lossy().to_string())
    }

    /// List all saved snapshots
    pub fn list_snapshots() -> Result<Vec<DeviceSnapshot>, String> {
        let dir = Self::get_snapshots_dir();
        let mut list = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(snap) = serde_json::from_str::<DeviceSnapshot>(&content) {
                            list.push(snap);
                        }
                    }
                }
            }
        }

        // Sort descending by timestamp
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(list)
    }

    /// Calculate recovery steps for transitioning from resulting state back to previous state.
    /// Supports multi-action recovery:
    /// - Previously Enabled -> now Uninstalled: Restore, then Enable if necessary
    /// - Previously Disabled -> now Uninstalled: Restore, then Disable
    /// - Previously Disabled -> now Enabled: Disable
    /// - Previously Enabled -> now Disabled: Enable
    pub fn derive_recovery_steps(
        previous_state: Option<PackageState>,
        resulting_state: Option<PackageState>,
    ) -> Vec<ActionType> {
        match (previous_state, resulting_state) {
            (Some(PackageState::Enabled), Some(PackageState::Uninstalled)) => {
                vec![ActionType::Restore, ActionType::Enable]
            }
            (Some(PackageState::Disabled), Some(PackageState::Uninstalled)) => {
                vec![ActionType::Restore, ActionType::Disable]
            }
            (Some(PackageState::Disabled), Some(PackageState::Enabled)) => {
                vec![ActionType::Disable]
            }
            (Some(PackageState::Enabled), Some(PackageState::Disabled)) => {
                vec![ActionType::Enable]
            }
            _ => Vec::new(),
        }
    }

    /// Record executed operation into the persistent audit journal atomically
    pub fn log_operation(
        device_serial: &str,
        user_id: u32,
        outcome: &OperationOutcome,
    ) -> Result<JournalEntry, String> {
        let _guard = JOURNAL_LOCK.lock().map_err(|_| "Journal lock poisoned")?;
        let _cross_lock = crate::storage::lock::CrossProcessLock::lock_journal().ok();
        let recovery_steps =
            Self::derive_recovery_steps(outcome.previous_state, outcome.resulting_state);

        let entry = JournalEntry {
            entry_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            device_serial: device_serial.to_string(),
            user_id,
            package_id: outcome.package_id.clone(),
            action: outcome.action,
            previous_state: outcome.previous_state,
            resulting_state: outcome.resulting_state,
            success: outcome.success,
            verified: outcome.verified,
            message: outcome.message.clone(),
            recovery_steps,
        };

        let mut current_journal = Self::read_journal()?;
        if !outcome.message.starts_with("[Pending]") {
            current_journal.retain(|e| {
                !(e.device_serial == device_serial
                    && e.user_id == user_id
                    && e.package_id == outcome.package_id
                    && e.message.starts_with("[Pending]"))
            });
        }
        current_journal.push(entry.clone());

        let path = Self::get_journal_path();
        let temp_path = path.with_extension("tmp");
        let json = serde_json::to_string_pretty(&current_journal).map_err(|e| e.to_string())?;

        fs::write(&temp_path, json)
            .map_err(|e| format!("Failed to write journal temp file: {e}"))?;
        fs::rename(&temp_path, &path)
            .map_err(|e| format!("Failed to commit journal entry: {e}"))?;

        Ok(entry)
    }

    /// Read full journal history from disk
    pub fn read_journal() -> Result<Vec<JournalEntry>, String> {
        let _cross_lock = crate::storage::lock::CrossProcessLock::lock_journal().ok();
        let path = Self::get_journal_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read journal: {e}"))?;
        let entries = serde_json::from_str::<Vec<JournalEntry>>(&content)
            .map_err(|e| format!("Invalid journal format: {e}"))?;
        Ok(entries)
    }

    /// Propose verified recovery action bound to current device and profile
    pub fn propose_recovery(
        entry: &JournalEntry,
        current_serial: &str,
        current_user_id: u32,
        current_package_state: Option<PackageState>,
    ) -> RecoveryProposal {
        let serial_match = entry.device_serial == current_serial;
        let user_match = entry.user_id == current_user_id;

        let prev = match entry.previous_state {
            Some(s) => s,
            None => {
                return RecoveryProposal {
                    entry_id: entry.entry_id.clone(),
                    package_id: entry.package_id.clone(),
                    device_serial: entry.device_serial.clone(),
                    user_id: entry.user_id,
                    target_state: PackageState::Enabled,
                    steps: Vec::new(),
                    explanation: "Unknown previous state in journal".into(),
                    can_recover: false,
                    blockage_reason: Some("Previous state was not captured".into()),
                };
            }
        };

        if !serial_match {
            return RecoveryProposal {
                entry_id: entry.entry_id.clone(),
                package_id: entry.package_id.clone(),
                device_serial: entry.device_serial.clone(),
                user_id: entry.user_id,
                target_state: prev,
                steps: Vec::new(),
                explanation: format!(
                    "Recorded on serial '{}', but connected device is '{}'",
                    entry.device_serial, current_serial
                ),
                can_recover: false,
                blockage_reason: Some(
                    "Target device serial mismatch. Cross-device recovery prohibited.".into(),
                ),
            };
        }

        if !user_match {
            return RecoveryProposal {
                entry_id: entry.entry_id.clone(),
                package_id: entry.package_id.clone(),
                device_serial: entry.device_serial.clone(),
                user_id: entry.user_id,
                target_state: prev,
                steps: Vec::new(),
                explanation: format!(
                    "Recorded for User {}, but current selected profile is User {}",
                    entry.user_id, current_user_id
                ),
                can_recover: false,
                blockage_reason: Some(
                    "Target Android user profile mismatch. Cross-profile recovery prohibited."
                        .into(),
                ),
            };
        }

        let steps = Self::derive_recovery_steps(Some(prev), current_package_state);
        let already_in_state = current_package_state == Some(prev);

        let (can_recover, reason) = if already_in_state {
            (
                false,
                Some(format!("Package is already in desired state ({prev}).")),
            )
        } else if steps.is_empty() {
            (
                false,
                Some("No valid transition steps identified from current state.".into()),
            )
        } else {
            (true, None)
        };

        RecoveryProposal {
            entry_id: entry.entry_id.clone(),
            package_id: entry.package_id.clone(),
            device_serial: entry.device_serial.clone(),
            user_id: entry.user_id,
            target_state: prev,
            steps,
            explanation: format!("Recover to previous state: {prev}"),
            can_recover,
            blockage_reason: reason,
        }
    }

    /// Export selection debloat profile to a JSON file
    pub fn export_profile(profile: &DebloatProfile, path: &PathBuf) -> Result<(), String> {
        let json = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("Failed to export profile: {e}"))
    }

    /// Import selection debloat profile from a JSON file
    pub fn import_profile(path: &PathBuf) -> Result<DebloatProfile, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read profile: {e}"))?;
        let profile = serde_json::from_str::<DebloatProfile>(&content)
            .map_err(|e| format!("Invalid profile format: {e}"))?;
        Ok(profile)
    }

    /// Clear all journal history
    pub fn clear_journal() -> Result<(), String> {
        let _guard = JOURNAL_LOCK.lock().map_err(|_| "Journal lock poisoned")?;
        let path = Self::get_journal_path();
        if path.exists() {
            fs::remove_file(path).map_err(|e| format!("Failed to clear journal: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_step_recovery_mapping() {
        // Previously Disabled -> now Uninstalled
        let steps = JournalManager::derive_recovery_steps(
            Some(PackageState::Disabled),
            Some(PackageState::Uninstalled),
        );
        assert_eq!(steps, vec![ActionType::Restore, ActionType::Disable]);

        // Previously Enabled -> now Uninstalled
        let steps = JournalManager::derive_recovery_steps(
            Some(PackageState::Enabled),
            Some(PackageState::Uninstalled),
        );
        assert_eq!(steps, vec![ActionType::Restore, ActionType::Enable]);

        // Previously Disabled -> now Enabled
        let steps = JournalManager::derive_recovery_steps(
            Some(PackageState::Disabled),
            Some(PackageState::Enabled),
        );
        assert_eq!(steps, vec![ActionType::Disable]);

        // Previously Enabled -> now Disabled
        let steps = JournalManager::derive_recovery_steps(
            Some(PackageState::Enabled),
            Some(PackageState::Disabled),
        );
        assert_eq!(steps, vec![ActionType::Enable]);
    }

    #[test]
    fn test_propose_recovery_cross_device_blocked() {
        let entry = JournalEntry {
            entry_id: "e1".into(),
            timestamp: 1000,
            device_serial: "phone_A".into(),
            user_id: 0,
            package_id: "com.test.app".into(),
            action: ActionType::Disable,
            previous_state: Some(PackageState::Enabled),
            resulting_state: Some(PackageState::Disabled),
            success: true,
            verified: true,
            message: "Disabled".into(),
            recovery_steps: vec![ActionType::Enable],
        };

        // Current device is phone_B
        let proposal =
            JournalManager::propose_recovery(&entry, "phone_B", 0, Some(PackageState::Disabled));
        assert_eq!(proposal.can_recover, false);
        assert!(proposal
            .blockage_reason
            .unwrap()
            .contains("Target device serial mismatch"));
    }
}
