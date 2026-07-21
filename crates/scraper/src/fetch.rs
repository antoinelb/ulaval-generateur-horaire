use std::time::{Duration, SystemTime};

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::{Client, ClientBuilder, StatusCode};
use tokio::sync::Mutex;
use tokio::time::{self, Instant};

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("Building HTTP client: {source}")]
    Client {
        #[source]
        source: reqwest::Error,
    },
    #[error(
        "GET {url} failed with status {status} after {attempts} attempt(s)"
    )]
    Status {
        url: String,
        status: StatusCode,
        attempts: u32,
    },
    #[error(
        "GET {url} transport error after {attempts} attempt(s): {source}"
    )]
    Transport {
        url: String,
        attempts: u32,
        #[source]
        source: reqwest::Error,
    },
    #[error("GET {url}: server asked to retry after {asked_secs} s, above the {cap_secs} s cap")]
    RetryAfterTooLong {
        url: String,
        asked_secs: u64,
        cap_secs: u64,
    },
}

pub struct Fetcher {
    client: Client,
    min_interval: Duration,
    backoff: Duration,
    next_allowed: Mutex<Instant>,
}

impl Fetcher {
    pub fn new(
        min_interval: Duration,
        backoff: Duration,
    ) -> Result<Self, FetchError> {
        let builder = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!(
                "ulaval-scheduler-scraper/",
                env!("CARGO_PKG_VERSION"),
                " (antoinelb@proton.me)",
            ));
        Self::from_builder(builder, min_interval, backoff)
    }

    // seam for the build-failure path: `new`'s static config never fails,
    // so the error branch is only reachable by injecting a bad builder
    fn from_builder(
        builder: ClientBuilder,
        min_interval: Duration,
        backoff: Duration,
    ) -> Result<Self, FetchError> {
        let client = builder
            .build()
            .map_err(|source| FetchError::Client { source })?;
        Ok(Self {
            client,
            min_interval,
            backoff,
            next_allowed: Mutex::new(Instant::now()),
        })
    }

    pub async fn fetch(&self, url: &str) -> Result<String, FetchError> {
        const max_attempts: u32 = 3;
        const retry_after_cap_secs: u64 = 300;

        let mut last_error = None;
        for attempt in 1..=max_attempts {
            self.wait_for_slot().await;
            let resp = match self.client.get(url).send().await {
                Ok(resp) => resp,
                Err(source) => {
                    last_error = Some(FetchError::Transport {
                        url: url.to_string(),
                        attempts: attempt,
                        source,
                    });
                    // don't sleep after the last attempt
                    if attempt < max_attempts {
                        time::sleep(self.backoff).await;
                    }
                    continue;
                }
            };
            let status = resp.status();
            if !status.is_success() {
                let error = FetchError::Status {
                    url: url.to_string(),
                    status,
                    attempts: attempt,
                };
                if !should_retry(status) {
                    return Err(error);
                }
                last_error = Some(error);
                match parse_retry_after(resp.headers(), SystemTime::now()) {
                    Some(wait) if wait.as_secs() > retry_after_cap_secs => {
                        return Err(FetchError::RetryAfterTooLong {
                            url: url.to_string(),
                            asked_secs: wait.as_secs(),
                            cap_secs: retry_after_cap_secs,
                        });
                    }
                    Some(wait) => {
                        self.delay_until(Instant::now() + wait).await;
                    }
                    None => {
                        // don't sleep after the last attempt
                        if attempt < max_attempts {
                            time::sleep(self.backoff).await;
                        }
                    }
                }
                continue;
            }

            return resp.text().await.map_err(|source| {
                FetchError::Transport {
                    url: url.to_string(),
                    attempts: attempt,
                    source,
                }
            });
        }

        Err(last_error
            .expect("`max_attempts` >= 1 so the loop body should have set it"))
    }

    async fn wait_for_slot(&self) {
        let mut next_allowed = self.next_allowed.lock().await;
        time::sleep_until(*next_allowed).await;
        *next_allowed = Instant::now() + self.min_interval;
    }

    async fn delay_until(&self, until: Instant) {
        let mut next_allowed = self.next_allowed.lock().await;
        *next_allowed = (*next_allowed).max(until);
    }
}

