use std::time::Duration;

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use jiff::civil::Date;
use reqwest::{Client, IntoUrl, RequestBuilder, Response, StatusCode, header::RETRY_AFTER};
use serde::{Deserialize, de::DeserializeOwned};
use tokio::time::{Instant, sleep};
use tracing::debug;
use yokoku_core::library::ports::MetadataError;
use yokoku_domain::ExternalId;

const USER_AGENT: &str = concat!("yokoku/", env!("CARGO_PKG_VERSION"));
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT: Duration = Duration::from_secs(30);
/// At most 40 requests a second.
const SPACING: Duration = Duration::from_millis(25);
const MAX_ATTEMPTS: u32 = 3;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// An HTTP client for one metadata source: spaces requests, retries temporary failures and maps
/// failed answers to `MetadataError`.
pub(crate) struct Http {
    client: Client,
    source: &'static str,
    limiter: DefaultDirectRateLimiter,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(alias = "status_message")]
    message: Option<String>,
}

impl Http {
    /// `source` names the source in logs and errors, e.g. `TMDB`.
    pub(crate) fn new(source: &'static str) -> Self {
        Self::with_timeout(source, TIMEOUT)
    }

    fn with_timeout(source: &'static str, timeout: Duration) -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()
            .expect("TLS backend initializes");
        let quota = Quota::with_period(SPACING).expect("SPACING is not zero");
        Self { client, source, limiter: RateLimiter::direct(quota) }
    }

    pub(crate) fn get(&self, url: impl IntoUrl) -> RequestBuilder {
        self.client.get(url)
    }

    pub(crate) fn post(&self, url: impl IntoUrl) -> RequestBuilder {
        self.client.post(url)
    }

    /// Sends `request`, trying up to three times while the source is rate limited, overloaded or
    /// unreachable. A 404 for `item` is `NotFound`; 401 and 403 are `Refused`.
    pub(crate) async fn send(
        &self,
        request: RequestBuilder,
        item: Option<ExternalId>,
    ) -> Result<Response, MetadataError> {
        let mut attempt = 1;
        loop {
            self.limiter.until_ready().await;
            let started = Instant::now();
            let result = request.try_clone().expect("requests have no streamed body").send().await;
            let wait = match &result {
                Ok(response) => {
                    debug!(
                        source = self.source,
                        url = %response.url(),
                        status = response.status().as_u16(),
                        elapsed_ms = started.elapsed().as_millis(),
                        "metadata request"
                    );
                    temporary(response.status()).then(|| retry_after(response).unwrap_or(backoff(attempt)))
                },
                Err(error) => (error.is_connect() || error.is_timeout()).then(|| backoff(attempt)),
            };
            match wait {
                Some(wait) if attempt < MAX_ATTEMPTS => {
                    debug!(source = self.source, attempt, wait_ms = wait.as_millis(), "retrying metadata request");
                    sleep(wait).await;
                    attempt += 1;
                },
                _ => return self.check(result.map_err(|error| MetadataError::Unavailable(error.into()))?, item).await,
            }
        }
    }

    async fn check(&self, response: Response, item: Option<ExternalId>) -> Result<Response, MetadataError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        if let Some(item) = item
            && status == StatusCode::NOT_FOUND
        {
            return Err(MetadataError::NotFound(item));
        }
        let reason = match response.json::<ErrorBody>().await.ok().and_then(|body| body.message) {
            Some(message) => format!("{} answered {status}: {message}", self.source),
            None => format!("{} answered {status}", self.source),
        };
        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(MetadataError::Refused(reason)),
            _ => Err(MetadataError::Unavailable(reason.into())),
        }
    }
}

/// Reads the body as JSON; a body of another shape is `Invalid`.
pub(crate) async fn json<T: DeserializeOwned>(response: Response) -> Result<T, MetadataError> {
    let body = response.bytes().await.map_err(|error| MetadataError::Unavailable(error.into()))?;
    serde_json::from_slice(&body).map_err(|error| MetadataError::Invalid(error.into()))
}

/// `2021-10-22` or `2021-10-22T00:00:00.000Z`; empty strings are missing dates.
pub(crate) fn date(value: Option<&str>) -> Option<Date> {
    value?.get(..10)?.parse().ok()
}

pub(crate) fn year(value: Option<&str>) -> Option<i16> {
    value?.get(..4)?.parse().ok()
}

fn temporary(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

/// `Retry-After` in seconds, capped at `MAX_RETRY_AFTER`.
fn retry_after(response: &Response) -> Option<Duration> {
    let seconds = response.headers().get(RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()?;
    Some(Duration::from_secs(seconds).min(MAX_RETRY_AFTER))
}

/// 1 s, then 2 s.
fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(1 << (attempt - 1))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::Instant;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};
    use yokoku_core::library::ports::MetadataError;

    use super::{Http, SPACING};

    #[tokio::test]
    async fn requests_are_spaced() {
        let server = MockServer::start().await;
        Mock::given(path("/")).respond_with(ResponseTemplate::new(200)).mount(&server).await;
        let http = Http::new("test");

        let started = Instant::now();
        for _ in 0..5 {
            http.send(http.get(server.uri()), None).await.unwrap();
        }

        assert!(started.elapsed() >= SPACING * 4);
    }

    #[tokio::test(start_paused = true)]
    async fn slow_answers_time_out() {
        let server = MockServer::start().await;
        let slow = ResponseTemplate::new(200).set_delay(Duration::from_secs(5));
        Mock::given(path("/")).respond_with(slow).mount(&server).await;
        let http = Http::with_timeout("test", Duration::from_millis(50));

        let started = Instant::now();
        let error = http.send(http.get(server.uri()), None).await.unwrap_err();

        assert!(matches!(error, MetadataError::Unavailable(_)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
