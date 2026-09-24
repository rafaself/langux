use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use serde_json::Value;

use crate::{
    CancellationToken, LanguageCode, SourceLanguage, TranslationError, TranslationProvider,
    TranslationRequest,
};

use super::GoogleTranslationProvider;

const TEST_API_KEY: &str = "test-key-that-must-not-appear-in-a-url";

struct MockServer {
    endpoint: String,
    requests: Receiver<Vec<u8>>,
    worker: thread::JoinHandle<()>,
}

impl MockServer {
    fn start(responses: Vec<(u16, String)>) -> Self {
        Self::start_with_delay(responses, Duration::ZERO)
    }

    fn start_with_delay(responses: Vec<(u16, String)>, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let address = listener.local_addr().expect("read mock server address");
        let (sender, requests) = mpsc::channel();
        let worker = thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().expect("accept provider request");
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .expect("set mock read timeout");
                let request = read_request(&mut stream);
                sender.send(request).expect("send captured request");
                if !delay.is_zero() {
                    thread::sleep(delay);
                }
                let response = format!(
                    "HTTP/1.1 {status} Test\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        Self {
            endpoint: format!("http://{address}/language/translate/v2"),
            requests,
            worker,
        }
    }

    fn receive_request(&self) -> Vec<u8> {
        self.requests
            .recv_timeout(Duration::from_secs(3))
            .expect("provider sent request")
    }

    fn finish(self) {
        self.worker.join().expect("mock server completed");
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let count = stream.read(&mut chunk).expect("read provider request");
        if count == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..count]);
        if request_is_complete(&request) {
            break;
        }
    }
    request
}

fn request_is_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    request.len() >= header_end + 4 + content_length
}

fn provider(server: &MockServer) -> GoogleTranslationProvider {
    GoogleTranslationProvider::build(TEST_API_KEY, &server.endpoint, false)
        .expect("valid test provider")
}

fn request(source_language: SourceLanguage) -> TranslationRequest {
    TranslationRequest::new(
        "olá mundo",
        source_language,
        LanguageCode::new("en").expect("valid target language"),
    )
}

fn request_body(request: &[u8]) -> Value {
    let header_end = request
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .expect("request headers end");
    serde_json::from_slice(&request[header_end + 4..]).expect("request JSON body")
}

#[test]
fn auto_detect_sends_the_key_in_a_sensitive_header_and_normalizes_result() {
    let server = MockServer::start(vec![(
            200,
            r#"{"data":{"translations":[{"translatedText":"hello world","detectedSourceLanguage":"pt"}]}}"#.into(),
        )]);
    let provider = provider(&server);
    let cancellation = CancellationToken::new();
    let result = provider
        .translate(&request(SourceLanguage::AutoDetect), &cancellation)
        .expect("translation succeeds");
    let captured = server.receive_request();
    let captured = String::from_utf8(captured).expect("request is UTF-8");
    let (headers, body) = captured.split_once("\r\n\r\n").expect("request body");

    assert_eq!(
        result,
        crate::TranslationResult::new(
            "hello world",
            Some(LanguageCode::new("pt").expect("valid detected language")),
        )
    );
    assert!(headers.starts_with("POST /language/translate/v2 HTTP/1.1\r\n"));
    assert!(!headers.lines().next().unwrap_or_default().contains('?'));
    assert!(
        headers
            .to_ascii_lowercase()
            .contains("x-goog-api-key: test-key-that-must-not-appear-in-a-url")
    );
    assert!(provider.api_key.is_sensitive());
    assert!(!format!("{:?}", provider.api_key).contains(TEST_API_KEY));
    assert!(!body.contains(TEST_API_KEY));
    let body = request_body(captured.as_bytes());
    assert_eq!(body["q"], "olá mundo");
    assert_eq!(body["target"], "en");
    assert_eq!(body["format"], "text");
    assert!(body.get("source").is_none());
    server.finish();
}

#[test]
fn explicit_source_is_sent_and_response_without_detection_is_normalized() {
    let server = MockServer::start(vec![(
        200,
        r#"{"data":{"translations":[{"translatedText":"hello world"}]}}"#.into(),
    )]);
    let provider = provider(&server);
    let result = provider
        .translate(
            &request(SourceLanguage::Specific(
                LanguageCode::new("pt-BR").expect("valid source language"),
            )),
            &CancellationToken::new(),
        )
        .expect("translation succeeds");
    let captured = server.receive_request();

    assert_eq!(result, crate::TranslationResult::new("hello world", None));
    assert_eq!(request_body(&captured)["source"], "pt-BR");
    server.finish();
}

#[test]
fn common_provider_failures_map_to_normalized_categories() {
    let server = MockServer::start(vec![
        (401, r#"{"error":{"status":"UNAUTHENTICATED"}}"#.into()),
        (429, r#"{"error":{"status":"RESOURCE_EXHAUSTED"}}"#.into()),
        (
            403,
            r#"{"error":{"status":"PERMISSION_DENIED","errors":[{"reason":"rateLimitExceeded"}]}}"#
                .into(),
        ),
        (
            400,
            r#"{"error":{"status":"INVALID_ARGUMENT","errors":[{"reason":"API_KEY_INVALID"}]}}"#
                .into(),
        ),
        (500, r#"{"error":{"status":"INTERNAL"}}"#.into()),
    ]);
    let provider = provider(&server);
    let outcomes = (0..5)
        .map(|_| {
            provider.translate(
                &request(SourceLanguage::AutoDetect),
                &CancellationToken::new(),
            )
        })
        .collect::<Vec<_>>();
    for _ in 0..5 {
        server.receive_request();
    }

    assert_eq!(outcomes[0], Err(TranslationError::UnauthorizedCredential));
    assert_eq!(outcomes[1], Err(TranslationError::QuotaOrRateLimit));
    assert_eq!(outcomes[2], Err(TranslationError::QuotaOrRateLimit));
    assert_eq!(outcomes[3], Err(TranslationError::UnauthorizedCredential));
    assert_eq!(outcomes[4], Err(TranslationError::ProviderFailure));
    server.finish();
}

#[test]
fn malformed_success_response_maps_to_malformed_response() {
    let server = MockServer::start(vec![(200, "{}".into())]);
    let result = provider(&server).translate(
        &request(SourceLanguage::AutoDetect),
        &CancellationToken::new(),
    );

    assert_eq!(result, Err(TranslationError::MalformedResponse));
    server.receive_request();
    server.finish();
}

#[test]
fn cancellation_aborts_an_in_flight_request() {
    let server = MockServer::start_with_delay(
        vec![(
            200,
            r#"{"data":{"translations":[{"translatedText":"hello"}]}}"#.into(),
        )],
        Duration::from_millis(500),
    );
    let provider = provider(&server);
    let operation = request(SourceLanguage::AutoDetect);
    let cancellation = CancellationToken::new();
    let worker_cancellation = cancellation.clone();
    let worker = thread::spawn(move || provider.translate(&operation, &worker_cancellation));
    server.receive_request();
    cancellation.cancel();

    assert_eq!(
        worker.join().expect("provider worker completes"),
        Err(TranslationError::Cancelled)
    );
    server.finish();
}

#[test]
fn missing_or_invalid_credentials_are_rejected_without_a_request() {
    assert!(matches!(
        GoogleTranslationProvider::new(""),
        Err(TranslationError::MissingCredential)
    ));
    assert!(matches!(
        GoogleTranslationProvider::new("invalid\nkey"),
        Err(TranslationError::UnauthorizedCredential)
    ));
}
