pub mod adb;
pub mod ai;
pub mod dataset;
pub mod execution;
pub mod journal;
pub mod plan;
pub mod policy;
pub mod storage;

use adb::{
    ActionType, AdbExecutor, AdbLocator, AdbStatus, AndroidPackageType, AndroidUser,
    ClassifiedCause, DeviceInfo, DeviceManager, DevicePackage, OperationOutcome, PackageManager,
    PackageState, UserManager,
};
use ai::{
    ActionPlan, AiClient, ChatMessage, ClassifiedError, GatewayReadinessInfo, GenerationTestResult,
    ModelDiscoveryResult, ProposedAction, StreamChunkPayload, StreamDonePayload,
    StreamErrorPayload,
};
use dataset::DatasetStats;
use journal::{
    DeviceSnapshot, JournalEntry, JournalManager, PackageSnapshotEntry, RecoveryProposal,
};
use plan::{PlanValidationResult, PlanValidator};
use storage::credentials::{AppConfig, ConfigStore, CredentialStore};

use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanApprovalSession {
    pub approval_token: String,
    pub plan_id: String,
    pub revision: u64,
    pub target_device_serial: String,
    pub target_user_id: u32,
    pub inventory_hash: String,
    pub approved_actions: Vec<ProposedAction>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchItemValidation {
    pub package_id: String,
    pub package_type: AndroidPackageType,
    pub current_state: PackageState,
    pub action: ActionType,
    pub target_user: u32,
    pub eligible: bool,
    pub exclusion_reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchPreflightResult {
    pub items: Vec<BatchItemValidation>,
    pub total_requested: usize,
    pub eligible_count: usize,
    pub excluded_count: usize,
}

pub struct AppState {
    pub config: RwLock<AppConfig>,
    pub active_target: Mutex<Option<(String, u32)>>,
    pub active_approval_session: Mutex<Option<PlanApprovalSession>>,
    pub cancellation_tokens: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub device_mutation_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub gateway: Arc<ai::gateway::TzAiGateway>,
}

impl AppState {
    pub fn get_device_mutation_lock(&self, serial: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.device_mutation_locks.lock();
        locks
            .entry(serial.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

pub fn compute_inventory_hash(packages: &[DevicePackage]) -> String {
    use std::collections::BTreeSet;
    use std::hash::{Hash, Hasher};
    let mut sorted = BTreeSet::new();
    for p in packages {
        sorted.insert(format!("{}:{}:{}", p.package_id, p.user_id, p.state));
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for item in sorted {
        item.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn require_target(state: &AppState, serial: &str, user: u32) -> Result<(), String> {
    if state.active_target.lock().as_ref() != Some(&(serial.to_string(), user)) {
        return Err(
            "Device/profile selection changed. Select the target again and review the operation."
                .into(),
        );
    }
    Ok(())
}

#[tauri::command]
fn set_active_target(
    state: State<'_, AppState>,
    serial: Option<String>,
    user_id: Option<u32>,
) -> Result<(), String> {
    let target = serial.zip(user_id).filter(|(s, _)| !s.trim().is_empty());
    let mut active = state.active_target.lock();
    if *active != target {
        *state.active_approval_session.lock() = None;
        for token in state.cancellation_tokens.lock().values() {
            token.store(true, Ordering::SeqCst);
        }
        *active = target;
    }
    Ok(())
}

#[tauri::command]
async fn export_profile(
    state: State<'_, AppState>,
    device_serial: String,
    user_id: u32,
) -> Result<journal::DebloatProfile, String> {
    require_target(&state, &device_serial, user_id)?;
    let packages =
        PackageManager::scan_device_packages(&get_executor(&state)?, &device_serial, user_id)?;
    Ok(journal::DebloatProfile {
        profile_name: "Package state selection".into(),
        description: "State rules only; not a device or application-data backup".into(),
        created_at: chrono::Utc::now().timestamp(),
        actions: packages
            .into_iter()
            .filter(|p| p.state == adb::PackageState::Disabled)
            .map(|p| journal::ProfileItem {
                package_id: p.package_id,
                desired_action: ActionType::Disable,
            })
            .collect(),
    })
}

#[tauri::command]
fn import_profile(profile_json: String) -> Result<journal::DebloatProfile, String> {
    if profile_json.len() > 1024 * 1024 {
        return Err("Profile exceeds 1 MB".into());
    }
    let profile: journal::DebloatProfile =
        serde_json::from_str(&profile_json).map_err(|e| format!("Invalid profile: {e}"))?;
    if profile.actions.len() > 10000 || profile.profile_name.trim().is_empty() {
        return Err("Invalid profile name or action count".into());
    }
    let mut seen = HashSet::new();
    for a in &profile.actions {
        if !adb::is_valid_package_name(&a.package_id) || !seen.insert(a.package_id.clone()) {
            return Err("Invalid or duplicate package in profile".into());
        }
    }
    Ok(profile)
}

// -----------------------------------------------------------------------------
// ADB & Device Discovery Commands
// -----------------------------------------------------------------------------

#[tauri::command]
async fn get_adb_status(state: State<'_, AppState>) -> Result<AdbStatus, String> {
    let cfg = state.config.read();
    Ok(AdbLocator::locate(cfg.custom_adb_path.as_deref()))
}

#[tauri::command]
async fn download_platform_tools(app: AppHandle) -> Result<String, String> {
    let app_handle = app.clone();
    let res = AdbLocator::download_and_setup_platform_tools(move |progress, message| {
        let _ = app_handle.emit(
            "platform-tools-progress",
            serde_json::json!({
                "progress": progress,
                "message": message
            }),
        );
    })
    .await?;

    Ok(res.to_string_lossy().to_string())
}

fn get_executor(state: &AppState) -> Result<AdbExecutor, String> {
    let cfg = state.config.read();
    let status = AdbLocator::locate(cfg.custom_adb_path.as_deref());
    if let Some(path) = status.path {
        Ok(AdbExecutor::new(path))
    } else {
        Err(status
            .error
            .unwrap_or_else(|| "ADB binary not found. Please setup platform-tools.".into()))
    }
}

#[tauri::command]
async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, String> {
    let executor = get_executor(&state)?;
    let result = DeviceManager::list_devices(&executor);
    let active = state.active_target.lock().clone();
    if let Some((serial, _)) = active {
        if !result
            .as_ref()
            .map(|devices| {
                devices
                    .iter()
                    .any(|d| d.serial == serial && d.state.is_ready())
            })
            .unwrap_or(false)
        {
            *state.active_target.lock() = None;
            *state.active_approval_session.lock() = None;
            for token in state.cancellation_tokens.lock().values() {
                token.store(true, Ordering::SeqCst);
            }
        }
    }
    result
}

#[tauri::command]
fn pair_wireless_adb(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    code: String,
) -> Result<String, String> {
    let executor = get_executor(&state)?;
    DeviceManager::pair_wireless(&executor, &host, port, &code)
}

#[tauri::command]
fn connect_wireless_adb(
    state: State<'_, AppState>,
    host: String,
    port: u16,
) -> Result<String, String> {
    let executor = get_executor(&state)?;
    DeviceManager::connect_wireless(&executor, &host, port)
}

#[tauri::command]
fn disconnect_wireless_adb(state: State<'_, AppState>, address: String) -> Result<String, String> {
    let executor = get_executor(&state)?;
    DeviceManager::disconnect_wireless(&executor, &address)
}

#[tauri::command]
async fn list_users(
    state: State<'_, AppState>,
    device_serial: String,
) -> Result<Vec<AndroidUser>, String> {
    let executor = get_executor(&state)?;
    UserManager::list_users(&executor, &device_serial)
}

#[tauri::command]
async fn scan_packages(
    state: State<'_, AppState>,
    device_serial: String,
    user_id: u32,
) -> Result<Vec<DevicePackage>, String> {
    let executor = get_executor(&state)?;
    PackageManager::scan_device_packages(&executor, &device_serial, user_id)
}

#[tauri::command]
fn get_dataset_stats() -> DatasetStats {
    dataset::get_dataset_stats()
}

// -----------------------------------------------------------------------------
// Configuration & Credential Commands
// -----------------------------------------------------------------------------

#[tauri::command]
fn get_app_config(state: State<'_, AppState>) -> AppConfig {
    let mut cfg = state.config.read().clone();
    for conn in &mut cfg.connections {
        conn.has_credential = CredentialStore::has_credential(&conn.credential_id);
    }
    cfg
}

#[tauri::command]
fn save_app_config(state: State<'_, AppState>, new_config: AppConfig) -> Result<(), String> {
    if new_config.connections.is_empty()
        || !new_config
            .connections
            .iter()
            .any(|c| c.id == new_config.active_connection_id)
    {
        return Err("Select an existing active connection".into());
    }
    let mut ids = HashSet::new();
    for c in &new_config.connections {
        if c.id != c.credential_id || !ids.insert(c.id.clone()) || c.id.is_empty() {
            return Err("Connection credentials must have unique matching IDs".into());
        }
        AiClient::normalize_endpoint(&c.base_url, "models").map_err(|e| e.message)?;
    }
    ConfigStore::save_config(&new_config)?;
    state.gateway.sync_profiles(&new_config.connections);
    *state.config.write() = new_config;
    Ok(())
}

#[tauri::command]
fn save_credential(connection_id: String, secret: String) -> Result<(), String> {
    let clean = secret.trim();
    if clean.is_empty() {
        return Err("Credential cannot be empty".into());
    }
    CredentialStore::save_credential(&connection_id, clean)
}

#[tauri::command]
fn remove_credential(connection_id: String) -> Result<(), String> {
    CredentialStore::delete_credential(&connection_id)
}

#[tauri::command]
fn approve_destination(state: State<'_, AppState>, host: String) -> Result<(), String> {
    AiClient::normalize_endpoint(&host, "models").map_err(|e| e.message)?;
    let clean_host = host.trim().trim_end_matches('/').to_string();
    if clean_host.is_empty() {
        return Err("Destination host cannot be empty".into());
    }
    let mut cfg = state.config.write();
    if !cfg.approved_destinations.contains(&clean_host) {
        cfg.approved_destinations.push(clean_host);
        ConfigStore::save_config(&cfg)?;
    }
    Ok(())
}

#[tauri::command]
async fn test_saved_credential(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<GenerationTestResult, ClassifiedError> {
    let profile = {
        let cfg = state.config.read();
        cfg.connections
            .iter()
            .find(|c| c.id == connection_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Connection profile '{connection_id}' not found"),
                suggested_action: "Select an existing connection profile".into(),
                diagnostic_details: None,
                http_status: None,
            })?
    };

    let cred =
        CredentialStore::get_credential(&profile.credential_id).map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Authentication,
            message: format!("Failed to read credential from secure store: {e}"),
            suggested_action: "Re-enter your API key in Settings".into(),
            diagnostic_details: Some(e),
            http_status: None,
        })?;

    let model = profile
        .manual_model_id
        .as_deref()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(&profile.selected_model)
        .trim()
        .to_string();

    let model_to_use = if !model.is_empty() {
        model
    } else if profile.protocol == storage::credentials::ApiProtocol::GeminiNative {
        "gemini-3.5-flash".to_string()
    } else {
        model
    };

    AiClient::test_generation(&profile, cred.as_deref(), &model_to_use).await
}

#[tauri::command]
async fn test_new_credential(
    state: State<'_, AppState>,
    connection_id: String,
    secret: String,
) -> Result<GenerationTestResult, ClassifiedError> {
    let profile = {
        let cfg = state.config.read();
        cfg.connections
            .iter()
            .find(|c| c.id == connection_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Connection profile '{connection_id}' not found"),
                suggested_action: "Select an existing connection profile".into(),
                diagnostic_details: None,
                http_status: None,
            })?
    };

    let model = profile
        .manual_model_id
        .as_deref()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(&profile.selected_model)
        .trim()
        .to_string();

    let model_to_use = if !model.is_empty() {
        model
    } else if profile.protocol == storage::credentials::ApiProtocol::GeminiNative {
        "gemini-3.5-flash".to_string()
    } else {
        model
    };

    AiClient::test_generation(&profile, Some(secret.trim()), &model_to_use).await
}

#[tauri::command]
async fn discover_models(
    state: State<'_, AppState>,
    connection_id: String,
    page_token: Option<String>,
) -> Result<ModelDiscoveryResult, ClassifiedError> {
    let profile = {
        let cfg = state.config.read();
        cfg.connections
            .iter()
            .find(|c| c.id == connection_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Connection profile '{connection_id}' not found"),
                suggested_action: "Select an existing connection profile".into(),
                diagnostic_details: None,
                http_status: None,
            })?
    };

    if profile.id == ai::GATEWAY_CONNECTION_ID
        || profile.id == "preset-tz-gateway"
        || profile.id == "preset-freellmapi"
        || profile.preset == storage::credentials::ConnectionPreset::FreeLlmApi
    {
        return state.gateway.discover_all_models(&CredentialStore).await;
    }

    let cred =
        CredentialStore::get_credential(&profile.credential_id).map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Authentication,
            message: e,
            suggested_action: "Check Windows Credential Manager and save the credential again"
                .into(),
            diagnostic_details: None,
            http_status: None,
        })?;
    AiClient::discover_models(&profile, cred.as_deref(), page_token.as_deref()).await
}

#[tauri::command]
fn get_gateway_status(state: State<'_, AppState>) -> Result<ai::gateway::GatewayStatus, String> {
    Ok(state.gateway.get_status())
}

#[tauri::command]
async fn check_gateway_readiness(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<GatewayReadinessInfo, ClassifiedError> {
    let profile = {
        let cfg = state.config.read();
        cfg.connections
            .iter()
            .find(|c| c.id == connection_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Connection profile '{connection_id}' not found"),
                suggested_action: "Select an existing connection profile".into(),
                diagnostic_details: None,
                http_status: None,
            })?
    };

    if profile.id == ai::GATEWAY_CONNECTION_ID
        || profile.id == "preset-tz-gateway"
        || profile.id == "preset-freellmapi"
        || profile.preset == storage::credentials::ConnectionPreset::FreeLlmApi
    {
        let status = state.gateway.get_status();
        return Ok(GatewayReadinessInfo {
            is_ready: status.is_ready,
            gateway_available: true,
            upstream_available: status.healthy_providers_count > 0,
            message: if status.is_ready {
                format!(
                    "TZ AI Gateway is active and ready ({} healthy provider(s) configured).",
                    status.healthy_providers_count
                )
            } else {
                "TZ AI Gateway has no configured, healthy providers. Configure API keys in Settings.".into()
            },
            served_model: Some("gateway:auto".into()),
        });
    }

    let cred =
        CredentialStore::get_credential(&profile.credential_id).map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Authentication,
            message: e,
            suggested_action: "Check Windows Credential Manager and save the credential again"
                .into(),
            diagnostic_details: None,
            http_status: None,
        })?;
    AiClient::check_gateway_readiness(&profile, cred.as_deref()).await
}

