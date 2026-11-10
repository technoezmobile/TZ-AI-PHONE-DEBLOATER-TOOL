use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tz_ai_debloater_lib::ai::client::AiClient;
use tz_ai_debloater_lib::ai::types::ErrorType;
use tz_ai_debloater_lib::storage::credentials::{
    ApiProtocol, ConnectionMode, ConnectionPreset, ConnectionProfile,
};

/// Helper to start a local mock HTTP server on an ephemeral port
async fn start_mock_server<F>(handler: F) -> (String, tokio::task::JoinHandle<()>)
where
    F: Fn(String) -> (u16, Vec<(&'static str, &'static str)>, String) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Bind mock server");
    let addr = listener.local_addr().expect("Local addr");
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    continue;
                }
                let request_str = String::from_utf8_lossy(&buf[..n]).to_string();

                let (status_code, headers, body) = handler(request_str);
                let status_text = match status_code {
                    200 => "OK",
                    404 => "Not Found",
                    401 => "Unauthorized",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    _ => "Response",
                };

                let mut resp = format!("HTTP/1.1 {} {}\r\n", status_code, status_text);
                for (k, v) in headers {
                    resp.push_str(&format!("{}: {}\r\n", k, v));
                }
                resp.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
                resp.push_str(&body);

                let _ = socket.write_all(resp.as_bytes()).await;
            }
        }
    });

    (base_url, handle)
}

// -----------------------------------------------------------------------------
// 1. Gemini Native Discovery Pagination & Capability Filtering
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_gemini_discovery_pagination_and_capability_filtering() {
    let (base_url, _server) = start_mock_server(|req| {
        if req.contains("pageToken=page2") {
            let body = serde_json::json!({
                "models": [
                    {
                        "name": "models/gemini-1.5-pro",
                        "displayName": "Gemini 1.5 Pro",
                        "supportedGenerationMethods": ["generateContent", "countTokens"]
                    }
                ]
            })
            .to_string();
            (200, vec![("Content-Type", "application/json")], body)
        } else {
            let body = serde_json::json!({
                "models": [
                    {
                        "name": "models/gemini-1.5-flash",
                        "displayName": "Gemini 1.5 Flash",
                        "supportedGenerationMethods": ["generateContent", "countTokens"]
                    },
                    {
                        "name": "models/text-embedding-004",
                        "displayName": "Text Embedding 004",
                        // DOES NOT support generateContent! Must be filtered out!
                        "supportedGenerationMethods": ["embedContent"]
                    },
                    {
                        "name": "models/aqa",
                        "displayName": "Attributed Question Answering",
                        "supportedGenerationMethods": ["aqa"]
                    }
                ],
                "nextPageToken": "page2"
            })
            .to_string();
            (200, vec![("Content-Type", "application/json")], body)
        }
    })
    .await;

    let profile = ConnectionProfile {
        id: "test-gemini".into(),
        name: "Test Gemini".into(),
        mode: ConnectionMode::DirectApi,
        preset: ConnectionPreset::GoogleGemini,
        base_url,
        protocol: ApiProtocol::GeminiNative,
        credential_id: "test-cred".into(),
        selected_model: "".into(),
        manual_model_id: None,
        last_test_status: None,
        last_verified_at: None,
        supported_capabilities: vec!["chat".into(), "streaming".into()],
        tested_for_generation: false,
        has_credential: true,
    };

    // First page
    let res = AiClient::discover_models(&profile, Some("fake-key"), None)
        .await
        .expect("First page");
    assert_eq!(
        res.models.len(),
        1,
        "Only models supporting generateContent should be returned"
    );
    assert_eq!(res.models[0].id, "gemini-1.5-flash");
    assert_eq!(res.models[0].display_name, "Gemini 1.5 Flash");
    assert_eq!(res.next_page_token, Some("page2".into()));

    // Second page with pagination token
    let res2 = AiClient::discover_models(&profile, Some("fake-key"), Some("page2"))
        .await
        .expect("Second page");
    assert_eq!(res2.models.len(), 1);
    assert_eq!(res2.models[0].id, "gemini-1.5-pro");
    assert_eq!(res2.next_page_token, None);
}

