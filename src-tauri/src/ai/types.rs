use crate::adb::operation::ActionType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortcutIntent {
    ReviewBloatware,
    ReviewTrackers,
    UninstallPromos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub timestamp: i64,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredModel {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub supported_generation_methods: Vec<String>,
    pub is_router_alias: bool,
    pub tested_for_generation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDiscoveryResult {
    pub success: bool,
    pub connection_id: String,
    pub models: Vec<DiscoveredModel>,
    pub next_page_token: Option<String>,
    pub error: Option<ClassifiedError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTestResult {
    pub success: bool,
    pub connection_id: String,
    pub model_id: String,
    pub latency_ms: u64,
    pub message: String,
    pub error: Option<ClassifiedError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayReadinessInfo {
    pub is_ready: bool,
    pub gateway_available: bool,
    pub upstream_available: bool,
    pub message: String,
    pub served_model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    Authentication,
    ModelUnavailable,
    PermissionAccess,
    QuotaRateLimit,
    InvalidEndpointOrProtocol,
    NetworkTimeout,
    UnsupportedRequestParameter,
    InvalidTruncatedAiResponse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    pub error_type: ErrorType,
    pub message: String,
    pub suggested_action: String,
    pub diagnostic_details: Option<String>,
    pub http_status: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkPayload {
    pub request_id: String,
    pub text: String,
    pub is_thought: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDonePayload {
    pub request_id: String,
    pub finish_reason: String,
    pub served_model: Option<String>,
    pub plan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamErrorPayload {
    pub request_id: String,
    pub error: ClassifiedError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub package_id: String,
    pub package_name: Option<String>,
    pub proposed_action: ActionType,
    pub reason: String,
    pub evidence: String,
    pub expected_side_effects: Option<String>,
    pub uncertainty: String, // "Low", "Medium", "High"
    pub removal_category: String,
    pub is_protected: bool,
    pub protection_warning: Option<String>,
    pub selected: bool,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub id: String,
    pub revision: u64,
    pub target_device_serial: String,
    pub target_user_id: u32,
    pub explanation: String,
    pub preserved_functionality: Vec<String>,
    pub actions: Vec<ProposedAction>,
    pub created_at: i64,
    pub approved_by_user: bool,
    pub is_valid: bool,
    pub invalidation_reason: Option<String>,
}