#[tauri::command]
async fn test_model_generation(
    state: State<'_, AppState>,
    connection_id: String,
    model_id: String,
) -> Result<GenerationTestResult, ClassifiedError> {
    let profile = {
        let cfg = state.config.read();
        cfg.connections
            .iter()
            .find(|c| c.id == connection_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Connection profile '{connection_id}' not found"),
                suggested_action: "Select an existing connection profile".into(),
                diagnostic_details: None,
                http_status: None,
            })?
    };

    let cred =
        CredentialStore::get_credential(&profile.credential_id).map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Authentication,
            message: e,
            suggested_action: "Check Windows Credential Manager and save the credential again"
                .into(),
            diagnostic_details: None,
            http_status: None,
        })?;
    AiClient::test_generation(&profile, cred.as_deref(), &model_id).await
}

struct RequestRegistration<'a> {
    state: &'a AppState,
    id: String,
}
impl Drop for RequestRegistration<'_> {
    fn drop(&mut self) {
        self.state.cancellation_tokens.lock().remove(&self.id);
    }
}

// -----------------------------------------------------------------------------
// AI Chat & Streaming Commands
// -----------------------------------------------------------------------------

#[tauri::command]
async fn stream_ai_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: String,
    user_prompt: String,
    device_serial: String,
    user_id: u32,
    connection_id: Option<String>,
    conversation_history: Vec<ChatMessage>,
    shortcut_intent: Option<ai::types::ShortcutIntent>,
) -> Result<(), ClassifiedError> {
    let (profile, consent_given, approved_destinations) = {
        let cfg = state.config.read();
        let conn_id = connection_id.unwrap_or_else(|| cfg.active_connection_id.clone());
        let prof = cfg
            .connections
            .iter()
            .find(|c| c.id == conn_id)
            .cloned()
            .ok_or_else(|| ClassifiedError {
                error_type: ai::ErrorType::InvalidEndpointOrProtocol,
                message: format!("Active connection profile '{conn_id}' not found"),
                suggested_action: "Verify connection profile selection in Settings".into(),
                diagnostic_details: None,
                http_status: None,
            })?;
        (
            prof,
            cfg.ai_consent_given,
            cfg.approved_destinations.clone(),
        )
    };

    if !consent_given {
        return Err(ClassifiedError {
            error_type: ai::ErrorType::PermissionAccess,
            message: "AI consent not given".into(),
            suggested_action:
                "Review and accept the AI Privacy & Consent notice before sending requests".into(),
            diagnostic_details: None,
            http_status: None,
        });
    }

    // Validate destination approval for external/custom endpoints
    if let Ok(parsed_url) = reqwest::Url::parse(&profile.base_url) {
        if let Some(host) = parsed_url.host_str() {
            let host_clean = host.to_lowercase();
            let is_loopback =
                host_clean == "localhost" || host_clean == "127.0.0.1" || host_clean == "::1";
            if !is_loopback
                && !approved_destinations
                    .iter()
                    .any(|d| d.trim_end_matches('/') == profile.base_url.trim_end_matches('/'))
            {
                return Err(ClassifiedError {
                    error_type: ai::ErrorType::PermissionAccess,
                    message: format!("Destination endpoint '{host}' has not been approved for device data transmission."),
                    suggested_action: "Approve this destination in Settings before sending requests.".into(),
                    diagnostic_details: Some(format!("Host {host} is unapproved")),
                    http_status: None,
                });
            }
        }
    }

    require_target(&state, &device_serial, user_id).map_err(|message| ClassifiedError {
        error_type: ai::ErrorType::PermissionAccess,
        message,
        suggested_action: "Select the connected device and Android profile again".into(),
        diagnostic_details: None,
        http_status: None,
    })?;
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut tokens = state.cancellation_tokens.lock();
        tokens.insert(request_id.clone(), Arc::clone(&cancel_token));
    }

    let _request_registration = RequestRegistration {
        state: &state,
        id: request_id.clone(),
    };
    let executor = get_executor(&state).map_err(|e| ClassifiedError {
        error_type: ai::ErrorType::Unknown,
        message: format!("ADB unavailable: {e}"),
        suggested_action: "Check device connection and ADB installation".into(),
        diagnostic_details: Some(e),
        http_status: None,
    })?;

    let packages = PackageManager::scan_device_packages(&executor, &device_serial, user_id)
        .map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Unknown,
            message: format!("Failed to read device packages: {e}"),
            suggested_action: "Check device permissions and connection".into(),
            diagnostic_details: Some(e),
            http_status: None,
        })?;

    let devices = DeviceManager::list_devices(&executor).unwrap_or_default();
    let dev = devices.into_iter().find(|d| d.serial == device_serial);
    let dev_model = dev
        .as_ref()
        .map(|d| format!("{} {}", d.brand, d.model))
        .unwrap_or_else(|| "Android Device".into());
    let android_ver = dev
        .as_ref()
        .map(|d| d.android_version.clone())
        .unwrap_or_else(|| "Unknown".into());

    // Exclude packages already uninstalled for this user!
    let currently_installed: Vec<crate::adb::package::DevicePackage> = packages
        .into_iter()
        .filter(|p| p.state != crate::adb::package::PackageState::Uninstalled)
        .collect();

    // If an explicit shortcut intent is active, earlier conversation must not override its scope
    let effective_history = if shortcut_intent.is_some() {
        Vec::new()
    } else {
        conversation_history
    };

    let mut target_packages = currently_installed;
    if let Some(intent) = shortcut_intent {
        match intent {
            ai::types::ShortcutIntent::ReviewTrackers => {
                target_packages.retain(|p| AiClient::is_tracker_or_analytics(&p.package_id));
                if target_packages.is_empty() {
                    let msg = format!(
                        "No catalog-supported tracking or analytics packages were detected among the currently installed packages for User {}.",
                        user_id
                    );
                    let _ = app.emit("ai-stream-chunk", StreamChunkPayload {
                        request_id: request_id.clone(),
                        text: msg,
                        is_thought: false,
                    });
                    let _ = app.emit("ai-stream-done", StreamDonePayload {
                        request_id: request_id.clone(),
                        finish_reason: "stop".into(),
                        served_model: None,
                        plan_id: None,
                    });
                    return Ok(());
                }
            }
            ai::types::ShortcutIntent::UninstallPromos => {
                target_packages.retain(|p| AiClient::is_promotional(&p.package_id));
                if target_packages.is_empty() {
                    let msg = format!(
                        "No eligible promotional or sponsor packages were detected among the currently installed packages for User {}.",
                        user_id
                    );
                    let _ = app.emit("ai-stream-chunk", StreamChunkPayload {
                        request_id: request_id.clone(),
                        text: msg,
                        is_thought: false,
                    });
                    let _ = app.emit("ai-stream-done", StreamDonePayload {
                        request_id: request_id.clone(),
                        finish_reason: "stop".into(),
                        served_model: None,
                        plan_id: None,
                    });
                    return Ok(());
                }
            }
            ai::types::ShortcutIntent::ReviewBloatware => {
                // Bloatware scope: strictly eligible bloatware candidates with enough evidence
                // Excludes built-in utilities (Notes, Compass) and MediaTek vendor packages
                target_packages.retain(|p| {
                    p.removal_eligibility == crate::adb::package::RemovalEligibility::Eligible
                });
            }
        }
    } else {
        // General debloat queries: exclude protected components, built-in utilities, and uncertain vendor packages from automated proposals
        target_packages.retain(|p| {
            !p.is_protected
                && !crate::policy::SafetyPolicy::is_built_in_utility(&p.package_id)
                && !crate::policy::SafetyPolicy::is_uncertain_vendor_package(&p.package_id)
        });
    }

    let installed_package_ids: Vec<String> = target_packages
        .iter()
        .map(|p| p.package_id.clone())
        .collect();

    let cred =
        CredentialStore::get_credential(&profile.credential_id).map_err(|e| ClassifiedError {
            error_type: ai::ErrorType::Authentication,
            message: e,
            suggested_action: "Check Windows Credential Manager and save the credential again"
                .into(),
            diagnostic_details: None,
            http_status: None,
        })?;

    let req_id_clone = request_id.clone();
    let app_handle = app.clone();

    let on_chunk = move |text: String| {
        let _ = app_handle.emit(
            "ai-stream-chunk",
            StreamChunkPayload {
                request_id: req_id_clone.clone(),
                text,
                is_thought: false,
            },
        );
    };

    let model_to_use = profile
        .manual_model_id
        .as_deref()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(&profile.selected_model)
        .trim()
        .to_string();

    let stream_result = if profile.preset == storage::credentials::ConnectionPreset::FreeLlmApi
        || profile.id == ai::GATEWAY_CONNECTION_ID
        || profile.id == "preset-tz-gateway"
        || profile.id == "preset-freellmapi"
    {
        state
            .gateway
            .stream_chat_with_failover(
                &app,
                &request_id,
                &user_prompt,
                &dev_model,
                &android_ver,
                &installed_package_ids,
                &effective_history,
                &model_to_use,
                Arc::clone(&cancel_token),
                on_chunk,
            )
            .await
    } else {
        AiClient::stream_chat(
            &profile,
            cred.as_deref(),
            &model_to_use,
            &user_prompt,
            &dev_model,
            &android_ver,
            &installed_package_ids,
            &effective_history,
            Arc::clone(&cancel_token),
            on_chunk,
        )
        .await
    };

    // Clean up cancellation token
    {
        let mut tokens = state.cancellation_tokens.lock();
        tokens.remove(&request_id);
    }

    match stream_result {
        Ok((full_text, served_model)) => {
            if cancel_token.load(Ordering::SeqCst)
                || require_target(&state, &device_serial, user_id).is_err()
            {
                return Ok(());
            }
            // Extract structured ActionPlan only if changes were requested and JSON plan exists
            let mut plan_opt =
                AiClient::extract_plan_from_text(&full_text, &device_serial, user_id);

            // If a plan was extracted, run backend validation and sanitization
            if let Some(ref mut plan) = plan_opt {
                let installed_set: HashSet<String> = installed_package_ids.into_iter().collect();
                let active_launcher =
                    DeviceManager::get_active_launcher_for_user(&executor, &device_serial, user_id);
                let active_ime =
                    DeviceManager::get_active_ime_for_user(&executor, &device_serial, user_id);

                let _ = PlanValidator::validate_plan(
                    plan,
                    &device_serial,
                    user_id,
                    &installed_set,
                    active_launcher,
                    active_ime,
                    shortcut_intent,
                );
            }

            if let Some(ref plan) = plan_opt {
                let _ = app.emit(
                    "ai-plan-generated",
                    serde_json::json!({
                        "request_id": request_id,
                        "plan": plan
                    }),
                );
            }

            let _ = app.emit(
                "ai-stream-done",
                StreamDonePayload {
                    request_id: request_id.clone(),
                    finish_reason: "stop".into(),
                    served_model,
                    plan_id: plan_opt.as_ref().map(|p| p.id.clone()),
                },
            );

            Ok(())
        }
        Err(err) => {
            let _ = app.emit(
                "ai-stream-error",
                StreamErrorPayload {
                    request_id: request_id.clone(),
                    error: err.clone(),
                },
            );
            Err(err)
        }
    }
}