fn should_retry(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn parse_retry_after(
    headers: &HeaderMap,
    now: SystemTime,
) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
    if let Ok(secs) = value.trim().parse::<u64>() {
        Some(Duration::from_secs(secs))
    } else {
        let date = httpdate::parse_http_date(value.trim()).ok()?;
        Some(date.duration_since(now).unwrap_or(Duration::ZERO))
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    const HTTP_DATE: &str = "Fri, 31 Dec 1999 23:59:59 GMT";

    #[test]
    fn server_errors_and_too_many_requests_are_retryable() {
        for status in [
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::TOO_MANY_REQUESTS,
        ] {
            assert!(should_retry(status), "{status} should be retryable");
        }
    }

    #[test]
    fn successes_and_client_errors_are_not_retryable() {
        for status in
            [StatusCode::OK, StatusCode::FORBIDDEN, StatusCode::NOT_FOUND]
        {
            assert!(!should_retry(status), "{status} should not be retryable");
        }
    }

    #[test]
    fn absent_retry_after_is_none() {
        let headers = HeaderMap::new();

        assert_eq!(parse_retry_after(&headers, SystemTime::UNIX_EPOCH), None);
    }

    #[test]
    fn retry_after_in_seconds_is_parsed() {
        assert_eq!(
            parse_retry_after(&retry_after("120"), SystemTime::UNIX_EPOCH),
            Some(Duration::from_secs(120))
        );
    }

    #[test]
    fn non_ascii_retry_after_is_none() {
        // header values may legally carry opaque bytes (RFC 9110 obs-text),
        // which to_str() refuses — that refusal must read as "no header"
        let mut headers = HeaderMap::new();
        headers.insert(
            RETRY_AFTER,
            reqwest::header::HeaderValue::from_bytes(b"\xff120")
                .unwrap_or_else(|e| panic!("opaque bytes are legal: {e}")),
        );

        assert_eq!(parse_retry_after(&headers, SystemTime::UNIX_EPOCH), None);
    }

    #[test]
    fn unparseable_retry_after_is_none_not_an_error() {
        assert_eq!(
            parse_retry_after(&retry_after("soon"), SystemTime::UNIX_EPOCH),
            None
        );
    }

    #[test]
    fn retry_after_future_date_is_the_delta_from_now() {
        let date = httpdate::parse_http_date(HTTP_DATE)
            .unwrap_or_else(|e| panic!("parse fixed test date: {e}"));
        let now = date - Duration::from_secs(30);

        assert_eq!(
            parse_retry_after(&retry_after(HTTP_DATE), now),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn retry_after_past_date_saturates_to_zero() {
        let date = httpdate::parse_http_date(HTTP_DATE)
            .unwrap_or_else(|e| panic!("parse fixed test date: {e}"));
        let now = date + Duration::from_secs(30);

        assert_eq!(
            parse_retry_after(&retry_after(HTTP_DATE), now),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn invalid_client_config_is_a_client_error() {
        // an invalid user agent is stored by the builder and only surfaces
        // at build() — the one way to exercise the Client error path
        let builder = Client::builder().user_agent("invalid\nvalue");

        let result =
            Fetcher::from_builder(builder, Duration::ZERO, Duration::ZERO);

        assert!(
            matches!(result, Err(FetchError::Client { .. })),
            "expected a Client error from an invalid builder"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn first_slot_is_immediate_and_later_slots_wait_min_interval() {
        let fetcher = fetcher(Duration::from_millis(100));
        let start = Instant::now();

        fetcher.wait_for_slot().await;
        let first = Instant::now();
        fetcher.wait_for_slot().await;
        let second = Instant::now();

        assert_eq!(first, start, "first slot must not wait");
        assert_eq!(
            second.duration_since(first),
            Duration::from_millis(100),
            "second slot must wait the full interval"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn delay_until_pushes_the_shared_clock_and_never_pulls_it_back() {
        let fetcher = fetcher(Duration::ZERO);
        let start = Instant::now();

        fetcher.delay_until(start + Duration::from_secs(30)).await;
        // an earlier deadline (a second, milder Retry-After) must not undo
        // the stricter one
        fetcher.delay_until(start + Duration::from_secs(10)).await;
        fetcher.wait_for_slot().await;

        assert_eq!(
            Instant::now().duration_since(start),
            Duration::from_secs(30),
            "the latest deadline must win"
        );
    }

    fn fetcher(min_interval: Duration) -> Fetcher {
        Fetcher::new(min_interval, Duration::ZERO)
            .unwrap_or_else(|e| panic!("build fetcher: {e}"))
    }

    fn retry_after(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(value)
                .unwrap_or_else(|e| panic!("header value {value}: {e}")),
        );
        headers
    }
}
