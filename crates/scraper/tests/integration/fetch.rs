use std::time::Duration;

use reqwest::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use ulaval_scheduler_scraper::fetch::{FetchError, Fetcher};

#[tokio::test]
async fn success_returns_the_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(200).set_body_string("bonjour"))
        .expect(1)
        .mount(&server)
        .await;

    let body = fetcher()
        .fetch(&format!("{}/page", server.uri()))
        .await
        .unwrap_or_else(|e| panic!("fetch on 200: {e}"));

    assert_eq!(body, "bonjour");
}

#[tokio::test]
async fn retryable_status_with_retry_after_is_retried_until_success() {
    let server = MockServer::start().await;
    // mounted first + up_to_n_times(1): consumed by the first request, the
    // second falls through to the 200 mock below
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(503).insert_header("retry-after", "0"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("enfin"))
        .expect(1)
        .mount(&server)
        .await;

    let body = fetcher()
        .fetch(&server.uri())
        .await
        .unwrap_or_else(|e| panic!("fetch after one 503: {e}"));

    assert_eq!(body, "enfin");
}

#[tokio::test]
async fn permanent_status_fails_on_the_first_attempt() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let error = fetcher()
        .fetch(&server.uri())
        .await
        .expect_err("a 404 must fail");

    assert!(
        matches!(
            &error,
            FetchError::Status { status, attempts: 1, .. }
                if *status == StatusCode::NOT_FOUND
        ),
        "expected permanent Status error after 1 attempt, got {error:?}"
    );
}

#[tokio::test]
async fn retries_are_exhausted_after_three_attempts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(3)
        .mount(&server)
        .await;

    let error = fetcher()
        .fetch(&server.uri())
        .await
        .expect_err("persistent 500s must fail");

    assert!(
        matches!(&error, FetchError::Status { attempts: 3, .. }),
        "expected Status error after 3 attempts, got {error:?}"
    );
}

#[tokio::test]
async fn retry_after_above_the_cap_stops_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(503).insert_header("retry-after", "9999"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let error = fetcher()
        .fetch(&server.uri())
        .await
        .expect_err("an over-cap Retry-After must fail");

    assert!(
        matches!(
            &error,
            FetchError::RetryAfterTooLong {
                asked_secs: 9999,
                ..
            }
        ),
        "expected RetryAfterTooLong, got {error:?}"
    );
}

#[tokio::test]
async fn success_status_with_a_severed_body_is_a_transport_error() {
    // wiremock can't truncate a body mid-stream, so hand-roll the one
    // response this needs: promise 100 bytes, send 5, close the socket
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| panic!("bind a local port: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| panic!("read the bound address: {e}"));
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\nbonjo",
            );
        } // the stream drops here, closing the connection 95 bytes short
    });

    let error = fetcher()
        .fetch(&format!("http://{addr}"))
        .await
        .expect_err("a truncated body must fail");

    assert!(
        matches!(&error, FetchError::Transport { attempts: 1, .. }),
        "expected Transport error on the body read, got {error:?}"
    );
}

#[tokio::test]
async fn transport_errors_are_retried_then_reported() {
    // reserve a free port with a plain listener, then drop it: unlike an
    // async server's delayed shutdown, closing the socket is synchronous,
    // so connecting is guaranteed to be refused
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| panic!("bind a throwaway port: {e}"));
    let port = listener
        .local_addr()
        .unwrap_or_else(|e| panic!("read the bound port: {e}"))
        .port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}");

    let error = fetcher()
        .fetch(&url)
        .await
        .expect_err("a refused connection must fail");

    assert!(
        matches!(&error, FetchError::Transport { attempts: 3, .. }),
        "expected Transport error after 3 attempts, got {error:?}"
    );
}

fn fetcher() -> Fetcher {
    // zero intervals: the throttle's timing is unit-tested on a virtual
    // clock; these tests only assert HTTP behavior and must stay fast
    Fetcher::new(Duration::ZERO, Duration::ZERO)
        .unwrap_or_else(|e| panic!("build fetcher: {e}"))
}
