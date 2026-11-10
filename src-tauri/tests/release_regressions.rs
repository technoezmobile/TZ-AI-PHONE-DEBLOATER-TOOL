use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tz_ai_debloater_lib::{
    ai::AiClient,
    storage::credentials::{ApiProtocol, ConnectionProfile},
};

async fn server(parts: Vec<Vec<u8>>, stall: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 65536];
        let _ = socket.read(&mut request).await;
        if stall {
            tokio::time::sleep(Duration::from_secs(10)).await;
            return;
        }
        let length: usize = parts.iter().map(Vec::len).sum();
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        for part in parts {
            let _ = socket.write_all(&part).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    base
}
fn profile(url: String, protocol: ApiProtocol) -> ConnectionProfile {
    let mut p = ConnectionProfile::new_freellmapi();
    p.base_url = url;
    p.protocol = protocol;
    p
}

#[tokio::test]
async fn split_utf8_stream_preserves_text() {
    let bytes =
        "data: {\"choices\":[{\"delta\":{\"content\":\"नमस्ते\"}}]}\n\ndata: [DONE]\n\n".as_bytes();
    let split = bytes.iter().position(|b| *b >= 128).unwrap() + 1;
    let url = server(
        vec![bytes[..split].to_vec(), bytes[split..].to_vec()],
        false,
    )
    .await;
    let p = profile(url, ApiProtocol::OpenAiChatCompletions);
    let (text, _) = AiClient::stream_chat(
        &p,
        None,
        "test",
        "hello",
        "phone",
        "14",
        &[],
        &[],
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(text, "नमस्ते");
}
#[tokio::test]
async fn truncated_stream_is_never_a_completed_plan() {
    let url = server(
        vec![b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec()],
        false,
    )
    .await;
    let p = profile(url, ApiProtocol::OpenAiChatCompletions);
    assert!(AiClient::stream_chat(
        &p,
        None,
        "test",
        "hello",
        "phone",
        "14",
        &[],
        &[],
        Arc::new(AtomicBool::new(false)),
        |_| {}
    )
    .await
    .is_err());
}
#[tokio::test]
async fn cancellation_interrupts_waiting_for_headers() {
    let url = server(vec![], true).await;
    let p = profile(url, ApiProtocol::OpenAiChatCompletions);
    let token = Arc::new(AtomicBool::new(false));
    let t = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        t.store(true, Ordering::SeqCst);
    });
    let start = Instant::now();
    let (text, _) = AiClient::stream_chat(
        &p,
        None,
        "test",
        "hello",
        "phone",
        "14",
        &[],
        &[],
        token,
        |_| {},
    )
    .await
    .unwrap();
    assert!(text.is_empty());
    assert!(start.elapsed() < Duration::from_secs(2));
}
#[tokio::test]
async fn responses_ignores_non_text_delta_events() {
    let url=server(vec![b"data: {\"type\":\"response.reasoning_summary_text.delta\",\"delta\":\"private\"}\n\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"visible\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"model\":\"served-model\"}}\n\n".to_vec()],false).await;
    let p = profile(url, ApiProtocol::OpenAiResponses);
    let (text, model) = AiClient::stream_chat(
        &p,
        None,
        "test",
        "hello",
        "phone",
        "14",
        &[],
        &[],
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(text, "visible");
    assert_eq!(model.as_deref(), Some("served-model"));
}
#[test]
fn endpoint_validation_rejects_non_http_even_on_loopback() {
    assert!(AiClient::normalize_endpoint("ftp://localhost", "models").is_err());
    assert!(AiClient::normalize_endpoint("https://example.com/?key=secret", "models").is_err());
    assert_eq!(
        AiClient::normalize_endpoint("https://example.com/v1beta", "v1beta/models").unwrap(),
        "https://example.com/v1beta/models"
    );
}
#[test]
fn action_type_is_required() {
    assert!(AiClient::extract_plan_from_text(
        r#"{"actions":[{"package_id":"com.example.app"}]}"#,
        "serial",
        0
    )
    .is_none());
}
