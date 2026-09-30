use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{body_partial_json, header, method, path},
};

/// The RPC path, under the server's address.
pub const RPC: &str = "/transmission/rpc";

pub const SESSION: &str = "6qXR0iKsWG3NkpqOtgfvVFzN";

/// A Transmission that answers every request without the session id with 409, as Transmission
/// does; add answers with [`answer`].
pub async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(RPC))
        .and(|request: &Request| !request.headers.contains_key("X-Transmission-Session-Id"))
        .respond_with(ResponseTemplate::new(409).insert_header("X-Transmission-Session-Id", SESSION))
        .mount(&server)
        .await;
    server
}

/// Answers `rpc_method` calls carrying the session id with `response`.
pub async fn answer(server: &MockServer, rpc_method: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(RPC))
        .and(header("X-Transmission-Session-Id", SESSION))
        .and(body_partial_json(json!({ "method": rpc_method })))
        .respond_with(response)
        .mount(server)
        .await;
}

pub fn success(arguments: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "arguments": arguments, "result": "success" }))
}
