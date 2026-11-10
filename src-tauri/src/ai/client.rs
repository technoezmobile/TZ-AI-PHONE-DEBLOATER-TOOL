use super::types::*;
use crate::adb::operation::ActionType;
use crate::dataset::get_catalog;
use crate::policy::SafetyPolicy;
use crate::storage::credentials::{ApiProtocol, ConnectionPreset, ConnectionProfile};
use reqwest::header::AUTHORIZATION;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub struct AiClient;

impl AiClient {
    /// Normalize URL without duplicating `/v1` and enforce security rules
    pub fn normalize_endpoint(
        base_url: &str,
        path_suffix: &str,
    ) -> Result<String, ClassifiedError> {
        let trimmed_base = base_url.trim().trim_end_matches('/');
        if trimmed_base.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidEndpointOrProtocol,
                message: "Base URL cannot be empty".into(),
                suggested_action: "Enter a valid HTTP or HTTPS base URL in Connection Settings"
                    .into(),
                diagnostic_details: None,
                http_status: None,
            });
        }

        // Validate URL parse
        let parsed = reqwest::Url::parse(trimmed_base).map_err(|e| ClassifiedError {
            error_type: ErrorType::InvalidEndpointOrProtocol,
            message: format!("Malformed base URL: {e}"),
            suggested_action: "Provide a valid URL with scheme (e.g. https://api.example.com)"
                .into(),
            diagnostic_details: Some(e.to_string()),
            http_status: None,
        })?;

        // Reject embedded credentials in URLs (e.g. http://user:pass@host)
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidEndpointOrProtocol,
                message: "Embedded credentials in URL are rejected for security".into(),
                suggested_action:
                    "Remove username/password from the URL and use the dedicated credential field"
                        .into(),
                diagnostic_details: None,
                http_status: None,
            });
        }

        // Enforce HTTPS unless local loopback
        let host = parsed.host_str().unwrap_or("");
        let is_loopback = host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host == "[::1]"
            || host.ends_with(".localhost");
        if parsed.scheme() != "https" && !is_loopback {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidEndpointOrProtocol,
                message: format!("Insecure HTTP scheme is prohibited for remote endpoint '{host}'"),
                suggested_action:
                    "Use HTTPS for remote connections, or use localhost for local gateways".into(),
                diagnostic_details: Some(format!("Host {host} is not a loopback address")),
                http_status: None,
            });
        }

        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidEndpointOrProtocol,
                message: "Use an HTTP(S) base URL without query strings or fragments".into(),
                suggested_action: "Edit the connection base URL".into(),
                diagnostic_details: None,
                http_status: None,
            });
        }
        let suffix = path_suffix.trim_start_matches('/');
        if trimmed_base.ends_with("/v1beta") && suffix.starts_with("v1beta/") {
            return Ok(format!("{trimmed_base}/{}", &suffix[7..]));
        }

        // Avoid duplicating /v1:
        // If base ends with /v1 and suffix starts with v1/, strip the duplicate
        if trimmed_base.ends_with("/v1") && suffix.starts_with("v1/") {
            let deduplicated = &suffix[3..];
            Ok(format!("{trimmed_base}/{deduplicated}"))
        } else if !trimmed_base.ends_with("/v1") && suffix.starts_with("v1/") {
            Ok(format!("{trimmed_base}/{suffix}"))
        } else {
            Ok(format!("{trimmed_base}/{suffix}"))
        }
    }

    /// Build secure HTTP client with origin-preserving redirect policy
    pub fn build_http_client(timeout_secs: u64) -> Result<reqwest::Client, ClassifiedError> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            // Security: do NOT forward secrets across origin-changing redirects
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("Too many redirects")
                } else if attempt.url().origin() != attempt.previous()[0].origin() {
                    // Origin change detected - stop to avoid credential leak
                    attempt.error("Cross-origin redirect prohibited for credential security")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|e| ClassifiedError {
                error_type: ErrorType::Unknown,
                message: format!("Failed to initialize HTTP client: {e}"),
                suggested_action: "Check system network configuration".into(),
                diagnostic_details: Some(e.to_string()),
                http_status: None,
            })
    }

    /// Classify raw HTTP response errors accurately
    pub fn classify_response_error(status: reqwest::StatusCode, body: &str) -> ClassifiedError {
        let code = status.as_u16();
        let body_lower = body.to_lowercase();

        if code == 403 {
            ClassifiedError {
                error_type: ErrorType::PermissionAccess,
                message: "The account or project does not have access to this resource".into(),
                suggested_action: "Check API enablement, key restrictions, and model permissions"
                    .into(),
                diagnostic_details: None,
                http_status: Some(code),
            }
        } else if code == 401 {
            ClassifiedError {
                error_type: ErrorType::Authentication,
                message: "Authentication failed. The API key or token was rejected.".into(),
                suggested_action:
                    "Verify or update the credential for this connection profile in Settings."
                        .into(),
                diagnostic_details: None,
                http_status: Some(code),
            }
        } else if code == 404
            || body_lower.contains("not found")
            || body_lower.contains("model not found")
        {
            ClassifiedError {
                error_type: ErrorType::ModelUnavailable,
                message: "The requested model is unavailable or does not exist for this account.".into(),
                suggested_action: "Use the Refresh models button in the toolbar to discover available models, or specify a valid model identifier.".into(),
                diagnostic_details: Some(body.to_string()),
                http_status: Some(code),
            }
        } else if code == 429 || body_lower.contains("quota") || body_lower.contains("rate limit") {
            ClassifiedError {
                error_type: ErrorType::QuotaRateLimit,
                message: "API rate limit or quota exceeded.".into(),
                suggested_action:
                    "Wait a moment before retrying or check your provider plan quotas.".into(),
                diagnostic_details: Some(body.to_string()),
                http_status: Some(code),
            }
        } else if code == 400 && body_lower.contains("invalid") {
            ClassifiedError {
                error_type: ErrorType::UnsupportedRequestParameter,
                message:
                    "The request contained an unsupported parameter for this model or protocol."
                        .into(),
                suggested_action:
                    "Verify that the selected model supports the selected API protocol.".into(),
                diagnostic_details: Some(body.to_string()),
                http_status: Some(code),
            }
        } else {
            ClassifiedError {
                error_type: ErrorType::Unknown,
                message: format!("HTTP error {code} from server"),
                suggested_action: "Inspect the diagnostic details below or try again shortly."
                    .into(),
                diagnostic_details: Some(body.to_string()),
                http_status: Some(code),
            }
        }
    }

    /// Discover models from provider or gateway using real credentials
    pub async fn discover_models(
        profile: &ConnectionProfile,
        credential: Option<&str>,
        page_token: Option<&str>,
    ) -> Result<ModelDiscoveryResult, ClassifiedError> {
        let client = Self::build_http_client(15)?;
        let key = credential.unwrap_or("").trim();

        match profile.protocol {
            ApiProtocol::GeminiNative => {
                let suffix = match page_token {
                    Some(tok) => {
                        let encoded: String = reqwest::Url::parse_with_params(
                            "http://localhost/",
                            &[("pageToken", tok)],
                        )
                        .unwrap()
                        .query()
                        .unwrap()
                        .to_string();
                        format!("v1beta/models?pageSize=50&{encoded}")
                    }
                    None => "v1beta/models?pageSize=50".to_string(),
                };
                let url = Self::normalize_endpoint(&profile.base_url, &suffix)?;

                let mut req = client.get(&url);
                if !key.is_empty() {
                    req = req.header("x-goog-api-key", key);
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Connection failed to Gemini endpoint: {e}"),
                    suggested_action:
                        "Check your internet connection and verify Google AI Studio availability."
                            .into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                if !status.is_success() {
                    return Err(Self::classify_response_error(status, &text));
                }

                let parsed: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Received invalid JSON from model discovery endpoint".into(),
                        suggested_action: "Verify the base URL and protocol configuration".into(),
                        diagnostic_details: Some(e.to_string()),
                        http_status: Some(status.as_u16()),
                    })?;

                let mut models = Vec::new();
                if let Some(arr) = parsed.get("models").and_then(|m| m.as_array()) {
                    for item in arr {
                        let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let display_name = item
                            .get("displayName")
                            .and_then(|d| d.as_str())
                            .unwrap_or(name);
                        let desc = item
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(|s| s.to_string());

                        // Metadata inspection: must support "generateContent"!
                        let mut methods = Vec::new();
                        let mut supports_generation = false;
                        if let Some(m_arr) = item
                            .get("supportedGenerationMethods")
                            .and_then(|m| m.as_array())
                        {
                            for m in m_arr {
                                if let Some(method) = m.as_str() {
                                    methods.push(method.to_string());
                                    if method == "generateContent" {
                                        supports_generation = true;
                                    }
                                }
                            }
                        }

                        // Filter models according to required capabilities
                        if supports_generation {
                            let clean_id = name.strip_prefix("models/").unwrap_or(name);
                            models.push(DiscoveredModel {
                                id: clean_id.to_string(),
                                display_name: display_name.to_string(),
                                description: desc,
                                supported_generation_methods: methods,
                                is_router_alias: false,
                                tested_for_generation: false,
                            });
                        }
                    }
                }

                let next_token = parsed
                    .get("nextPageToken")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                Ok(ModelDiscoveryResult {
                    success: true,
                    connection_id: profile.id.clone(),
                    models,
                    next_page_token: next_token,
                    error: None,
                })
            }

            ApiProtocol::OpenAiChatCompletions | ApiProtocol::OpenAiResponses => {
                let url = Self::normalize_endpoint(&profile.base_url, "models")?;
                let mut req = client.get(&url);

                if !key.is_empty() {
                    req = req.header(AUTHORIZATION, format!("Bearer {key}"));
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Connection failed to models endpoint: {e}"),
                    suggested_action: "Verify the base URL and that the server is running.".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                if !status.is_success() {
                    return Err(Self::classify_response_error(status, &text));
                }

                let parsed: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Received invalid JSON from models endpoint".into(),
                        suggested_action: "Verify server responses".into(),
                        diagnostic_details: Some(e.to_string()),
                        http_status: Some(status.as_u16()),
                    })?;

                let mut models = Vec::new();
                if let Some(arr) = parsed.get("data").and_then(|d| d.as_array()) {
                    for item in arr {
                        if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                            let is_router = profile.preset == ConnectionPreset::FreeLlmApi;
                            models.push(DiscoveredModel {
                                id: id.to_string(),
                                display_name: id.to_string(),
                                description: None,
                                supported_generation_methods: vec!["chat.completions".into()],
                                is_router_alias: is_router,
                                tested_for_generation: false,
                            });
                        }
                    }
                }

                Ok(ModelDiscoveryResult {
                    success: true,
                    connection_id: profile.id.clone(),
                    models,
                    next_page_token: None,
                    error: None,
                })
            }

            ApiProtocol::AnthropicMessages => {
                let mut url =
                    reqwest::Url::parse(&Self::normalize_endpoint(&profile.base_url, "v1/models")?)
                        .map_err(|_| ClassifiedError {
                            error_type: ErrorType::InvalidEndpointOrProtocol,
                            message: "Invalid endpoint".into(),
                            suggested_action: "Check URL".into(),
                            diagnostic_details: None,
                            http_status: None,
                        })?;
                if let Some(token) = page_token {
                    url.query_pairs_mut().append_pair("after_id", token);
                }
                let mut req = client.get(url).header("anthropic-version", "2023-06-01");

                if !key.is_empty() {
                    req = req.header("x-api-key", key);
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Connection failed to Anthropic models endpoint: {e}"),
                    suggested_action: "Verify Anthropic endpoint connectivity.".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();

                if !status.is_success() {
                    return Err(Self::classify_response_error(status, &text));
                }

                let parsed: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Invalid JSON from Anthropic models endpoint".into(),
                        suggested_action: "Verify Anthropic response format".into(),
                        diagnostic_details: Some(e.to_string()),
                        http_status: Some(status.as_u16()),
                    })?;

                let mut models = Vec::new();
                if let Some(arr) = parsed.get("data").and_then(|d| d.as_array()) {
                    for item in arr {
                        let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let display = item
                            .get("display_name")
                            .and_then(|d| d.as_str())
                            .unwrap_or(id);
                        if !id.is_empty() {
                            models.push(DiscoveredModel {
                                id: id.to_string(),
                                display_name: display.to_string(),
                                description: None,
                                supported_generation_methods: vec!["messages".into()],
                                is_router_alias: false,
                                tested_for_generation: false,
                            });
                        }
                    }
                }

                Ok(ModelDiscoveryResult {
                    success: true,
                    connection_id: profile.id.clone(),
                    models,
                    next_page_token: if parsed["has_more"].as_bool().unwrap_or(false) {
                        parsed["last_id"].as_str().map(str::to_string)
                    } else {
                        None
                    },
                    error: None,
                })
            }
        }
    }

    /// Check gateway readiness (especially for FreeLLMAPI / self-hosted gateways)
    pub async fn check_gateway_readiness(
        profile: &ConnectionProfile,
        credential: Option<&str>,
    ) -> Result<GatewayReadinessInfo, ClassifiedError> {
        let client = Self::build_http_client(10)?;
        let key = credential.unwrap_or("").trim();

        // Check base models or health endpoint
        let url = Self::normalize_endpoint(&profile.base_url, "models")?;
        let mut req = client.get(&url);
        if !key.is_empty() {
            req = req.header(AUTHORIZATION, format!("Bearer {key}"));
        }

        let resp = req.send().await.map_err(|e| ClassifiedError {
            error_type: ErrorType::NetworkTimeout,
            message: format!("Gateway is unreachable: {e}"),
            suggested_action:
                "Ensure the local AI gateway or custom endpoint is active and listening at the configured Base URL."
                    .into(),
            diagnostic_details: Some(e.to_string()),
            http_status: None,
        })?;

        let status = resp.status();
        let served_model = resp
            .headers()
            .get("x-served-model")
            .or_else(|| resp.headers().get("openai-model"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let gateway_available = status.is_success();
        let data: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
        let upstream_available = data["data"]
            .as_array()
            .map(|models| models.iter().any(|m| m["execution_status"] == "ready"))
            .unwrap_or(false);
        let is_ready = gateway_available && upstream_available;
        Ok(GatewayReadinessInfo {
            is_ready,
            gateway_available,
            upstream_available,
            message: if is_ready {
                "Gateway and at least one upstream model report ready".into()
            } else {
                if gateway_available {
                    "Gateway is reachable; upstream readiness is not verified. Run a generation test.".into()
                } else {
                    format!("Gateway returned status {status}")
                }
            },
            served_model,
        })
    }

    /// Validate that the HTTP response from generation contains actual generated text
    /// and rejects empty bodies, malformed JSON, incompatible shapes, and error objects.
    pub fn validate_generation_response(
        protocol: ApiProtocol,
        status_code: u16,
        body_text: &str,
    ) -> Result<String, ClassifiedError> {
        let trimmed = body_text.trim();
        if trimmed.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidTruncatedAiResponse,
                message: "Endpoint returned an empty response body".into(),
                suggested_action: "Check the API gateway or provider endpoint status".into(),
                diagnostic_details: None,
                http_status: Some(status_code),
            });
        }

        let response: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| ClassifiedError {
                error_type: ErrorType::InvalidTruncatedAiResponse,
                message: format!("Generation returned invalid JSON: {e}"),
                suggested_action: "Verify that the endpoint implements the selected API protocol"
                    .into(),
                diagnostic_details: Some(e.to_string()),
                http_status: Some(status_code),
            })?;

        if !response.is_object() {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidTruncatedAiResponse,
                message: "Expected a JSON object in generation response".into(),
                suggested_action: "Check the API endpoint and protocol compatibility".into(),
                diagnostic_details: Some(format!(
                    "Got JSON type: {}",
                    match response {
                        serde_json::Value::Array(_) => "Array",
                        serde_json::Value::String(_) => "String",
                        serde_json::Value::Number(_) => "Number",
                        serde_json::Value::Bool(_) => "Boolean",
                        _ => "Other",
                    }
                )),
                http_status: Some(status_code),
            });
        }

        // Check for provider error objects (even if HTTP status was 200)
        if let Some(err_val) = response.get("error") {
            if !err_val.is_null() {
                let msg = if let Some(obj) = err_val.as_object() {
                    obj.get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Provider returned an error object")
                } else if let Some(s) = err_val.as_str() {
                    s
                } else {
                    "Provider returned an error object"
                };
                return Err(ClassifiedError {
                    error_type: ErrorType::Unknown,
                    message: format!("Provider error: {msg}"),
                    suggested_action: "Review provider credentials, permissions, and quotas".into(),
                    diagnostic_details: Some(err_val.to_string()),
                    http_status: Some(status_code),
                });
            }
        }
        if response.get("type").and_then(|t| t.as_str()) == Some("error") {
            let msg = response
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Anthropic returned an error response");
            return Err(ClassifiedError {
                error_type: ErrorType::Unknown,
                message: format!("Anthropic error: {msg}"),
                suggested_action: "Review Anthropic API key and permissions".into(),
                diagnostic_details: Some(response.to_string()),
                http_status: Some(status_code),
            });
        }

        let mut extracted = String::new();
        match protocol {
            ApiProtocol::GeminiNative => {
                if let Some(candidates) = response.get("candidates").and_then(|c| c.as_array()) {
                    if candidates.is_empty() {
                        return Err(ClassifiedError {
                            error_type: ErrorType::InvalidTruncatedAiResponse,
                            message: "Gemini candidates list is empty".into(),
                            suggested_action: "Check model availability and safety filters".into(),
                            diagnostic_details: None,
                            http_status: Some(status_code),
                        });
                    }
                    for cand in candidates {
                        if let Some(finish) = cand.get("finishReason").and_then(|f| f.as_str()) {
                            if matches!(finish, "SAFETY" | "RECITATION" | "BLOCKLIST" | "OTHER") {
                                return Err(ClassifiedError {
                                    error_type: ErrorType::InvalidTruncatedAiResponse,
                                    message: format!(
                                        "Gemini generation blocked by filter: {finish}"
                                    ),
                                    suggested_action: "Adjust safety settings or prompt".into(),
                                    diagnostic_details: None,
                                    http_status: Some(status_code),
                                });
                            }
                        }
                        if let Some(parts) =
                            cand.pointer("/content/parts").and_then(|p| p.as_array())
                        {
                            for part in parts {
                                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                                    extracted.push_str(t);
                                }
                            }
                        }
                    }
                } else {
                    return Err(ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Missing 'candidates' array in Gemini response".into(),
                        suggested_action: "Verify Gemini API protocol endpoint".into(),
                        diagnostic_details: None,
                        http_status: Some(status_code),
                    });
                }
            }
            ApiProtocol::OpenAiChatCompletions => {
                if let Some(choices) = response.get("choices").and_then(|c| c.as_array()) {
                    if choices.is_empty() {
                        return Err(ClassifiedError {
                            error_type: ErrorType::InvalidTruncatedAiResponse,
                            message: "Chat completions choices array is empty".into(),
                            suggested_action: "Check gateway routing and model availability".into(),
                            diagnostic_details: None,
                            http_status: Some(status_code),
                        });
                    }
                    for choice in choices {
                        if let Some(content) =
                            choice.pointer("/message/content").and_then(|c| c.as_str())
                        {
                            extracted.push_str(content);
                        }
                    }
                } else {
                    return Err(ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Missing 'choices' array in Chat Completions response".into(),
                        suggested_action:
                            "Verify that the endpoint implements OpenAI Chat Completions".into(),
                        diagnostic_details: None,
                        http_status: Some(status_code),
                    });
                }
            }
            ApiProtocol::OpenAiResponses => {
                if let Some(output_text) = response.get("output_text").and_then(|t| t.as_str()) {
                    extracted.push_str(output_text);
                } else if let Some(output) = response.get("output").and_then(|o| o.as_array()) {
                    if output.is_empty() {
                        return Err(ClassifiedError {
                            error_type: ErrorType::InvalidTruncatedAiResponse,
                            message: "Responses output array is empty".into(),
                            suggested_action: "Check Responses model availability".into(),
                            diagnostic_details: None,
                            http_status: Some(status_code),
                        });
                    }
                    for item in output {
                        if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                            for part in content {
                                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                                    extracted.push_str(t);
                                }
                            }
                        }
                    }
                } else {
                    return Err(ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Missing 'output' or 'output_text' in OpenAI Responses response"
                            .into(),
                        suggested_action:
                            "Verify that the endpoint supports the OpenAI Responses protocol".into(),
                        diagnostic_details: None,
                        http_status: Some(status_code),
                    });
                }
            }
            ApiProtocol::AnthropicMessages => {
                if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
                    if content.is_empty() {
                        return Err(ClassifiedError {
                            error_type: ErrorType::InvalidTruncatedAiResponse,
                            message: "Anthropic messages content array is empty".into(),
                            suggested_action: "Check Anthropic model availability".into(),
                            diagnostic_details: None,
                            http_status: Some(status_code),
                        });
                    }
                    for item in content {
                        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if item_type == "text" {
                            if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                                extracted.push_str(t);
                            }
                        }
                    }
                } else {
                    return Err(ClassifiedError {
                        error_type: ErrorType::InvalidTruncatedAiResponse,
                        message: "Missing 'content' array in Anthropic response".into(),
                        suggested_action: "Verify that the endpoint implements Anthropic Messages"
                            .into(),
                        diagnostic_details: None,
                        http_status: Some(status_code),
                    });
                }
            }
        }

        let final_text = extracted.trim();
        if final_text.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::InvalidTruncatedAiResponse,
                message: "The model completed the request but returned no generated text".into(),
                suggested_action: "Verify model capabilities and prompt configuration".into(),
                diagnostic_details: None,
                http_status: Some(status_code),
            });
        }

        Ok(final_text.to_string())
    }

    /// Small, explicitly initiated generation test to verify actual inference capability
    pub async fn test_generation(
        profile: &ConnectionProfile,
        credential: Option<&str>,
        model_id: &str,
    ) -> Result<GenerationTestResult, ClassifiedError> {
        let start = Instant::now();
        let client = Self::build_http_client(20)?;
        let key = credential.unwrap_or("").trim();
        let clean_model = model_id.trim().trim_start_matches("models/");

        if clean_model.is_empty() {
            return Err(ClassifiedError {
                error_type: ErrorType::ModelUnavailable,
                message: "No model selected to test".into(),
                suggested_action: "Select or enter a model identifier first".into(),
                diagnostic_details: None,
                http_status: None,
            });
        }

        let (status, text) = match profile.protocol {
            ApiProtocol::GeminiNative => {
                let suffix = format!("v1beta/models/{clean_model}:generateContent");
                let url = Self::normalize_endpoint(&profile.base_url, &suffix)?;

                let body = json!({
                    "contents": [{
                        "parts": [{ "text": "Respond with the word 'READY' to verify test." }]
                    }],
                    "generationConfig": {
                        "maxOutputTokens": 256
                    }
                });

                let mut req = client.post(&url).json(&body);
                if !key.is_empty() {
                    req = req.header("x-goog-api-key", key);
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Test generation request failed: {e}"),
                    suggested_action: "Check connection and model identifier".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let s = resp.status();
                let t = resp.text().await.unwrap_or_default();
                (s, t)
            }

            ApiProtocol::OpenAiChatCompletions => {
                let url = Self::normalize_endpoint(&profile.base_url, "chat/completions")?;
                let body = json!({
                    "model": clean_model,
                    "messages": [{ "role": "user", "content": "Respond with 'READY'." }],
                    "max_tokens": 64
                });

                let mut req = client.post(&url).json(&body);
                if !key.is_empty() {
                    req = req.header(AUTHORIZATION, format!("Bearer {key}"));
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Test generation request failed: {e}"),
                    suggested_action: "Check gateway or API endpoint".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let s = resp.status();
                let t = resp.text().await.unwrap_or_default();
                (s, t)
            }

            ApiProtocol::OpenAiResponses => {
                let url = Self::normalize_endpoint(&profile.base_url, "responses")?;
                let body = json!({
                    "model": clean_model,
                    "input": "Respond with 'READY'."
                });

                let mut req = client.post(&url).json(&body);
                if !key.is_empty() {
                    req = req.header(AUTHORIZATION, format!("Bearer {key}"));
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Test generation request failed: {e}"),
                    suggested_action: "Check gateway or API endpoint".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let s = resp.status();
                let t = resp.text().await.unwrap_or_default();
                (s, t)
            }

            ApiProtocol::AnthropicMessages => {
                let url = Self::normalize_endpoint(&profile.base_url, "v1/messages")?;
                let body = json!({
                    "model": clean_model,
                    "messages": [{ "role": "user", "content": "Respond with 'READY'." }],
                    "max_tokens": 10
                });

                let mut req = client
                    .post(&url)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body);
                if !key.is_empty() {
                    req = req.header("x-api-key", key);
                }

                let resp = req.send().await.map_err(|e| ClassifiedError {
                    error_type: ErrorType::NetworkTimeout,
                    message: format!("Test generation request failed: {e}"),
                    suggested_action: "Check Anthropic connection".into(),
                    diagnostic_details: Some(e.to_string()),
                    http_status: None,
                })?;

                let s = resp.status();
                let t = resp.text().await.unwrap_or_default();
                (s, t)
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        if !status.is_success() {
            return Err(Self::classify_response_error(status, &text));
        }

        // Strictly validate generated text response according to protocol schema
        let validated_text =
            Self::validate_generation_response(profile.protocol, status.as_u16(), &text)?;

        Ok(GenerationTestResult {
            success: true,
            connection_id: profile.id.clone(),
            model_id: clean_model.to_string(),
            latency_ms,
            message: format!(
                "Generation test succeeded in {latency_ms}ms (received {} chars)",
                validated_text.len()
            ),
            error: None,
        })
    }

    /// Check if a package is classified as tracking, analytics, telemetry or ad service
    pub fn is_tracker_or_analytics(pkg: &str) -> bool {
        let catalog = get_catalog();
        let id_lower = pkg.to_lowercase();
        if let Some(m) = catalog.get(pkg) {
            let desc_lower = m.description.to_lowercase();
            m.labels.iter().any(|l| {
                let l = l.to_lowercase();
                l == "tracking" || l == "analytics" || l == "telemetry" || l == "ads"
            }) || desc_lower.contains("analytic")
               || desc_lower.contains("tracking")
               || desc_lower.contains("telemetry")
               || desc_lower.contains("usage stats")
               || desc_lower.contains("ad service")
               || desc_lower.contains("data collection")
        } else {
            id_lower.contains("analytic")
                || id_lower.contains("telemetry")
                || id_lower.contains("tracker")
                || id_lower.contains("msa.global")
        }
    }

    /// Check if a package is classified as preinstalled promotional, partner or commercial bloatware
    pub fn is_promotional(pkg: &str) -> bool {
        let catalog = get_catalog();
        let id_lower = pkg.to_lowercase();
        if let Some(m) = catalog.get(pkg) {
            let desc_lower = m.description.to_lowercase();
            m.labels.iter().any(|l| {
                let l = l.to_lowercase();
                l == "promotional" || l == "preinstalled" || l == "commercial" || l == "bloatware"
            }) || desc_lower.contains("promotional")
               || desc_lower.contains("partner")
               || desc_lower.contains("sponsor")
               || desc_lower.contains("advert")
               || m.list.to_lowercase() == "misc"
        } else {
            id_lower.contains("facebook")
                || id_lower.contains("netflix")
                || id_lower.contains("amazon")
                || id_lower.contains("tiktok")
                || id_lower.contains("discover")
                || id_lower.contains("gamecenter")
        }
    }

    /// Redact device serial and user identifier to protect privacy
    pub fn sanitize_prompt_context(
        device_model: &str,
        android_version: &str,
        installed_packages: &[String],
    ) -> (String, Vec<serde_json::Value>) {
        let catalog = get_catalog();

        let mut pkg_summaries = Vec::with_capacity(installed_packages.len());
        for pkg in installed_packages {
            if let Some(meta) = catalog.get(pkg) {
                pkg_summaries.push(json!({
                    "id": pkg,
                    "removal": meta.removal,
                    "list": meta.list,
                    "desc": meta.description
                }));
            } else {
                pkg_summaries.push(json!({
                    "id": pkg,
                    "removal": "Unlisted",
                    "list": "Unlisted",
                    "desc": "Unknown package"
                }));
            }
        }

        let system_instruction = format!(
            "You are the debloating intelligence engine inside 'TZ AI DEBLOATER TOOL'.\n\
            Target Device: {} (Android {})\n\
            Analyze the user's intent. If they ask a general question or want an explanation, provide a clear, helpful plain text explanation.\n\
            If they ask to debloat, remove, disable, or modify packages, provide an explanation followed by a STRICT JSON action plan block formatted as:\n\
            ```json\n\
            {{\n\
              \"explanation\": \"Will prepare recommendations to remove non-essential bloatware while preserving core system capabilities.\",\n\
              \"preserved_functionality\": [\"calls\", \"payments\", \"camera\", ...],\n\
              \"actions\": [\n\
                {{\n\
                  \"package_id\": \"com.example.app\",\n\
                  \"package_name\": \"App Name\",\n\
                  \"proposed_action\": \"UninstallUser\",\n\
                  \"reason\": \"Specific reason for removal\",\n\
                  \"evidence\": \"UAD catalog / Manufacturer bloatware\",\n\
                  \"expected_side_effects\": \"No notifications from this app\",\n\
                  \"uncertainty\": \"Low\"\n\
                }}\n\
              ]\n\
            }}\n\
            ```\n\
            RULES:\n\
            1. NEVER propose removing critical core OS packages (SystemUI, Settings, PackageInstaller, PermissionController, Phone/Telecom, Emergency, Keyboard/IME, Active Launcher).\n\
            2. Respect explicit user preservation preferences (calls, camera, banking, notifications, updates).\n\
            3. ACTION SELECTION RULES:\n\
               - When the user asks to remove, debloat, delete, or uninstall packages, propose 'UninstallUser' (per-user uninstall that keeps the system partition APK intact while completely removing the app and user state for this profile).\n\
               - Propose 'Disable' ONLY when the user explicitly asks to disable, or when a package cannot be uninstalled but supports component disabling.\n\
               - Propose 'Enable' to re-enable a disabled package, and 'Restore' to reinstall an uninstalled system package.\n\
               - NEVER silently substitute 'Disable' when the user asked for removal.\n\
            4. TENSE AND WORDING:\n\
               - Use PROPOSED / FUTURE tense in all explanations and reason strings before execution (e.g. 'Will uninstall...', 'Will remove...', 'Proposed for removal'). NEVER write 'Removed...' or 'Disabled...' in past tense before the plan is executed.\n\
            5. Do NOT output a plan if the user only asked a question.",
            device_model, android_version
        );

        (system_instruction, pkg_summaries)
    }

    /// Extract structured ActionPlan from AI markdown/JSON response
    pub fn extract_plan_from_text(
        text: &str,
        target_serial: &str,
        target_user: u32,
    ) -> Option<ActionPlan> {
        let json_str = if let Some(start) = text.find("```json") {
            let after_start = &text[start + 7..];
            if let Some(end) = after_start.find("```") {
                &after_start[..end]
            } else {
                after_start
            }
        } else if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                if end > start {
                    &text[start..=end]
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else {
            return None;
        };

        let parsed: serde_json::Value = serde_json::from_str(json_str.trim()).ok()?;
        let explanation = parsed
            .get("explanation")
            .and_then(|v| v.as_str())
            .unwrap_or("AI debloating recommendations")
            .to_string();

        let mut preserved = Vec::new();
        if let Some(arr) = parsed
            .get("preserved_functionality")
            .and_then(|v| v.as_array())
        {
            for item in arr {
                if let Some(s) = item.as_str() {
                    preserved.push(s.to_string());
                }
            }
        }

        let mut actions = Vec::new();
        if let Some(arr) = parsed.get("actions").and_then(|v| v.as_array()) {
            for item in arr {
                let pkg_id = item
                    .get("package_id")
                    .and_then(|v| v.as_str())?
                    .trim()
                    .to_string();
                if pkg_id.is_empty() {
                    continue;
                }

                let pkg_name = item
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let action_str = item.get("proposed_action").and_then(|v| v.as_str())?;
                let proposed_action = match action_str {
                    "UninstallUser" => ActionType::UninstallUser,
                    "Disable" => ActionType::Disable,
                    // Reject unknown action types rather than silently converting to Disable
                    _ => return None,
                };

                let reason = item
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Recommended bloatware removal")
                    .to_string();
                let evidence = item
                    .get("evidence")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UAD package catalog")
                    .to_string();
                let side_effects = item
                    .get("expected_side_effects")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let uncertainty = item
                    .get("uncertainty")
                    .and_then(|v| v.as_str())
                    .unwrap_or("High")
                    .to_string();

                let (is_protected, warning) = SafetyPolicy::is_package_protected(&pkg_id);

                actions.push(ProposedAction {
                    package_id: pkg_id,
                    package_name: pkg_name,
                    proposed_action,
                    reason,
                    evidence,
                    expected_side_effects: side_effects,
                    uncertainty,
                    removal_category: "Recommended".to_string(),
                    is_protected,
                    protection_warning: warning,
                    selected: !is_protected,
                    is_executable: !is_protected,
                });
            }
        }

        if actions.is_empty() {
            return None;
        }

        Some(ActionPlan {
            id: Uuid::new_v4().to_string(),
            revision: 1,
            target_device_serial: target_serial.to_string(),
            target_user_id: target_user,
            explanation,
            preserved_functionality: preserved,
            actions,
            created_at: chrono::Utc::now().timestamp(),
            approved_by_user: false,
            is_valid: true,
            invalidation_reason: None,
        })
    }

    pub async fn stream_chat<F>(
        profile: &ConnectionProfile,
        credential: Option<&str>,
        model_id: &str,
        user_prompt: &str,
        device_model: &str,
        android_version: &str,
        installed_packages: &[String],
        history: &[ChatMessage],
        cancel_token: Arc<AtomicBool>,
        on_chunk: F,
    ) -> Result<(String, Option<String>), ClassifiedError>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        super::stream::stream_chat(
            profile,
            credential,
            model_id,
            user_prompt,
            device_model,
            android_version,
            installed_packages,
            history,
            cancel_token,
            on_chunk,
        )
        .await
    }
}