#[tauri::command]
fn cancel_device_operations(
    state: State<'_, AppState>,
    device_serial: String,
) -> Result<(), String> {
    if let Some(token) = state
        .cancellation_tokens
        .lock()
        .get(&format!("device:{device_serial}"))
    {
        token.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
fn cancel_ai_request(state: State<'_, AppState>, request_id: String) -> Result<(), String> {
    let mut tokens = state.cancellation_tokens.lock();
    if let Some(token) = tokens.remove(&request_id) {
        token.store(true, Ordering::SeqCst);
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Plan Validation, Approval Session & Protected Execution Commands
// -----------------------------------------------------------------------------

#[tauri::command]
async fn validate_plan(
    state: State<'_, AppState>,
    mut plan: ActionPlan,
    current_serial: String,
    current_user_id: u32,
) -> Result<PlanValidationResult, String> {
    let executor = get_executor(&state)?;
    let packages =
        PackageManager::scan_device_packages(&executor, &current_serial, current_user_id)?;
    let set: HashSet<String> = packages
        .iter()
        .filter(|p| p.state != crate::adb::package::PackageState::Uninstalled)
        .map(|p| p.package_id.clone())
        .collect();
    let launcher =
        DeviceManager::get_active_launcher_for_user(&executor, &current_serial, current_user_id);
    let ime = DeviceManager::get_active_ime_for_user(&executor, &current_serial, current_user_id);

    Ok(PlanValidator::validate_plan(
        &mut plan,
        &current_serial,
        current_user_id,
        &set,
        launcher,
        ime,
        None,
    ))
}

#[tauri::command]
async fn request_plan_approval(
    state: State<'_, AppState>,
    mut plan: ActionPlan,
) -> Result<PlanApprovalSession, String> {
    require_target(&state, &plan.target_device_serial, plan.target_user_id)?;
    let executor = get_executor(&state)?;
    let packages = PackageManager::scan_device_packages(
        &executor,
        &plan.target_device_serial,
        plan.target_user_id,
    )?;

    let installed_set: HashSet<String> = packages
        .iter()
        .filter(|p| p.state != crate::adb::package::PackageState::Uninstalled)
        .map(|p| p.package_id.clone())
        .collect();
    let launcher = DeviceManager::get_active_launcher_for_user(
        &executor,
        &plan.target_device_serial,
        plan.target_user_id,
    );
    let ime = DeviceManager::get_active_ime_for_user(
        &executor,
        &plan.target_device_serial,
        plan.target_user_id,
    );

    let target_serial = plan.target_device_serial.clone();
    let target_user = plan.target_user_id;

    let val = PlanValidator::validate_plan(
        &mut plan,
        &target_serial,
        target_user,
        &installed_set,
        launcher,
        ime,
        None,
    );

    if !val.is_valid {
        return Err(format!(
            "Plan cannot be approved: {}",
            val.errors.join("; ")
        ));
    }

    let approved_actions: Vec<ProposedAction> = plan
        .actions
        .into_iter()
        .filter(|a| a.selected && a.is_executable)
        .collect();

    if approved_actions.is_empty() {
        return Err("Cannot approve an action plan with no selected executable actions.".into());
    }

    let inventory_hash = compute_inventory_hash(&packages);
    let token = format!("appr_{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().timestamp();

    let session = PlanApprovalSession {
        approval_token: token,
        plan_id: plan.id,
        revision: plan.revision,
        target_device_serial: plan.target_device_serial,
        target_user_id: plan.target_user_id,
        inventory_hash,
        approved_actions,
        created_at: now,
        expires_at: now + 900, // 15 minute TTL
    };

    *state.active_approval_session.lock() = Some(session.clone());
    Ok(session)
}

#[tauri::command]
async fn execute_action_plan(
    app: AppHandle,
    state: State<'_, AppState>,
    approval_token: String,
    plan_id: String,
    revision: u64,
    continue_on_independent_failure: Option<bool>,
) -> Result<Vec<OperationOutcome>, String> {
    // 1. Consume and validate approval session atomically
    let session = {
        let mut session_guard = state.active_approval_session.lock();
        match session_guard.take() {
            Some(sess) => {
                if sess.approval_token != approval_token {
                    return Err("Approval token mismatch. Plan approval is invalid.".into());
                }
                if sess.plan_id != plan_id || sess.revision != revision {
                    return Err("Plan revision or identity mismatch. Re-approval required.".into());
                }
                let now = chrono::Utc::now().timestamp();
                if now > sess.expires_at {
                    return Err("Plan approval has expired. Please review and re-approve.".into());
                }
                sess
            }
            None => {
                return Err("No active backend approval found for this plan. Explicit user review and approval required.".into());
            }
        }
    };

    require_target(
        &state,
        &session.target_device_serial,
        session.target_user_id,
    )?;
    let executor = get_executor(&state)?;

    // 2. Acquire per-device mutation lock to prevent overlapping executions
    let device_lock = state.get_device_mutation_lock(&session.target_device_serial);
    let _guard = device_lock.lock().await;
    require_target(
        &state,
        &session.target_device_serial,
        session.target_user_id,
    )?;

    if chrono::Utc::now().timestamp() >= session.expires_at {
        return Err("Approval expired while waiting for the device lock".into());
    }
    // 3. Verify target device and inventory state consistency
    let current_packages = PackageManager::scan_device_packages(
        &executor,
        &session.target_device_serial,
        session.target_user_id,
    )?;
    let current_hash = compute_inventory_hash(&current_packages);
    if current_hash != session.inventory_hash {
        return Err("Device package inventory changed since plan approval. Approval was invalidated for device safety.".into());
    }

    // 4. Save pre-change snapshot atomically. Must succeed before any mutation!
    let devices = DeviceManager::list_devices(&executor).unwrap_or_default();
    let model = devices
        .into_iter()
        .find(|d| d.serial == session.target_device_serial)
        .map(|d| d.model)
        .unwrap_or_else(|| "Unknown".into());

    let snapshot = DeviceSnapshot {
        snapshot_id: uuid::Uuid::new_v4().to_string(),
        device_serial: session.target_device_serial.clone(),
        device_model: model,
        user_id: session.target_user_id,
        timestamp: chrono::Utc::now().timestamp(),
        note: format!("Pre-execution snapshot for plan {}", session.plan_id),
        package_states: current_packages
            .iter()
            .map(|p| PackageSnapshotEntry {
                package_id: p.package_id.clone(),
                state: p.state,
                is_system: p.is_system,
            })
            .collect(),
    };

    JournalManager::save_snapshot(&snapshot).map_err(|e| {
        format!("Pre-mutation snapshot persistence failed. Aborting execution: {e}")
    })?;

    // 5. Execute approved actions sequentially with two-phase journaling and progress events
    let mut outcomes = Vec::new();
    let total_actions = session.approved_actions.len();

    let cancel_key = format!("device:{}", session.target_device_serial);
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancellation_tokens
        .lock()
        .insert(cancel_key.clone(), cancel.clone());
    for (index, action) in session.approved_actions.iter().enumerate() {
        if cancel.load(Ordering::SeqCst)
            || require_target(
                &state,
                &session.target_device_serial,
                session.target_user_id,
            )
            .is_err()
        {
            break;
        }
        let _ = app.emit(
            "plan-execution-progress",
            serde_json::json!({
                "current": index + 1,
                "total": total_actions,
                "package_id": action.package_id,
                "action": action.proposed_action.display_name(),
            }),
        );

        // Re-check live package state immediately prior to executing this action
        if let Ok(Some(current_live_state)) = PackageManager::query_single_package_state(
            &executor,
            &session.target_device_serial,
            session.target_user_id,
            &action.package_id,
        ) {
            if action.proposed_action == ActionType::UninstallUser && current_live_state == crate::adb::package::PackageState::Uninstalled {
                outcomes.push(OperationOutcome {
                    package_id: action.package_id.clone(),
                    action: action.proposed_action,
                    success: true,
                    verified: true,
                    exit_code: Some(0),
                    previous_state: Some(crate::adb::package::PackageState::Uninstalled),
                    resulting_state: Some(crate::adb::package::PackageState::Uninstalled),
                    message: format!("Already uninstalled for User {} (redundant uninstall skipped)", session.target_user_id),
                    classified_cause: ClassifiedCause::None,
                    execution_backend: "DirectAdbPm".into(),
                    command_args: Vec::new(),
                    target_device: session.target_device_serial.clone(),
                    target_user_id: session.target_user_id,
                    raw_stdout: None,
                    raw_stderr: None,
                });
                continue;
            }
            if action.proposed_action == ActionType::Disable && current_live_state == crate::adb::package::PackageState::Disabled {
                outcomes.push(OperationOutcome {
                    package_id: action.package_id.clone(),
                    action: action.proposed_action,
                    success: true,
                    verified: true,
                    exit_code: Some(0),
                    previous_state: Some(crate::adb::package::PackageState::Disabled),
                    resulting_state: Some(crate::adb::package::PackageState::Disabled),
                    message: format!("Already disabled for User {} (redundant disable skipped)", session.target_user_id),
                    classified_cause: ClassifiedCause::None,
                    execution_backend: "DirectAdbPm".into(),
                    command_args: Vec::new(),
                    target_device: session.target_device_serial.clone(),
                    target_user_id: session.target_user_id,
                    raw_stdout: None,
                    raw_stderr: None,
                });
                continue;
            }
        }

        let outcome = match execution::run_recorded(
            &executor,
            &session.target_device_serial,
            session.target_user_id,
            &action.package_id,
            action.proposed_action,
        ) {
            Ok(o) => o,
            Err(e) => OperationOutcome {
                package_id: action.package_id.clone(),
                action: action.proposed_action,
                success: false,
                verified: false,
                exit_code: None,
                previous_state: None,
                resulting_state: None,
                message: e.clone(),
                classified_cause: ClassifiedCause::Unclassified { summary: e },
                execution_backend: "DirectAdbPm".into(),
                command_args: Vec::new(),
                target_device: session.target_device_serial.clone(),
                target_user_id: session.target_user_id,
                raw_stdout: None,
                raw_stderr: None,
            },
        };
        let success = outcome.success && outcome.verified;
        let is_critical = match &outcome.classified_cause {
            ClassifiedCause::DeviceOffline => true,
            ClassifiedCause::InvalidProfileTarget { .. } => true,
            _ => {
                outcome.message.contains("disconnected")
                    || outcome.message.contains("inaccessible profile")
                    || outcome.message.contains("device lock")
            }
        };
        outcomes.push(outcome);

        if !success {
            let allow_continue = continue_on_independent_failure.unwrap_or(false);
            if is_critical || !allow_continue {
                // Stop on critical failures or when continue policy is not enabled
                break;
            }
        }
    }

    state.cancellation_tokens.lock().remove(&cancel_key);

    // Record skipped actions if batch stopped before completion
    let completed_count = outcomes.len();
    if completed_count < total_actions {
        for unattempted in &session.approved_actions[completed_count..] {
            outcomes.push(OperationOutcome {
                package_id: unattempted.package_id.clone(),
                action: unattempted.proposed_action,
                success: false,
                verified: false,
                exit_code: None,
                previous_state: None,
                resulting_state: None,
                message: "Skipped / not attempted (prior action stopped execution).".into(),
                classified_cause: ClassifiedCause::Unclassified {
                    summary: "Skipped: prior action stopped execution".into(),
                },
                execution_backend: "None".into(),
                command_args: Vec::new(),
                target_device: session.target_device_serial.clone(),
                target_user_id: session.target_user_id,
                raw_stdout: None,
                raw_stderr: None,
            });
        }
    }

    let attempted_count = completed_count;
    let succeeded_count = outcomes.iter().filter(|o| o.success && o.verified).count();
    let failed_count = outcomes
        .iter()
        .take(completed_count)
        .filter(|o| !o.success || !o.verified)
        .count();
    let skipped_count = outcomes.len() - completed_count;
    let unknown_count = outcomes
        .iter()
        .filter(|o| o.message.contains("uncertain"))
        .count();

    let _ = app.emit(
        "plan-execution-complete",
        serde_json::json!({
            "total": total_actions,
            "completed": completed_count,
            "attempted_count": attempted_count,
            "succeeded_count": succeeded_count,
            "failed_count": failed_count,
            "skipped_count": skipped_count,
            "unknown_count": unknown_count,
            "success_count": succeeded_count,
        }),
    );

    Ok(outcomes)
}

#[tauri::command]
fn create_retry_plan(
    original_plan: ActionPlan,
    remaining_package_ids: Vec<String>,
) -> Result<ActionPlan, String> {
    let mut retry_actions = Vec::new();
    let id_set: HashSet<String> = remaining_package_ids.into_iter().collect();

    for act in original_plan.actions {
        if id_set.contains(&act.package_id) {
            retry_actions.push(act);
        }
    }

    if retry_actions.is_empty() {
        return Err("No eligible remaining actions found to create a retry plan.".into());
    }

    Ok(ActionPlan {
        id: format!("retry-{}", uuid::Uuid::new_v4()),
        revision: 1,
        target_device_serial: original_plan.target_device_serial,
        target_user_id: original_plan.target_user_id,
        explanation: format!(
            "Reviewed retry plan for {} remaining actions that were not completed or skipped in the previous execution run.",
            retry_actions.len()
        ),
        preserved_functionality: original_plan.preserved_functionality,
        actions: retry_actions,
        created_at: chrono::Utc::now().timestamp(),
        approved_by_user: false,
        is_valid: true,
        invalidation_reason: None,
    })
}

#[tauri::command]
async fn execute_package_action(
    state: State<'_, AppState>,
    device_serial: String,
    user_id: u32,
    package_id: String,
    action: ActionType,
) -> Result<OperationOutcome, String> {
    let executor = get_executor(&state)?;

    let device_lock = state.get_device_mutation_lock(&device_serial);
    let _guard = device_lock.lock().await;
    require_target(&state, &device_serial, user_id)?;
    execution::run_recorded(&executor, &device_serial, user_id, &package_id, action)
}

#[tauri::command]
async fn validate_batch_selection(
    state: State<'_, AppState>,
    device_serial: String,
    user_id: u32,
    action: ActionType,
    package_ids: Vec<String>,
) -> Result<BatchPreflightResult, String> {
    let executor = get_executor(&state)?;
    require_target(&state, &device_serial, user_id)?;

    let scanned_packages = PackageManager::scan_device_packages(&executor, &device_serial, user_id)?;
    let pkg_map: HashMap<String, DevicePackage> = scanned_packages
        .into_iter()
        .map(|p| (p.package_id.clone(), p))
        .collect();

    let mut items = Vec::with_capacity(package_ids.len());
    let mut eligible_count = 0;
    let mut excluded_count = 0;

    for pkg_id in &package_ids {
        if let Some(pkg) = pkg_map.get(pkg_id) {
            let mut eligible = true;
            let mut exclusion_reason = None;

            match action {
                ActionType::UninstallUser => {
                    if pkg.state == PackageState::Uninstalled {
                        eligible = false;
                        exclusion_reason = Some("Package is already uninstalled for this user".to_string());
                    } else if pkg.is_protected {
                        eligible = false;
                        exclusion_reason = Some(format!(
                            "Protected component: {}",
                            pkg.protection_reason.as_deref().unwrap_or("Critical system package")
                        ));
                    } else if policy::SafetyPolicy::is_confirmed_critical_vendor_service(&pkg.package_id) {
                        eligible = false;
                        exclusion_reason = Some("Confirmed critical telephony/modem/radio vendor service".to_string());
                    }
                }
                ActionType::Disable => {
                    if pkg.state == PackageState::Disabled {
                        eligible = false;
                        exclusion_reason = Some("Package is already disabled for this user".to_string());
                    } else if pkg.state == PackageState::Uninstalled {
                        eligible = false;
                        exclusion_reason = Some("Cannot disable package that is uninstalled for this user (restore first)".to_string());
                    } else if pkg.is_protected {
                        eligible = false;
                        exclusion_reason = Some(format!(
                            "Protected component: {}",
                            pkg.protection_reason.as_deref().unwrap_or("Critical system package")
                        ));
                    } else if policy::SafetyPolicy::is_confirmed_critical_vendor_service(&pkg.package_id) {
                        eligible = false;
                        exclusion_reason = Some("Confirmed critical telephony/modem/radio vendor service".to_string());
                    }
                }
                ActionType::Enable => {
                    if pkg.state == PackageState::Enabled {
                        eligible = false;
                        exclusion_reason = Some("Package is already enabled".to_string());
                    } else if pkg.state == PackageState::Uninstalled {
                        eligible = false;
                        exclusion_reason = Some("Cannot enable package that is uninstalled for this user (restore first)".to_string());
                    }
                }
                ActionType::Restore => {
                    if pkg.state != PackageState::Uninstalled {
                        eligible = false;
                        exclusion_reason = Some("Package is already installed on device".to_string());
                    }
                }
            }

            if eligible {
                eligible_count += 1;
            } else {
                excluded_count += 1;
            }

            items.push(BatchItemValidation {
                package_id: pkg.package_id.clone(),
                package_type: pkg.package_type,
                current_state: pkg.state,
                action,
                target_user: user_id,
                eligible,
                exclusion_reason,
            });
        } else {
            excluded_count += 1;
            items.push(BatchItemValidation {
                package_id: pkg_id.clone(),
                package_type: AndroidPackageType::User,
                current_state: PackageState::Uninstalled,
                action,
                target_user: user_id,
                eligible: false,
                exclusion_reason: Some("Package not found on target device for this user profile".to_string()),
            });
        }
    }

    Ok(BatchPreflightResult {
        total_requested: package_ids.len(),
        eligible_count,
        excluded_count,
        items,
    })
}

// -----------------------------------------------------------------------------
// Snapshot & Journal Commands
// -----------------------------------------------------------------------------

#[tauri::command]
async fn create_snapshot(
    state: State<'_, AppState>,
    device_serial: String,
    user_id: u32,
    note: String,
) -> Result<String, String> {
    let executor = get_executor(&state)?;
    let packages = PackageManager::scan_device_packages(&executor, &device_serial, user_id)?;
    let devices = DeviceManager::list_devices(&executor).unwrap_or_default();
    let model = devices
        .into_iter()
        .find(|d| d.serial == device_serial)
        .map(|d| d.model)
        .unwrap_or_else(|| "Unknown".into());

    let snapshot = DeviceSnapshot {
        snapshot_id: uuid::Uuid::new_v4().to_string(),
        device_serial,
        device_model: model,
        user_id,
        timestamp: chrono::Utc::now().timestamp(),
        note,
        package_states: packages
            .iter()
            .map(|p| PackageSnapshotEntry {
                package_id: p.package_id.clone(),
                state: p.state,
                is_system: p.is_system,
            })
            .collect(),
    };

    JournalManager::save_snapshot(&snapshot)
}

#[tauri::command]
fn list_snapshots() -> Result<Vec<DeviceSnapshot>, String> {
    JournalManager::list_snapshots()
}

#[tauri::command]
fn get_journal_history() -> Result<Vec<JournalEntry>, String> {
    JournalManager::read_journal()
}

#[tauri::command]
fn clear_journal_history() -> Result<(), String> {
    JournalManager::clear_journal()
}

#[tauri::command]
async fn propose_recovery_action(
    state: State<'_, AppState>,
    entry_id: String,
    current_serial: String,
    current_user_id: u32,
) -> Result<RecoveryProposal, String> {
    let entries = JournalManager::read_journal()?;
    let entry = entries
        .into_iter()
        .find(|e| e.entry_id == entry_id)
        .ok_or_else(|| format!("Journal entry '{entry_id}' not found"))?;

    let executor = get_executor(&state)?;
    let current_state = PackageManager::query_single_package_state(
        &executor,
        &current_serial,
        current_user_id,
        &entry.package_id,
    )
    .unwrap_or(None);

    Ok(JournalManager::propose_recovery(
        &entry,
        &current_serial,
        current_user_id,
        current_state,
    ))
}

#[tauri::command]
async fn execute_recovery(
    state: State<'_, AppState>,
    entry_id: String,
    current_serial: String,
    current_user_id: u32,
) -> Result<Vec<OperationOutcome>, String> {
    let lock = state.get_device_mutation_lock(&current_serial);
    let _guard = lock.lock().await;
    require_target(&state, &current_serial, current_user_id)?;
    let entry = JournalManager::read_journal()?
        .into_iter()
        .find(|e| e.entry_id == entry_id)
        .ok_or("Journal entry not found")?;
    let executor = get_executor(&state)?;
    let current = PackageManager::query_single_package_state(
        &executor,
        &current_serial,
        current_user_id,
        &entry.package_id,
    )?;
    let proposal =
        JournalManager::propose_recovery(&entry, &current_serial, current_user_id, current);
    if !proposal.can_recover {
        return Err(proposal
            .blockage_reason
            .unwrap_or("Recovery unavailable".into()));
    }
    let mut outcomes = Vec::new();
    for step in proposal.steps {
        require_target(&state, &current_serial, current_user_id)?;
        let outcome = execution::run_recorded(
            &executor,
            &current_serial,
            current_user_id,
            &proposal.package_id,
            step,
        )?;
        let stop = !outcome.success || !outcome.verified;
        outcomes.push(outcome);
        if stop {
            break;
        }
    }

    // Verify final requested state was achieved
    let final_state = PackageManager::query_single_package_state(
        &executor,
        &current_serial,
        current_user_id,
        &proposal.package_id,
    )?;
    if final_state != Some(proposal.target_state) {
        return Err(format!(
            "Recovery completed steps but final package state is {:?}, expected {:?}",
            final_state, proposal.target_state
        ));
    }

    Ok(outcomes)
}

// -----------------------------------------------------------------------------
// Application Entrypoint
// -----------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = ConfigStore::load_config();
    let gateway = Arc::new(ai::gateway::TzAiGateway::new());
    gateway.sync_profiles(&initial_config.connections);

    let state = AppState {
        config: RwLock::new(initial_config),
        active_target: Mutex::new(None),
        active_approval_session: Mutex::new(None),
        cancellation_tokens: Mutex::new(HashMap::new()),
        device_mutation_locks: Mutex::new(HashMap::new()),
        gateway,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            set_active_target,
            export_profile,
            import_profile,
            get_adb_status,
            download_platform_tools,
            list_devices,
            pair_wireless_adb,
            connect_wireless_adb,
            disconnect_wireless_adb,
            list_users,
            scan_packages,
            get_dataset_stats,
            get_app_config,
            save_app_config,
            save_credential,
            remove_credential,
            approve_destination,
            test_saved_credential,
            test_new_credential,
            discover_models,
            get_gateway_status,
            check_gateway_readiness,
            test_model_generation,
            stream_ai_chat,
            cancel_ai_request,
            cancel_device_operations,
            validate_plan,
            request_plan_approval,
            execute_action_plan,
            execute_package_action,
            validate_batch_selection,
            create_retry_plan,
            create_snapshot,
            list_snapshots,
            get_journal_history,
            clear_journal_history,
            propose_recovery_action,
            execute_recovery,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TZ AI DEBLOATER TOOL application");
}
