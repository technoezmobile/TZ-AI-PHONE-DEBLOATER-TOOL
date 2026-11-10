use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionMode {
    DirectApi,
    Gateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionPreset {
    GoogleGemini,
    OpenAi,
    Anthropic,
    FreeLlmApi,
    CustomGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiProtocol {
    GeminiNative,
    OpenAiChatCompletions,
    OpenAiResponses,
    AnthropicMessages,
}

impl std::fmt::Display for ApiProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GeminiNative => write!(f, "GeminiNative"),
            Self::OpenAiChatCompletions => write!(f, "OpenAiChatCompletions"),
            Self::OpenAiResponses => write!(f, "OpenAiResponses"),
            Self::AnthropicMessages => write!(f, "AnthropicMessages"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub mode: ConnectionMode,
    pub preset: ConnectionPreset,
    pub base_url: String,
    pub protocol: ApiProtocol,
    pub credential_id: String,
    pub selected_model: String,
    pub manual_model_id: Option<String>,
    pub last_test_status: Option<String>,
    pub last_verified_at: Option<String>,
    pub supported_capabilities: Vec<String>,
    pub tested_for_generation: bool,
    pub has_credential: bool,
}

impl ConnectionProfile {
    pub fn new_gemini() -> Self {
        Self {
            id: "preset-gemini".into(),
            name: "Google Gemini (Direct)".into(),
            mode: ConnectionMode::DirectApi,
            preset: ConnectionPreset::GoogleGemini,
            base_url: "https://generativelanguage.googleapis.com".into(),
            protocol: ApiProtocol::GeminiNative,
            credential_id: "preset-gemini".into(),
            selected_model: "gemini-3.5-flash".into(),
            manual_model_id: None,
            last_test_status: None,
            last_verified_at: None,
            supported_capabilities: vec!["chat".into(), "streaming".into(), "model_listing".into()],
            tested_for_generation: false,
            has_credential: false,
        }
    }

    pub fn new_openai() -> Self {
        Self {
            id: "preset-openai".into(),
            name: "OpenAI (Direct)".into(),
            mode: ConnectionMode::DirectApi,
            preset: ConnectionPreset::OpenAi,
            base_url: "https://api.openai.com/v1".into(),
            protocol: ApiProtocol::OpenAiChatCompletions,
            credential_id: "preset-openai".into(),
            selected_model: "".into(),
            manual_model_id: None,
            last_test_status: None,
            last_verified_at: None,
            supported_capabilities: vec!["chat".into(), "streaming".into(), "model_listing".into()],
            tested_for_generation: false,
            has_credential: false,
        }
    }

    pub fn new_anthropic() -> Self {
        Self {
            id: "preset-anthropic".into(),
            name: "Anthropic (Direct)".into(),
            mode: ConnectionMode::DirectApi,
            preset: ConnectionPreset::Anthropic,
            base_url: "https://api.anthropic.com".into(),
            protocol: ApiProtocol::AnthropicMessages,
            credential_id: "preset-anthropic".into(),
            selected_model: "".into(),
            manual_model_id: None,
            last_test_status: None,
            last_verified_at: None,
            supported_capabilities: vec!["chat".into(), "streaming".into()],
            tested_for_generation: false,
            has_credential: false,
        }
    }

    pub fn new_freellmapi() -> Self {
        Self::new_tz_gateway()
    }

    pub fn new_tz_gateway() -> Self {
        Self {
            id: "preset-tz-gateway".into(),
            name: "TZ AI Gateway (Integrated Router)".into(),
            mode: ConnectionMode::Gateway,
            preset: ConnectionPreset::FreeLlmApi,
            base_url: "http://127.0.0.1:3001/v1".into(),
            protocol: ApiProtocol::OpenAiChatCompletions,
            credential_id: "preset-tz-gateway".into(),
            selected_model: "gateway:auto".into(),
            manual_model_id: None,
            last_test_status: None,
            last_verified_at: None,
            supported_capabilities: vec![
                "chat".into(),
                "streaming".into(),
                "model_listing".into(),
                "readiness".into(),
                "auto_routing".into(),
                "failover".into(),
            ],
            tested_for_generation: false,
            has_credential: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub custom_adb_path: Option<String>,
    pub active_connection_id: String,
    pub connections: Vec<ConnectionProfile>,
    pub ai_consent_given: bool,
    pub approved_destinations: Vec<String>,
    pub auto_refresh_on_connect: bool,
    pub confirm_each_action: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let gemini = ConnectionProfile::new_gemini();
        let openai = ConnectionProfile::new_openai();
        let anthropic = ConnectionProfile::new_anthropic();
        let freellm = ConnectionProfile::new_freellmapi();

        Self {
            custom_adb_path: None,
            active_connection_id: gemini.id.clone(),
            connections: vec![gemini, openai, anthropic, freellm],
            ai_consent_given: false,
            approved_destinations: vec![
                "https://generativelanguage.googleapis.com".into(),
                "https://api.openai.com/v1".into(),
                "https://api.anthropic.com".into(),
                "localhost:3001".into(),
                "127.0.0.1:3001".into(),
            ],
            auto_refresh_on_connect: true,
            confirm_each_action: true,
        }
    }
}

// ---------------------------------------------------------------------------
// OS-Backed Secure Credential Storage
// ---------------------------------------------------------------------------

#[cfg(windows)]
pub mod os_keyring {
    use std::ptr;

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;

    #[repr(C)]
    struct CREDENTIALW {
        flags: u32,
        cred_type: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: [u32; 2],
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut std::ffi::c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const CREDENTIALW, flags: u32) -> i32;
        fn CredReadW(
            target_name: *const u16,
            cred_type: u32,
            flags: u32,
            credential: *mut *mut CREDENTIALW,
        ) -> i32;
        fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut std::ffi::c_void);
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn save(target: &str, secret: &str) -> Result<(), String> {
        let target_w = to_wide(target);
        let user_w = to_wide("TZ_AI_DEBLOATER_USER");
        let secret_bytes = secret.as_bytes();

        let cred = CREDENTIALW {
            flags: 0,
            cred_type: CRED_TYPE_GENERIC,
            target_name: target_w.as_ptr() as *mut u16,
            comment: ptr::null_mut(),
            last_written: [0, 0],
            credential_blob_size: secret_bytes.len() as u32,
            credential_blob: secret_bytes.as_ptr() as *mut u8,
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: ptr::null_mut(),
            target_alias: ptr::null_mut(),
            user_name: user_w.as_ptr() as *mut u16,
        };

        unsafe {
            if CredWriteW(&cred, 0) == 0 {
                let err = std::io::Error::last_os_error();
                return Err(format!("Windows Credential Manager save failed: {err}"));
            }
        }
        Ok(())
    }

    pub fn read(target: &str) -> Result<Option<String>, String> {
        let target_w = to_wide(target);
        let mut p_cred: *mut CREDENTIALW = ptr::null_mut();

        unsafe {
            if CredReadW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p_cred) == 0 {
                let err = std::io::Error::last_os_error();
                // 1168 = ERROR_NOT_FOUND
                if err.raw_os_error() == Some(1168) {
                    return Ok(None);
                }
                return Err(format!("Windows Credential Manager read failed: {err}"));
            }

            if p_cred.is_null() {
                return Ok(None);
            }

            let blob_size = (*p_cred).credential_blob_size as usize;
            let blob_ptr = (*p_cred).credential_blob;
            if blob_ptr.is_null() || blob_size == 0 {
                CredFree(p_cred as *mut std::ffi::c_void);
                return Ok(None);
            }
            let bytes = std::slice::from_raw_parts(blob_ptr, blob_size).to_vec();
            CredFree(p_cred as *mut std::ffi::c_void);
            Ok(Some(String::from_utf8(bytes).map_err(|_| {
                "Stored credential is not valid UTF-8".to_string()
            })?))
        }
    }

    pub fn delete(target: &str) -> Result<(), String> {
        let target_w = to_wide(target);
        unsafe {
            if CredDeleteW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0) == 0 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(1168) {
                    return Ok(()); // Already deleted
                }
                return Err(format!("Windows Credential Manager delete failed: {err}"));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// macOS Keychain Integration
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod macos_keychain {
    use std::process::Command;

    const ACCOUNT_NAME: &str = "TZ_AI_DEBLOATER_USER";

    pub fn save(target: &str, secret: &str) -> Result<(), String> {
        let status = Command::new("/usr/bin/security")
            .args(&[
                "add-generic-password",
                "-a",
                ACCOUNT_NAME,
                "-s",
                target,
                "-w",
                secret,
                "-U",
            ])
            .status()
            .map_err(|e| format!("Failed to invoke macOS security tool: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("macOS security tool exited with status {status}"))
        }
    }

    pub fn read(target: &str) -> Result<Option<String>, String> {
        let output = Command::new("/usr/bin/security")
            .args(&["find-generic-password", "-a", ACCOUNT_NAME, "-s", target, "-w"])
            .output()
            .map_err(|e| format!("Failed to query macOS security tool: {e}"))?;

        if output.status.success() {
            let secret = String::from_utf8(output.stdout)
                .map_err(|_| "Stored macOS credential is not valid UTF-8".to_string())?;
            Ok(Some(secret.trim().to_string()))
        } else {
            // Error 44 = errSecItemNotFound
            Ok(None)
        }
    }

    pub fn delete(target: &str) -> Result<(), String> {
        let _ = Command::new("/usr/bin/security")
            .args(&["delete-generic-password", "-a", ACCOUNT_NAME, "-s", target])
            .status();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Linux Secret Service / Restricted File Vault
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux_keyring {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    const SERVICE_NAME: &str = "tz-ai-debloater";

    fn get_file_vault_path(target: &str) -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| {
            d.join("TZ_AI_Debloater")
                .join("vault")
                .join(format!("{target}.cred"))
        })
    }

    pub fn save(target: &str, secret: &str) -> Result<(), String> {
        // 1. Try secret-tool if available
        if which::which("secret-tool").is_ok() {
            use std::io::Write;
            let mut child = Command::new("secret-tool")
                .args(&["store", "--label=TZ AI Debloater Secret", "service", SERVICE_NAME, "account", target])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn secret-tool: {e}"))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(secret.as_bytes());
            }

            if let Ok(status) = child.wait() {
                if status.success() {
                    return Ok(());
                }
            }
        }

        // 2. Fallback to restricted permissions file in local app directory
        if let Some(path) = get_file_vault_path(target) {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
                }
            }
            fs::write(&path, secret).map_err(|e| format!("Failed to write secure file: {e}"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
            return Ok(());
        }

        Err("No available secure storage found on Linux system".into())
    }

    pub fn read(target: &str) -> Result<Option<String>, String> {
        // 1. Try secret-tool if available
        if which::which("secret-tool").is_ok() {
            if let Ok(output) = Command::new("secret-tool")
                .args(&["lookup", "service", SERVICE_NAME, "account", target])
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !s.is_empty() {
                        return Ok(Some(s));
                    }
                }
            }
        }

        // 2. Fallback to restricted permissions file
        if let Some(path) = get_file_vault_path(target) {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let s = content.trim().to_string();
                    if !s.is_empty() {
                        return Ok(Some(s));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn delete(target: &str) -> Result<(), String> {
        if which::which("secret-tool").is_ok() {
            let _ = Command::new("secret-tool")
                .args(&["clear", "service", SERVICE_NAME, "account", target])
                .status();
        }
        if let Some(path) = get_file_vault_path(target) {
            let _ = fs::remove_file(&path);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Credential Storage Abstraction & Implementations
// ---------------------------------------------------------------------------

pub trait CredentialStorage: Send + Sync {
    fn save(&self, credential_id: &str, secret: &str) -> Result<(), String>;
    fn get(&self, credential_id: &str) -> Result<Option<String>, String>;
    fn delete(&self, credential_id: &str) -> Result<(), String>;
    fn has(&self, credential_id: &str) -> bool;
}

#[derive(Default)]
pub struct InMemoryCredentialStore {
    store: std::sync::RwLock<HashMap<String, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            store: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl CredentialStorage for InMemoryCredentialStore {
    fn save(&self, credential_id: &str, secret: &str) -> Result<(), String> {
        let mut guard = self.store.write().map_err(|e| e.to_string())?;
        guard.insert(credential_id.to_string(), secret.to_string());
        Ok(())
    }

    fn get(&self, credential_id: &str) -> Result<Option<String>, String> {
        let guard = self.store.read().map_err(|e| e.to_string())?;
        Ok(guard.get(credential_id).cloned())
    }

    fn delete(&self, credential_id: &str) -> Result<(), String> {
        let mut guard = self.store.write().map_err(|e| e.to_string())?;
        guard.remove(credential_id);
        Ok(())
    }

    fn has(&self, credential_id: &str) -> bool {
        match self.get(credential_id) {
            Ok(Some(s)) => !s.trim().is_empty(),
            _ => false,
        }
    }
}

pub struct CredentialStore;

#[cfg(not(test))]
impl CredentialStore {
    pub fn backend_name() -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "Windows Credential Manager (Native DPAPI)"
        }
        #[cfg(target_os = "macos")]
        {
            "macOS Keychain (Generic Password)"
        }
        #[cfg(target_os = "linux")]
        {
            "Linux Secret Service / Restricted File Store"
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "Environment Variables"
        }
    }

    fn format_target(credential_id: &str) -> String {
        format!("TZ_AI_DEBLOATER_{credential_id}")
    }

    pub fn save_credential(credential_id: &str, secret: &str) -> Result<(), String> {
        let target = Self::format_target(credential_id);
        #[cfg(target_os = "windows")]
        {
            os_keyring::save(&target, secret)
        }
        #[cfg(target_os = "macos")]
        {
            macos_keychain::save(&target, secret)
        }
        #[cfg(target_os = "linux")]
        {
            linux_keyring::save(&target, secret)
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err("Unsupported platform for native credential vault. Please use environment variables (e.g. TZ_AI_API_KEY).".into())
        }
    }

    pub fn get_credential(credential_id: &str) -> Result<Option<String>, String> {
        let target = Self::format_target(credential_id);

        // 1. Check native OS keyring
        let os_secret = {
            #[cfg(target_os = "windows")]
            {
                os_keyring::read(&target)?
            }
            #[cfg(target_os = "macos")]
            {
                macos_keychain::read(&target)?
            }
            #[cfg(target_os = "linux")]
            {
                linux_keyring::read(&target)?
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                None
            }
        };

        if let Some(s) = os_secret {
            if !s.trim().is_empty() {
                return Ok(Some(s));
            }
        }

        // 2. Check environment variable fallback (especially useful for headless CI, terminal sessions)
        if let Ok(key) = std::env::var("TZ_AI_API_KEY") {
            if !key.trim().is_empty() {
                return Ok(Some(key.trim().to_string()));
            }
        }

        if credential_id == "preset-gemini" || credential_id.contains("gemini") {
            if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                if !key.trim().is_empty() {
                    return Ok(Some(key.trim().to_string()));
                }
            }
        } else if credential_id == "preset-openai" || credential_id.contains("openai") {
            if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                if !key.trim().is_empty() {
                    return Ok(Some(key.trim().to_string()));
                }
            }
        } else if credential_id == "preset-anthropic" || credential_id.contains("anthropic") {
            if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                if !key.trim().is_empty() {
                    return Ok(Some(key.trim().to_string()));
                }
            }
        }

        Ok(None)
    }

    pub fn delete_credential(credential_id: &str) -> Result<(), String> {
        let target = Self::format_target(credential_id);
        #[cfg(target_os = "windows")]
        {
            os_keyring::delete(&target)
        }
        #[cfg(target_os = "macos")]
        {
            macos_keychain::delete(&target)
        }
        #[cfg(target_os = "linux")]
        {
            linux_keyring::delete(&target)
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(())
        }
    }

    pub fn has_credential(credential_id: &str) -> bool {
        match Self::get_credential(credential_id) {
            Ok(Some(s)) => !s.trim().is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
static TEST_GLOBAL_STORE: std::sync::LazyLock<InMemoryCredentialStore> =
    std::sync::LazyLock::new(InMemoryCredentialStore::new);

#[cfg(test)]
impl CredentialStore {
    pub fn backend_name() -> &'static str {
        "In-Memory Test Keyring"
    }

    pub fn save_credential(credential_id: &str, secret: &str) -> Result<(), String> {
        TEST_GLOBAL_STORE.save(credential_id, secret)
    }

    pub fn get_credential(credential_id: &str) -> Result<Option<String>, String> {
        TEST_GLOBAL_STORE.get(credential_id)
    }

    pub fn delete_credential(credential_id: &str) -> Result<(), String> {
        TEST_GLOBAL_STORE.delete(credential_id)
    }

    pub fn has_credential(credential_id: &str) -> bool {
        TEST_GLOBAL_STORE.has(credential_id)
    }
}

// ---------------------------------------------------------------------------
// ConfigStore and Migration
// ---------------------------------------------------------------------------

static CONFIG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
pub struct ConfigStore;

impl ConfigStore {
    fn get_config_path() -> PathBuf {
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
        dir.join("config.json")
    }

    pub fn load_config() -> AppConfig {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                // First try loading modern config
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
                    let mut needs_save = false;
                    // Update credential presence metadata dynamically
                    for conn in &mut cfg.connections {
                        conn.has_credential = CredentialStore::has_credential(&conn.credential_id);
                        if (conn.protocol == ApiProtocol::GeminiNative || conn.preset == ConnectionPreset::GoogleGemini)
                            && conn.manual_model_id.is_none()
                        {
                            let m = conn.selected_model.trim();
                            if m.is_empty()
                                || m == "gemini-1.5-flash"
                                || m == "gemini-1.5-pro"
                                || m == "gemini-2.0-flash"
                                || m == "gemini-2.5-flash"
                                || m == "gemini-pro"
                                || m == "gemini-1.0-pro"
                            {
                                conn.selected_model = "gemini-3.5-flash".into();
                                needs_save = true;
                            }
                        }
                        if conn.name.contains("FreeLLMAPI") {
                            conn.name = "TZ AI Gateway".into();
                            needs_save = true;
                        }
                    }
                    if needs_save {
                        let _ = Self::save_config(&cfg);
                    }
                    return cfg;
                }

                // If parsing modern config failed, attempt migration from legacy config
                if let Ok(legacy) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(cfg) = Self::migrate_legacy_config(&legacy) {
                        let _ = Self::save_config(&cfg);
                        return cfg;
                    }
                }
            }
        }

        let mut default_cfg = AppConfig::default();
        for conn in &mut default_cfg.connections {
            conn.has_credential = CredentialStore::has_credential(&conn.credential_id);
        }
        default_cfg
    }

    pub fn migrate_legacy_config_with_store<S: CredentialStorage>(
        legacy: &serde_json::Value,
        store: &S,
    ) -> Option<AppConfig> {
        let mut cfg = AppConfig::default();

        if let Some(adb) = legacy.get("custom_adb_path").and_then(|v| v.as_str()) {
            cfg.custom_adb_path = Some(adb.to_string());
        }
        if let Some(consent) = legacy.get("ai_consent_given").and_then(|v| v.as_bool()) {
            cfg.ai_consent_given = consent;
        }

        // Migrate plaintext Gemini key if present
        if let Some(key) = legacy.get("gemini_api_key").and_then(|v| v.as_str()) {
            let trimmed = key.trim();
            if !trimmed.is_empty() && !trimmed.contains('•') {
                store.save("preset-gemini", trimmed).ok()?;
            }
        }

        // Migrate plaintext OpenAI key if present
        if let Some(key) = legacy.get("openai_api_key").and_then(|v| v.as_str()) {
            let trimmed = key.trim();
            if !trimmed.is_empty() && !trimmed.contains('•') {
                store.save("preset-openai", trimmed).ok()?;
            }
        }

        // Migrate plaintext Anthropic key if present
        if let Some(key) = legacy.get("anthropic_api_key").and_then(|v| v.as_str()) {
            let trimmed = key.trim();
            if !trimmed.is_empty() && !trimmed.contains('•') {
                store.save("preset-anthropic", trimmed).ok()?;
            }
        }

        for conn in &mut cfg.connections {
            conn.has_credential = store.has(&conn.credential_id);
        }

        Some(cfg)
    }

    pub fn migrate_legacy_config(legacy: &serde_json::Value) -> Option<AppConfig> {
        struct DefaultStore;
        impl CredentialStorage for DefaultStore {
            fn save(&self, id: &str, secret: &str) -> Result<(), String> {
                CredentialStore::save_credential(id, secret)
            }
            fn get(&self, id: &str) -> Result<Option<String>, String> {
                CredentialStore::get_credential(id)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                CredentialStore::delete_credential(id)
            }
            fn has(&self, id: &str) -> bool {
                CredentialStore::has_credential(id)
            }
        }
        Self::migrate_legacy_config_with_store(legacy, &DefaultStore)
    }

    pub fn save_config(config: &AppConfig) -> Result<(), String> {
        let _guard = CONFIG_WRITE_LOCK
            .lock()
            .map_err(|_| "Configuration lock poisoned")?;
        let _cross_lock = crate::storage::CrossProcessLock::lock_config().ok();
        let path = Self::get_config_path();

        // Ensure no plaintext keys are written to config.json
        let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;

        // Write atomically via temporary file
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json)
            .map_err(|e| format!("Failed to write temporary config: {e}"))?;
        fs::rename(&temp_path, &path)
            .map_err(|e| format!("Failed to atomically commit config: {e}"))?;

        Ok(())
    }

    /// Mask secret string for safe display in UI if ever needed
    pub fn mask_key(key: &str) -> String {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        if trimmed.len() <= 8 {
            return "••••••••".to_string();
        }
        let prefix: String = trimmed.chars().take(4).collect();
        let suffix: String = trimmed
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}••••••••{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_save_read_delete() {
        let test_id = "test-conn-123";
        let secret = "sk-test-secret-value-xyz";

        assert_eq!(CredentialStore::has_credential(test_id), false);

        CredentialStore::save_credential(test_id, secret).expect("save should succeed");
        assert_eq!(CredentialStore::has_credential(test_id), true);

        let retrieved = CredentialStore::get_credential(test_id).expect("get should succeed");
        assert_eq!(retrieved, Some(secret.to_string()));

        CredentialStore::delete_credential(test_id).expect("delete should succeed");
        assert_eq!(CredentialStore::has_credential(test_id), false);
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(ConfigStore::mask_key(""), "");
        assert_eq!(ConfigStore::mask_key("12345"), "••••••••");
        assert_eq!(
            ConfigStore::mask_key("AIzaSyAbc12345XyZ"),
            "AIza••••••••5XyZ"
        );
    }

    #[test]
    fn test_legacy_migration() {
        let legacy_json = serde_json::json!({
            "custom_adb_path": "C:\\adb.exe",
            "gemini_api_key": "AIzaSyTestLegacy123",
            "openai_api_key": "sk-legacy-test",
            "ai_consent_given": true
        });

        // Test with explicitly injected test store to guarantee isolation
        let isolated_store = InMemoryCredentialStore::new();
        let cfg = ConfigStore::migrate_legacy_config_with_store(&legacy_json, &isolated_store)
            .expect("migration should succeed");
        assert_eq!(cfg.custom_adb_path, Some("C:\\adb.exe".into()));
        assert_eq!(cfg.ai_consent_given, true);
        assert_eq!(isolated_store.has("preset-gemini"), true);
        assert_eq!(isolated_store.has("preset-openai"), true);
        assert_eq!(
            isolated_store.get("preset-gemini").unwrap(),
            Some("AIzaSyTestLegacy123".into())
        );
        assert_eq!(
            isolated_store.get("preset-openai").unwrap(),
            Some("sk-legacy-test".into())
        );
    }

    #[test]
    fn test_unit_tests_isolated_from_production_vault() {
        // Compile-time check: In test configuration, CredentialStore MUST NOT touch the production vault
        CredentialStore::save_credential("preset-gemini-test-isolation", "dummy-secret-123")
            .expect("in-memory save should succeed");
        assert_eq!(
            CredentialStore::get_credential("preset-gemini-test-isolation").unwrap(),
            Some("dummy-secret-123".into())
        );

        #[cfg(windows)]
        {
            // Verify that the actual Windows Credential Manager vault was NOT touched
            let real_vault_target = "TZ_AI_DEBLOATER_preset-gemini-test-isolation";
            let real_read = os_keyring::read(real_vault_target).expect("read check");
            assert_eq!(
                real_read, None,
                "SECURITY ERROR: Unit test wrote directly into the real Windows Credential Manager vault!"
            );
        }
    }
}
