use super::client::AiClient;
use super::types::{
    ChatMessage, ClassifiedError, DiscoveredModel, ErrorType, ModelDiscoveryResult,
};
use crate::storage::credentials::{ApiProtocol, ConnectionProfile, CredentialStore};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub const GATEWAY_CONNECTION_ID: &str = "preset-tz-gateway";
pub const DEFAULT_MAX_FAILOVERS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_name: String,
    pub is_available: bool,
    pub is_cooling_down: bool,
    pub cooldown_remaining_secs: u64,
    pub consecutive_failures: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub last_error: Option<String>,
    pub last_served_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub is_ready: bool,
    pub active_providers_count: usize,
    pub healthy_providers_count: usize,
    pub providers: Vec<ProviderHealth>,
    pub total_routed_requests: u64,
    pub total_failovers: u64,
}

struct ProviderInternalState {
    profile: ConnectionProfile,
    cooldown_until: Option<Instant>,
    consecutive_failures: u32,
    total_requests: u64,
    successful_requests: u64,
    last_error: Option<String>,
    last_served_at: Option<i64>,
}

pub struct TzAiGateway {
    providers: Arc<RwLock<HashMap<String, ProviderInternalState>>>,
    total_routed: AtomicU64,
    total_failovers: AtomicU64,
}

