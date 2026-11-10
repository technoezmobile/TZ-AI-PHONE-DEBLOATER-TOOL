pub mod client;
pub mod gateway;
mod stream;
pub mod types;

pub use client::AiClient;
pub use gateway::{GatewayStatus, ProviderHealth, TzAiGateway, GATEWAY_CONNECTION_ID};
pub use types::{
    ActionPlan, ChatMessage, ClassifiedError, DiscoveredModel, ErrorType, GatewayReadinessInfo,
    GenerationTestResult, ModelDiscoveryResult, ProposedAction, ShortcutIntent, StreamChunkPayload,
    StreamDonePayload, StreamErrorPayload,
};
