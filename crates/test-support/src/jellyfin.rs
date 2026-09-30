use serde_json::json;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{header, method, path},
};

/// Jellyfin's system info for `api_key`, reporting `version`; unmounted.
pub fn system_info(api_key: &str, version: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path("/System/Info"))
        .and(header("Authorization", format!("MediaBrowser Token=\"{api_key}\"").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ServerName": "media", "Version": version })))
}