impl Default for TzAiGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl TzAiGateway {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            total_routed: AtomicU64::new(0),
            total_failovers: AtomicU64::new(0),
        }
    }

    /// Sync active connection profiles into the gateway provider registry
    pub fn sync_profiles(&self, profiles: &[ConnectionProfile]) {
        let mut map = self.providers.write();
        let active_ids: std::collections::HashSet<String> = profiles
            .iter()
            .filter(|p| p.id != GATEWAY_CONNECTION_ID && p.id != "preset-freellmapi")
            .map(|p| p.id.clone())
            .collect();

        // Remove deleted connections from the live routing registry
        map.retain(|id, _| active_ids.contains(id));

        for p in profiles {
            if p.id == GATEWAY_CONNECTION_ID || p.id == "preset-freellmapi" {
                continue;
            }
            if let Some(existing) = map.get_mut(&p.id) {
                existing.profile = p.clone();
            } else {
                map.insert(
                    p.id.clone(),
                    ProviderInternalState {
                        profile: p.clone(),
                        cooldown_until: None,
                        consecutive_failures: 0,
                        total_requests: 0,
                        successful_requests: 0,
                        last_error: None,
                        last_served_at: None,
                    },
                );
            }
        }
    }

    /// Get current health status of the gateway and all registered providers
    pub fn get_status(&self) -> GatewayStatus {
        let now = Instant::now();
        let map = self.providers.read();
        let mut provider_healths = Vec::new();
        let mut healthy_count = 0;

        for (_id, state) in map.iter() {
            let is_cooling = state
                .cooldown_until
                .map(|until| now < until)
                .unwrap_or(false);
            let cooldown_rem = state
                .cooldown_until
                .and_then(|until| until.checked_duration_since(now))
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let has_key = state.profile.has_credential;
            let is_available = has_key && !is_cooling;
            if is_available {
                healthy_count += 1;
            }

            provider_healths.push(ProviderHealth {
                provider_name: state.profile.name.clone(),
                is_available,
                is_cooling_down: is_cooling,
                cooldown_remaining_secs: cooldown_rem,
                consecutive_failures: state.consecutive_failures,
                total_requests: state.total_requests,
                successful_requests: state.successful_requests,
                last_error: state.last_error.clone(),
                last_served_at: state.last_served_at,
            });
        }

        GatewayStatus {
            is_ready: healthy_count > 0,
            active_providers_count: map.len(),
            healthy_providers_count: healthy_count,
            providers: provider_healths,
            total_routed_requests: self.total_routed.load(Ordering::SeqCst),
            total_failovers: self.total_failovers.load(Ordering::SeqCst),
        }
    }

    /// Mark a provider as rate-limited or failed with backoff cooldown
    pub fn mark_failure(&self, profile_id: &str, is_rate_limit: bool, err_msg: &str) {
        let mut map = self.providers.write();
        if let Some(state) = map.get_mut(profile_id) {
            state.consecutive_failures += 1;
            state.last_error = Some(err_msg.to_string());

            let cooldown_duration = if is_rate_limit {
                // Rate limit (429): cool down for 60 seconds
                Duration::from_secs(60)
            } else {
                // Transient 5xx: cool down for 15s * consecutive failures (max 2 mins)
                Duration::from_secs((15 * state.consecutive_failures.min(8)) as u64)
            };
            state.cooldown_until = Some(Instant::now() + cooldown_duration);
        }
    }

    /// Mark a provider as successful, clearing cooldowns and resetting failure counters
    pub fn mark_success(&self, profile_id: &str) {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let mut map = self.providers.write();
        if let Some(state) = map.get_mut(profile_id) {
            state.consecutive_failures = 0;
            state.cooldown_until = None;
            state.successful_requests += 1;
            state.last_served_at = Some(now_unix);
        }
    }

    /// Discover models across all configured healthy providers and aggregate them
    pub async fn discover_all_models(
        &self,
        _credentials: &CredentialStore,
    ) -> Result<ModelDiscoveryResult, ClassifiedError> {
        let providers_to_query: Vec<ConnectionProfile> = {
            let map = self.providers.read();
            map.values().map(|s| s.profile.clone()).collect()
        };

        let mut aggregated = Vec::new();
        let mut errors = Vec::new();

        for profile in providers_to_query {
            let api_key = match CredentialStore::get_credential(&profile.credential_id) {
                Ok(Some(k)) if !k.trim().is_empty() => k,
                _ => continue,
            };

            match AiClient::discover_models(&profile, Some(&api_key), None).await {
                Ok(res) => {
                    for m in res.models {
                        aggregated.push(DiscoveredModel {
                            id: format!("gateway:{}:{}", profile.id, m.id),
                            display_name: format!("{} · {}", m.display_name, profile.name),
                            description: m.description,
                            supported_generation_methods: m.supported_generation_methods,
                            is_router_alias: false,
                            tested_for_generation: m.tested_for_generation,
                        });
                    }
                }
                Err(e) => {
                    errors.push(format!("{}: {}", profile.name, e.message));
                }
            }
        }

        // Add auto/optimal gateway synthetic model if we have any models
        if !aggregated.is_empty() {
            aggregated.insert(
                0,
                DiscoveredModel {
                    id: "gateway:auto".into(),
                    display_name: "Auto (Optimal Multi-Provider Failover)".into(),
                    description: Some(
                        "Automatically routes to the fastest healthy model with quota-exhaustion fallback."
                            .into(),
                    ),
                    supported_generation_methods: vec!["generateContent".into()],
                    is_router_alias: true,
                    tested_for_generation: true,
                },
            );
        } else if !errors.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::ModelUnavailable,
                message: format!(
                    "Gateway could not discover models from any configured provider: {}",
                    errors.join("; ")
                ),
                suggested_action:
                    "Check your API credentials in Settings for the underlying providers.".into(),
                diagnostic_details: Some(errors.join("\n")),
                http_status: None,
            });
        }

        Ok(ModelDiscoveryResult {
            success: true,
            connection_id: GATEWAY_CONNECTION_ID.to_string(),
            models: aggregated,
            next_page_token: None,
            error: None,
        })
    }

    /// Stream AI generation through TZ AI Gateway with automatic bounded failover
    pub async fn stream_chat_with_failover<F>(
        &self,
        app: &AppHandle,
        request_id: &str,
        user_prompt: &str,
        device_model: &str,
        android_version: &str,
        installed_packages: &[String],
        history: &[ChatMessage],
        requested_model: &str,
        cancel_token: Arc<AtomicBool>,
        on_chunk: F,
    ) -> Result<(String, Option<String>), ClassifiedError>
    where
        F: Fn(String) + Send + Sync + Clone + 'static,
    {
        self.total_routed.fetch_add(1, Ordering::SeqCst);

        // Determine candidate execution profiles
        let candidates = self.resolve_candidates(requested_model);
        if candidates.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::QuotaRateLimit,
                message: "No healthy, configured providers available in TZ AI Gateway.".into(),
                suggested_action: "Configure API credentials for at least one provider (Gemini, OpenAI, or Anthropic) in Settings.".into(),
                diagnostic_details: None,
                http_status: None,
            });
        }

        let mut last_error = None;
        let mut failover_count = 0;

        for (profile, model_id) in candidates {
            if cancel_token.load(Ordering::SeqCst) {
                return Err(ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: "Request cancelled by user".into(),
                    suggested_action: "".into(),
                    diagnostic_details: None,
                    http_status: None,
                });
            }

            let api_key = match CredentialStore::get_credential(&profile.credential_id) {
                Ok(Some(k)) => k,
                _ => continue,
            };

            let mut run_profile = profile.clone();
            run_profile.selected_model = model_id.clone();

            // Track request attempt
            {
                let mut map = self.providers.write();
                if let Some(state) = map.get_mut(&profile.id) {
                    state.total_requests += 1;
                }
            }

            let chunk_cb = on_chunk.clone();
            match AiClient::stream_chat(
                &run_profile,
                Some(&api_key),
                &model_id,
                user_prompt,
                device_model,
                android_version,
                installed_packages,
                history,
                cancel_token.clone(),
                chunk_cb,
            )
            .await
            {
                Ok((full_text, served)) => {
                    self.mark_success(&profile.id);
                    let served_str = served.unwrap_or(model_id.clone());
                    // Emit metadata that this was served by the gateway
                    let _ = app.emit(
                        "ai-gateway-served",
                        serde_json::json!({
                            "request_id": request_id,
                            "served_provider": profile.name,
                            "served_model": served_str,
                            "failover_count": failover_count,
                        }),
                    );
                    return Ok((
                        full_text,
                        Some(format!("{} · {}", profile.name, served_str)),
                    ));
                }
                Err(err) => {
                    let is_rate_limit = matches!(err.error_type, ErrorType::QuotaRateLimit)
                        || err.http_status == Some(429);
                    let is_transient = is_rate_limit
                        || matches!(
                            err.error_type,
                            ErrorType::ModelUnavailable | ErrorType::NetworkTimeout
                        )
                        || err.http_status.map(|s| s >= 500).unwrap_or(false);

                    self.mark_failure(&profile.id, is_rate_limit, &err.message);

                    if is_transient && failover_count < DEFAULT_MAX_FAILOVERS {
                        failover_count += 1;
                        self.total_failovers.fetch_add(1, Ordering::SeqCst);
                        last_error = Some(err);
                        // Emit failover notification chunk
                        on_chunk(format!(
                            "\n*[TZ Gateway: {} returned {}, failing over to backup provider...]*\n",
                            profile.name,
                            if is_rate_limit {
                                "Rate Limit / Quota Exhausted"
                            } else {
                                "Temporary Outage"
                            }
                        ));
                        continue;
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ClassifiedError {
            error_type: ErrorType::ModelUnavailable,
            message: "All candidate providers in TZ AI Gateway failed to serve request.".into(),
            suggested_action: "Review provider credentials and rate limits.".into(),
            diagnostic_details: None,
            http_status: None,
        }))
    }

    /// Resolves candidate profiles in priority order based on health and model request
    fn resolve_candidates(&self, requested_model: &str) -> Vec<(ConnectionProfile, String)> {
        let now = Instant::now();
        let map = self.providers.read();

        // Case 1: Specific routed model e.g. "gateway:preset-gemini:gemini-1.5-pro"
        if let Some(rest) = requested_model.strip_prefix("gateway:") {
            if rest != "auto" {
                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let profile_id = parts[0];
                    let model_name = parts[1];
                    if let Some(state) = map.get(profile_id) {
                        return vec![(state.profile.clone(), model_name.to_string())];
                    }
                }
            }
        }

        // Case 2: Auto / Failover resolution across healthy configured providers
        let mut eligible = Vec::new();
        for state in map.values() {
            if !state.profile.has_credential {
                continue;
            }
            if let Ok(Some(k)) = CredentialStore::get_credential(&state.profile.credential_id) {
                if k.trim().is_empty() {
                    continue;
                }
            } else {
                continue;
            }

            let is_cooling = state
                .cooldown_until
                .map(|until| now < until)
                .unwrap_or(false);
            if is_cooling {
                continue;
            }

            let model_to_use = if !state.profile.selected_model.is_empty() {
                state.profile.selected_model.clone()
            } else {
                match state.profile.protocol {
                    ApiProtocol::GeminiNative => "gemini-3.5-flash".into(),
                    ApiProtocol::OpenAiChatCompletions => "gpt-4o-mini".into(),
                    ApiProtocol::OpenAiResponses => "gpt-4o-mini".into(),
                    ApiProtocol::AnthropicMessages => "claude-3-5-sonnet-20241022".into(),
                }
            };

            eligible.push((state.profile.clone(), model_to_use));
        }

        // Priority sort: Gemini native -> OpenAI -> Anthropic
        eligible.sort_by_key(|(p, _)| match p.protocol {
            ApiProtocol::GeminiNative => 0,
            ApiProtocol::OpenAiChatCompletions => 1,
            ApiProtocol::OpenAiResponses => 2,
            ApiProtocol::AnthropicMessages => 3,
        });

        eligible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::credentials::ConnectionMode;

    fn make_test_profile(id: &str, name: &str, protocol: ApiProtocol) -> ConnectionProfile {
        ConnectionProfile {
            id: id.to_string(),
            name: name.to_string(),
            mode: ConnectionMode::DirectApi,
            preset: crate::storage::credentials::ConnectionPreset::CustomGateway,
            base_url: "https://api.example.com/v1".to_string(),
            protocol,
            credential_id: id.to_string(),
            selected_model: "test-model".to_string(),
            manual_model_id: None,
            last_test_status: None,
            last_verified_at: None,
            supported_capabilities: vec!["chat".into()],
            tested_for_generation: false,
            has_credential: true,
        }
    }

    #[test]
    fn test_sync_profiles_removes_deleted_connections() {
        let gateway = TzAiGateway::new();

        let p1 = make_test_profile("conn-1", "Provider 1", ApiProtocol::GeminiNative);
        let p2 = make_test_profile("conn-2", "Provider 2", ApiProtocol::OpenAiChatCompletions);
        let p3 = make_test_profile("conn-3", "Provider 3", ApiProtocol::AnthropicMessages);

        // Initial sync of 3 providers
        gateway.sync_profiles(&[p1.clone(), p2.clone(), p3.clone()]);
        let status = gateway.get_status();
        assert_eq!(status.active_providers_count, 3);

        // Sync with p2 deleted
        gateway.sync_profiles(&[p1.clone(), p3.clone()]);
        let status2 = gateway.get_status();
        assert_eq!(status2.active_providers_count, 2);
        assert!(status2.providers.iter().any(|p| p.provider_name == "Provider 1"));
        assert!(status2.providers.iter().any(|p| p.provider_name == "Provider 3"));
        assert!(!status2.providers.iter().any(|p| p.provider_name == "Provider 2"));

        // Sync with all deleted
        gateway.sync_profiles(&[]);
        let status3 = gateway.get_status();
        assert_eq!(status3.active_providers_count, 0);
    }
}