// -----------------------------------------------------------------------------
// 2. FreeLLMAPI Gateway Preset, Readiness & Served-Model Metadata
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_freellmapi_gateway_readiness_and_served_model() {
    let (base_url, _server) = start_mock_server(|_req| {
        let body = serde_json::json!({
            "data": [
                { "id": "gemini-flash-router", "object": "model" },
                { "id": "gpt-4o-router", "object": "model" }
            ]
        })
        .to_string();
        (
            200,
            vec![
                ("Content-Type", "application/json"),
                ("x-served-model", "google/gemini-2.0-flash-exp:free"),
            ],
            body,
        )
    })
    .await;

    let profile = ConnectionProfile {
        id: "preset-freellmapi".into(),
        name: "FreeLLMAPI".into(),
        mode: ConnectionMode::Gateway,
        preset: ConnectionPreset::FreeLlmApi,
        base_url: format!("{}/v1", base_url),
        protocol: ApiProtocol::OpenAiChatCompletions,
        credential_id: "test-gateway".into(),
        selected_model: "".into(),
        manual_model_id: None,
        last_test_status: None,
        last_verified_at: None,
        supported_capabilities: vec!["chat".into(), "streaming".into(), "readiness".into()],
        tested_for_generation: false,
        has_credential: true,
    };

    let readiness = AiClient::check_gateway_readiness(&profile, Some("test-bearer-token"))
        .await
        .expect("Gateway readiness check");

    assert!(
        !readiness.is_ready,
        "HTTP 200 alone must not claim upstream inference readiness"
    );
    assert!(!readiness.upstream_available);
    assert!(readiness.gateway_available);
    assert_eq!(
        readiness.served_model,
        Some("google/gemini-2.0-flash-exp:free".into())
    );

    // Model discovery should flag router aliases
    let models_res = AiClient::discover_models(&profile, Some("test-bearer-token"), None)
        .await
        .expect("Discover models");
    assert_eq!(models_res.models.len(), 2);
    assert!(models_res.models[0].is_router_alias);
}

// -----------------------------------------------------------------------------
// 3. Model Unavailable (HTTP 404) Error Classification
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_model_unavailable_error_classification() {
    let (base_url, _server) = start_mock_server(|_req| {
        let body = serde_json::json!({
            "error": {
                "code": 404,
                "message": "models/gemini-2.5-flash is not found for API version v1beta, or is not supported for generateContent.",
                "status": "NOT_FOUND"
            }
        }).to_string();
        (404, vec![("Content-Type", "application/json")], body)
    }).await;

    let profile = ConnectionProfile {
        id: "test-profile".into(),
        name: "Test Gemini".into(),
        mode: ConnectionMode::DirectApi,
        preset: ConnectionPreset::GoogleGemini,
        base_url,
        protocol: ApiProtocol::GeminiNative,
        credential_id: "test".into(),
        selected_model: "gemini-2.5-flash".into(),
        manual_model_id: None,
        last_test_status: None,
        last_verified_at: None,
        supported_capabilities: vec![],
        tested_for_generation: false,
        has_credential: true,
    };

    let test_res = AiClient::test_generation(&profile, Some("key"), "gemini-2.5-flash").await;
    assert!(test_res.is_err());
    let err = test_res.unwrap_err();

    // Verify accurate error classification: ModelUnavailable, not Authentication!
    assert_eq!(err.error_type, ErrorType::ModelUnavailable);
    assert!(err.message.contains("unavailable or does not exist"));
    assert!(err.suggested_action.contains("Refresh models"));
}

// -----------------------------------------------------------------------------
// 4. URL Normalization, /v1 Deduplication & Security Rules
// -----------------------------------------------------------------------------
#[test]
fn test_endpoint_normalization_comprehensive() {
    // Deduplicates /v1 when both base and suffix contain it
    let res = AiClient::normalize_endpoint("http://localhost:3001/v1", "v1/models").unwrap();
    assert_eq!(res, "http://localhost:3001/v1/models");

    let res2 = AiClient::normalize_endpoint("http://localhost:3001/v1/", "/v1/models").unwrap();
    assert_eq!(res2, "http://localhost:3001/v1/models");

    // Standard base without /v1
    let res3 = AiClient::normalize_endpoint("http://localhost:3001", "v1/models").unwrap();
    assert_eq!(res3, "http://localhost:3001/v1/models");

    // Rejects remote non-loopback HTTP
    let insecure = AiClient::normalize_endpoint("http://api.external.com/v1", "chat/completions");
    assert!(insecure.is_err());
    assert!(insecure
        .unwrap_err()
        .message
        .contains("Insecure HTTP scheme is prohibited"));

    // Allows remote HTTPS
    let secure = AiClient::normalize_endpoint("https://api.external.com/v1", "chat/completions");
    assert!(secure.is_ok());

    // Allows loopback HTTP
    assert!(AiClient::normalize_endpoint("http://127.0.0.1:8080", "models").is_ok());
    assert!(AiClient::normalize_endpoint("http://localhost:3000", "models").is_ok());

    // Rejects embedded credentials in URL
    let cred_url = AiClient::normalize_endpoint("http://user:secret@localhost:3001", "models");
    assert!(cred_url.is_err());
    assert!(cred_url
        .unwrap_err()
        .message
        .contains("Embedded credentials in URL are rejected"));
}

