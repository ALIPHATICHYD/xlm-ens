#[cfg(test)]
mod fault_injection_tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use stellar_rpc_client::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};
    use xlm_ns_sdk::config::ClientConfig;
    use xlm_ns_sdk::errors::SdkError;
    use xlm_ns_sdk::network;

    #[tokio::test]
    async fn test_timeout_scenario() {
        // Create a mock server that delays response longer than typical timeout
        let mock_server = MockServer::start().await;

        // Set up a delay longer than what we'll set as timeout in our client
        let delay_responder = DelayResponder::new(
            3500, // 3.5 seconds delay
            ResponseTemplate::new(200).set_body_json(jsonrpc_success_response()),
        );

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(delay_responder)
            .mount(&mock_server)
            .await;

        // Create HTTP client pointing to the mock server
        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a timeout error
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::Transport(msg) => {
                assert!(
                    msg.contains("timeout")
                        || msg.contains("timed out")
                        || msg.contains("failed to get network")
                );
            }
            other => panic!("Expected transport error (timeout), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_malformed_json_response() {
        let mock_server = MockServer::start().await;

        // Return invalid JSON that cannot be parsed
        let malformed_responder = MalformedJsonResponder::new("{\"invalid json\"}");

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(malformed_responder)
            .mount(&mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a deserialization/invalid request error
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::InvalidRequest(msg) | SdkError::Transport(msg) => {
                // Should contain JSON parsing error indication
                assert!(
                    msg.contains("JSON")
                        || msg.contains("invalid")
                        || msg.contains("parse")
                        || msg.contains("failed to get network")
                );
            }
            other => panic!("Expected JSON parse error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_partial_response() {
        let mut mock_server = MockServer::start().await;

        // Return truncated JSON response (missing closing braces)
        let partial_responder = PartialJsonResponder::new(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"passphrase\":\"Test\"}",
        );

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(partial_responder)
            .mount(&mut mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a deserialization error
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::InvalidRequest(msg) | SdkError::Transport(msg) => {
                assert!(
                    msg.contains("JSON")
                        || msg.contains("invalid")
                        || msg.contains("parse")
                        || msg.contains("failed to get network")
                );
            }
            other => panic!("Expected JSON parse error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_connection_reset_simulation() {
        let mut mock_server = MockServer::start().await;

        // Simulate connection reset by sending partial response
        let valid_response = jsonrpc_success_response().to_string();
        let connection_reset_responder = ConnectionResetResponder::new(
            // We need to create the bytes and length without moving the string multiple times
            {
                let response_bytes = valid_response.clone().into_bytes();
                let length = valid_response.len();
                (response_bytes, length / 2)
            }
            .0,
            {
                let length = valid_response.len();
                length / 2
            },
        );

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(connection_reset_responder)
            .mount(&mut mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return an error due to incomplete/invalid response
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::InvalidRequest(msg) | SdkError::Transport(msg) => {
                // Connection reset should result in some kind of parsing/transport error
                assert!(!msg.is_empty());
            }
            other => panic!("Expected connection error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_http_500_error() {
        let mut mock_server = MockServer::start().await;

        // Return HTTP 500 Internal Server Error
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mut mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a transport error
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::Transport(msg) => {
                // Should contain error information
                assert!(!msg.is_empty());
                assert!(msg.contains("500") || msg.contains("failed to get network"));
            }
            other => panic!("Expected transport error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_http_503_service_unavailable() {
        let mut mock_server = MockServer::start().await;

        // Return HTTP 503 Service Unavailable (should be retryable)
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mut mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a transport error (503 is treated as transient but still an error at this level)
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::Transport(msg) => {
                // Should contain error information
                assert!(!msg.is_empty());
                assert!(msg.contains("503") || msg.contains("failed to get network"));
            }
            other => panic!("Expected transport error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_rate_limit_429_error() {
        let mut mock_server = MockServer::start().await;

        // Return HTTP 429 Too Many Requests
        let mut response = ResponseTemplate::new(429);
        response = response.insert_header("Retry-After", "1");

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(response)
            .mount(&mut mock_server)
            .await;

        let http_client = Client::new(&mock_server.uri()).expect("Failed to create HTTP client");

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        // Should return a transport error
        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::Transport(msg) => {
                // Should contain error information
                assert!(!msg.is_empty());
                assert!(msg.contains("429") || msg.contains("failed to get network"));
            }
            other => panic!("Expected transport error, got {other:?}"),
        }
    }

    /// A wiremock responder that delays response beyond a specified duration
    struct DelayResponder {
        delay_ms: u64,
        response: ResponseTemplate,
    }

    impl DelayResponder {
        fn new(delay_ms: u64, response: ResponseTemplate) -> Self {
            Self { delay_ms, response }
        }
    }

    impl Respond for DelayResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            // Sleep for the specified duration
            std::thread::sleep(std::time::Duration::from_millis(self.delay_ms));
            self.response.clone()
        }
    }

    /// A wiremock responder that closes connection early (simulates connection reset)
    struct ConnectionResetResponder {
        response_bytes: Vec<u8>,
        close_at_byte: usize,
    }

    impl ConnectionResetResponder {
        fn new(response_bytes: Vec<u8>, close_at_byte: usize) -> Self {
            Self {
                response_bytes,
                close_at_byte,
            }
        }
    }

    impl Respond for ConnectionResetResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            // This is a simplification - wiremock doesn't easily allow mid-response connection closure
            // For a real implementation, we might need a custom TCP mock or different approach
            // For now, we'll return what we can send before the "disconnection"
            let send_bytes = std::cmp::min(self.close_at_byte, self.response_bytes.len());
            let partial_response = &self.response_bytes[..send_bytes];
            ResponseTemplate::new(200).set_body_raw(partial_response.to_vec(), "application/json")
        }
    }

    /// A wiremock responder that returns malformed JSON
    struct MalformedJsonResponder {
        invalid_json: &'static str,
    }

    impl MalformedJsonResponder {
        fn new(invalid_json: &'static str) -> Self {
            Self { invalid_json }
        }
    }

    impl Respond for MalformedJsonResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            ResponseTemplate::new(200)
                .set_body_raw(self.invalid_json.as_bytes(), "application/json")
        }
    }

    /// A wiremock responder that returns truncated/partial JSON responses
    struct PartialJsonResponder {
        partial_json: &'static str,
    }

    impl PartialJsonResponder {
        fn new(partial_json: &'static str) -> Self {
            Self { partial_json }
        }
    }

    impl Respond for PartialJsonResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            ResponseTemplate::new(200)
                .set_body_raw(self.partial_json.as_bytes(), "application/json")
        }
    }

    fn jsonrpc_success_response() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "passphrase": "Test SDF Network ; September 2015",
                "protocolVersion": 21
            }
        })
    }
}