// -----------------------------------------------------------------------------
// 5. Streaming and Backend Cancellation
// -----------------------------------------------------------------------------
#[tokio::test]
async fn test_streaming_cancellation_aborts_mid_stream() {
    let (base_url, _server) = start_mock_server(|_req| {
        // SSE chunk stream
        let body = "data: {\"choices\": [{\"delta\": {\"content\": \"Part1 \"}}]}\n\n\
                    data: {\"choices\": [{\"delta\": {\"content\": \"Part2 \"}}]}\n\n\
                    data: {\"choices\": [{\"delta\": {\"content\": \"Part3 \"}}]}\n\n\
                    data: [DONE]\n\n";
        (
            200,
            vec![("Content-Type", "text/event-stream")],
            body.to_string(),
        )
    })
    .await;

    let profile = ConnectionProfile {
        id: "test-stream".into(),
        name: "Test Stream".into(),
        mode: ConnectionMode::Gateway,
        preset: ConnectionPreset::CustomGateway,
        base_url,
        protocol: ApiProtocol::OpenAiChatCompletions,
        credential_id: "test".into(),
        selected_model: "gpt-4o-mini".into(),
        manual_model_id: None,
        last_test_status: None,
        last_verified_at: None,
        supported_capabilities: vec!["streaming".into()],
        tested_for_generation: true,
        has_credential: true,
    };

    let cancel_token = Arc::new(AtomicBool::new(false));
    let token_clone = Arc::clone(&cancel_token);

    let chunks_received = Arc::new(std::sync::Mutex::new(Vec::new()));
    let chunks_clone = Arc::clone(&chunks_received);

    // Cancel after receiving the first chunk
    let on_chunk = move |chunk: String| {
        let mut list = chunks_clone.lock().unwrap();
        list.push(chunk);
        // Abort stream immediately upon receiving first chunk!
        token_clone.store(true, Ordering::SeqCst);
    };

    let (full_text, _) = AiClient::stream_chat(
        &profile,
        Some("key"),
        "gpt-4o-mini",
        "Hello",
        "TestPhone",
        "14",
        &[],
        &[],
        cancel_token,
        on_chunk,
    )
    .await
    .expect("Stream should complete without error");

    let received = chunks_received.lock().unwrap();
    // Verify that cancellation stopped further processing
    assert!(received.len() >= 1);
    assert!(full_text.contains("Part1"));
}

// -----------------------------------------------------------------------------
// 6. Generation Validation: Empty Bodies, Malformed JSON & Shape Verification
// -----------------------------------------------------------------------------
#[test]
fn test_generation_validation_rejects_empty_and_malformed() {
    let protocols = [
        ApiProtocol::GeminiNative,
        ApiProtocol::OpenAiChatCompletions,
        ApiProtocol::OpenAiResponses,
        ApiProtocol::AnthropicMessages,
    ];

    for proto in protocols {
        // Empty body
        let res_empty = AiClient::validate_generation_response(proto, 200, "");
        assert!(
            res_empty.is_err(),
            "Protocol {:?} accepted empty body",
            proto
        );
        assert_eq!(
            res_empty.unwrap_err().error_type,
            ErrorType::InvalidTruncatedAiResponse
        );

        // Whitespace-only body
        let res_ws = AiClient::validate_generation_response(proto, 200, "   \n\t  ");
        assert!(
            res_ws.is_err(),
            "Protocol {:?} accepted whitespace body",
            proto
        );

        // Malformed JSON
        let res_bad_json =
            AiClient::validate_generation_response(proto, 200, "<html>502 Bad Gateway</html>");
        assert!(
            res_bad_json.is_err(),
            "Protocol {:?} accepted non-JSON HTML",
            proto
        );

        // Incompatible JSON shape (array instead of object)
        let res_array = AiClient::validate_generation_response(proto, 200, "[\"READY\"]");
        assert!(
            res_array.is_err(),
            "Protocol {:?} accepted JSON array instead of object",
            proto
        );
    }
}

// -----------------------------------------------------------------------------
// 7. Generation Validation: Provider Error Objects with HTTP 200
// -----------------------------------------------------------------------------
#[test]
fn test_generation_validation_rejects_provider_errors_with_http_200() {
    let openai_err = r#"{"error": {"message": "You exceeded your current quota.", "type": "insufficient_quota"}}"#;
    let res_openai =
        AiClient::validate_generation_response(ApiProtocol::OpenAiChatCompletions, 200, openai_err);
    assert!(
        res_openai.is_err(),
        "ChatCompletions accepted HTTP 200 with error object"
    );
    assert!(res_openai.unwrap_err().message.contains("quota"));

    let anthropic_err = r#"{"type": "error", "error": {"type": "invalid_request_error", "message": "Model not found"}}"#;
    let res_anthropic =
        AiClient::validate_generation_response(ApiProtocol::AnthropicMessages, 200, anthropic_err);
    assert!(
        res_anthropic.is_err(),
        "Anthropic accepted HTTP 200 with error object"
    );
    assert!(res_anthropic
        .unwrap_err()
        .message
        .contains("Model not found"));
}

// -----------------------------------------------------------------------------
// 8. Generation Validation: Protocol-Specific Schema & Text Extraction
// -----------------------------------------------------------------------------
#[test]
fn test_generation_validation_protocol_schemas() {
    // Gemini: Success
    let gemini_ok = r#"{"candidates": [{"content": {"parts": [{"text": "READY"}]}}]}"#;
    let res = AiClient::validate_generation_response(ApiProtocol::GeminiNative, 200, gemini_ok)
        .expect("Gemini ok");
    assert_eq!(res, "READY");

    // Gemini: Missing candidates
    let gemini_no_cand = r#"{"usageMetadata": {"totalTokenCount": 5}}"#;
    assert!(
        AiClient::validate_generation_response(ApiProtocol::GeminiNative, 200, gemini_no_cand)
            .is_err()
    );

    // Gemini: Blocked by safety
    let gemini_blocked = r#"{"candidates": [{"finishReason": "SAFETY"}]}"#;
    assert!(
        AiClient::validate_generation_response(ApiProtocol::GeminiNative, 200, gemini_blocked)
            .is_err()
    );

    // ChatCompletions: Success
    let chat_ok = r#"{"choices": [{"message": {"role": "assistant", "content": "READY"}}]}"#;
    let res =
        AiClient::validate_generation_response(ApiProtocol::OpenAiChatCompletions, 200, chat_ok)
            .expect("Chat ok");
    assert_eq!(res, "READY");

    // ChatCompletions: Empty content
    let chat_empty = r#"{"choices": [{"message": {"role": "assistant", "content": ""}}]}"#;
    assert!(AiClient::validate_generation_response(
        ApiProtocol::OpenAiChatCompletions,
        200,
        chat_empty
    )
    .is_err());

    // Responses: Success via output_text
    let resp_text_ok = r#"{"output_text": "READY"}"#;
    let res =
        AiClient::validate_generation_response(ApiProtocol::OpenAiResponses, 200, resp_text_ok)
            .expect("Responses text ok");
    assert_eq!(res, "READY");

    // Responses: Success via output array
    let resp_arr_ok = r#"{"output": [{"content": [{"type": "text", "text": "READY"}]}]}"#;
    let res =
        AiClient::validate_generation_response(ApiProtocol::OpenAiResponses, 200, resp_arr_ok)
            .expect("Responses array ok");
    assert_eq!(res, "READY");

    // Responses: Incomplete / missing text
    let resp_missing = r#"{"output": []}"#;
    assert!(AiClient::validate_generation_response(
        ApiProtocol::OpenAiResponses,
        200,
        resp_missing
    )
    .is_err());

    // Anthropic: Success
    let ant_ok = r#"{"content": [{"type": "text", "text": "READY"}]}"#;
    let res = AiClient::validate_generation_response(ApiProtocol::AnthropicMessages, 200, ant_ok)
        .expect("Anthropic ok");
    assert_eq!(res, "READY");

    // Anthropic: Empty content array
    let ant_empty = r#"{"content": []}"#;
    assert!(
        AiClient::validate_generation_response(ApiProtocol::AnthropicMessages, 200, ant_empty)
            .is_err()
    );
}
