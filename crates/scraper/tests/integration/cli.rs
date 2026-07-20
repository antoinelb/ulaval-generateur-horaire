use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ulaval_scheduler_core::{Catalogue, CatalogueEntry};
use ulaval_scheduler_scraper::cli;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::catalogue::{
    facet_html, mount_matiere_page, mount_page, page_html,
};

// -- cli::run against a mock server ----------------------------------------

#[tokio::test]
async fn scraping_writes_the_sorted_catalogue_and_no_error_log() {
    let server = MockServer::start().await;
    // unsorted on the page: the output must come back sorted by code
    mount_page(&server, 0, page_html(2, &["GEX-2000", "GEX-1000"])).await;
    let dir = test_dir("run-happy");

    run_catalogue(&dir, &server.uri())
        .await
        .unwrap_or_else(|e| panic!("scrape a clean catalogue: {e}"));

    let expected = serde_json::to_string_pretty(&Catalogue {
        courses: vec![entry("GEX-1000"), entry("GEX-2000")],
    })
    .unwrap_or_else(|e| panic!("serialize expected catalogue: {e}"))
        + "\n";
    assert_eq!(read(&dir.join("catalogue.json")), expected);
    assert!(
        !dir.join("catalogue_errors.log").exists(),
        "a clean scrape must not create an error log"
    );
    cleanup(&dir);
}

#[tokio::test]
async fn a_bare_url_partitions_per_matiere_and_writes_the_merged_catalogue() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        page_html(3, &["GEX-1000"]) + &facet_html(&["7", "113"]),
    )
    .await;
    // GEX-1000 sits in both facets: the artifact must sort and keep it once
    mount_matiere_page(
        &server,
        "7",
        0,
        page_html(3, &["ACT-1000", "ACT-2000", "GEX-1000"]),
    )
    .await;
    mount_matiere_page(&server, "113", 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("run-partitioned");

    run_catalogue(&dir, &server.uri())
        .await
        .unwrap_or_else(|e| panic!("scrape a partitioned catalogue: {e}"));

    let expected = serde_json::to_string_pretty(&Catalogue {
        courses: vec![entry("ACT-1000"), entry("ACT-2000"), entry("GEX-1000")],
    })
    .unwrap_or_else(|e| panic!("serialize expected catalogue: {e}"))
        + "\n";
    assert_eq!(read(&dir.join("catalogue.json")), expected);
    cleanup(&dir);
}

#[tokio::test]
async fn anomalies_are_written_to_the_error_log() {
    let server = MockServer::start().await;
    // one good entry + one entry without a code: 2 results seen in total
    let html = page_html(2, &["GEX-1000"]) + MALFORMED_ENTRY;
    mount_page(&server, 0, html).await;
    let dir = test_dir("run-anomalies");

    run_catalogue(&dir, &server.uri())
        .await
        .unwrap_or_else(|e| {
            panic!("anomalies must not abort the scrape: {e}")
        });

    let log = read(&dir.join("catalogue_errors.log"));
    assert_eq!(log.lines().count(), 1, "one raw line per anomaly: {log}");
    let expected = serde_json::to_string_pretty(&Catalogue {
        courses: vec![entry("GEX-1000")],
    })
    .unwrap_or_else(|e| panic!("serialize expected catalogue: {e}"))
        + "\n";
    assert_eq!(read(&dir.join("catalogue.json")), expected);
    cleanup(&dir);
}

#[tokio::test]
async fn a_clean_scrape_removes_the_stale_error_log() {
    let server = MockServer::start().await;
    mount_page(&server, 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("run-stale-log");
    fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("pre-create the output dir: {e}"));
    fs::write(dir.join("catalogue_errors.log"), "stale anomaly\n")
        .unwrap_or_else(|e| panic!("plant a stale log: {e}"));

    run_catalogue(&dir, &server.uri())
        .await
        .unwrap_or_else(|e| panic!("scrape a clean catalogue: {e}"));

    assert!(
        !dir.join("catalogue_errors.log").exists(),
        "a stale log would keep alarming the cron forever"
    );
    cleanup(&dir);
}

#[tokio::test]
async fn a_failing_scrape_writes_nothing() {
    // no mock mounted: every request 404s
    let server = MockServer::start().await;
    let dir = test_dir("run-scrape-fails");

    let result = run_catalogue(&dir, &server.uri()).await;

    assert!(result.is_err(), "a 404 catalogue must fail");
    assert!(
        !dir.exists(),
        "a failed scrape must not even create the output dir"
    );
}

#[tokio::test]
async fn an_output_dir_that_is_a_file_is_an_error() {
    let server = MockServer::start().await;
    mount_page(&server, 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("run-output-is-file");
    fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("create the parent dir: {e}"));
    let blocked = dir.join("not-a-dir");
    fs::write(&blocked, "in the way")
        .unwrap_or_else(|e| panic!("plant the blocking file: {e}"));

    let result = run_catalogue(&blocked, &server.uri()).await;

    assert!(result.is_err(), "an unusable output dir must fail");
    cleanup(&dir);
}

#[tokio::test]
async fn a_blocked_catalogue_tmp_path_is_an_error() {
    let server = MockServer::start().await;
    mount_page(&server, 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("run-blocked-tmp");
    // a directory where the atomic write puts its temp file
    fs::create_dir_all(dir.join("catalogue.tmp"))
        .unwrap_or_else(|e| panic!("block the tmp path: {e}"));

    let result = run_catalogue(&dir, &server.uri()).await;

    assert!(result.is_err(), "an unwritable catalogue must fail");
    assert!(!dir.join("catalogue.json").exists());
    cleanup(&dir);
}

#[tokio::test]
async fn a_blocked_error_log_write_is_an_error() {
    let server = MockServer::start().await;
    let html = page_html(2, &["GEX-1000"]) + MALFORMED_ENTRY;
    mount_page(&server, 0, html).await;
    let dir = test_dir("run-blocked-log-tmp");
    fs::create_dir_all(dir.join("catalogue_errors.tmp"))
        .unwrap_or_else(|e| panic!("block the log tmp path: {e}"));

    let result = run_catalogue(&dir, &server.uri()).await;

    assert!(result.is_err(), "an unwritable error log must fail");
    assert!(
        dir.join("catalogue.json").exists(),
        "the catalogue itself was written before the log failed"
    );
    cleanup(&dir);
}

#[tokio::test]
async fn an_unremovable_stale_error_log_is_an_error() {
    let server = MockServer::start().await;
    mount_page(&server, 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("run-log-is-dir");
    // a directory at the log path: exists() but remove_file() fails
    fs::create_dir_all(dir.join("catalogue_errors.log"))
        .unwrap_or_else(|e| panic!("block the log path: {e}"));

    let result = run_catalogue(&dir, &server.uri()).await;

    assert!(result.is_err(), "an unremovable stale log must fail");
    cleanup(&dir);
}

// -- programs ---------------------------------------------------------------

#[tokio::test]
async fn a_program_is_written_exactly_as_the_parser_fixture_expects() {
    // The scraper writes a `core::Program` with no envelope, which is what
    // the parser fixtures already are: the frozen page and its expected JSON
    // pin the production artifact, not merely the parser.
    let server = MockServer::start().await;
    mount_fixture(&server, "baccalaureat-en-genie-civil").await;
    let dir = test_dir("run-program");

    run_program(&dir, &[server.uri()])
        .await
        .unwrap_or_else(|e| panic!("scrape a program: {e}"));

    assert_eq!(
        read(&dir.join("programmes/baccalaureat-en-genie-civil.json")),
        read(&program_fixture("baccalaureat-en-genie-civil.json")),
    );
    assert!(
        !dir.join("programmes_errors.log").exists(),
        "génie civil parses without a single anomaly"
    );
    cleanup(&dir);
}

#[tokio::test]
async fn a_run_leaves_the_programs_it_was_not_given_alone() {
    // one file per program, and no sweeping: a run is scoped to the URLs it
    // was handed, so another program's snapshot — and the hand-maintained
    // cheminement type beside it — outlive it (ADR
    // `2026-07-un-fichier-par-programme`)
    let server = MockServer::start().await;
    mount_fixture(&server, "baccalaureat-en-genie-civil").await;
    let dir = test_dir("run-program-scoped");
    let programmes = dir.join("programmes");
    fs::create_dir_all(&programmes)
        .unwrap_or_else(|e| panic!("pre-create the programs dir: {e}"));
    let untouched = [
        "baccalaureat-en-genie-des-eaux.json",
        "baccalaureat-en-genie-des-eaux.manuel.json",
    ];
    for name in untouched {
        fs::write(programmes.join(name), "earlier run\n")
            .unwrap_or_else(|e| panic!("plant {name}: {e}"));
    }

    run_program(&dir, &[server.uri()])
        .await
        .unwrap_or_else(|e| panic!("scrape one program: {e}"));

    assert!(programmes.join("baccalaureat-en-genie-civil.json").exists());
    for name in untouched {
        assert_eq!(
            read(&programmes.join(name)),
            "earlier run\n",
            "{name} was not named by this run and must survive it"
        );
    }
    cleanup(&dir);
}

// -- the compiled binary end to end ----------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn the_binary_scrapes_a_catalogue_end_to_end() {
    // multi_thread: the blocking child process and the mock server must not
    // share the one test thread
    let server = MockServer::start().await;
    mount_page(&server, 0, page_html(1, &["GEX-1000"])).await;
    let dir = test_dir("e2e-happy");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scraper"))
        .args([
            "catalogue",
            "--output-dir",
            &dir.display().to_string(),
            "--url",
            &server.uri(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scraper binary: {e}"));

    assert!(output.status.success(), "{output:?}");
    assert!(dir.join("catalogue.json").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wrote catalogue"), "{stdout}");
    cleanup(&dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_binary_scrapes_a_program_end_to_end() {
    // multi_thread: the blocking child process and the mock server must not
    // share the one test thread
    let server = MockServer::start().await;
    mount_fixture(&server, "baccalaureat-en-genie-civil").await;
    let dir = test_dir("e2e-program");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scraper"))
        .args([
            "program",
            "--output-dir",
            &dir.display().to_string(),
            &server.uri(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scraper binary: {e}"));

    assert!(output.status.success(), "{output:?}");
    assert!(dir
        .join("programmes/baccalaureat-en-genie-civil.json")
        .exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Wrote programs"), "{stdout}");
    cleanup(&dir);
}

#[test]
fn the_binary_rejects_program_without_a_url_with_exit_code_2() {
    // the URL list is the whole work queue: there is nothing to default to
    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scraper"))
        .arg("program")
        .output()
        .unwrap_or_else(|e| panic!("run the scraper binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required"), "{stderr}");
    assert!(stderr.contains("URLS"), "{stderr}");
}

#[test]
fn the_binary_rejects_an_unknown_command_with_exit_code_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scraper"))
        .arg("catalgoue")
        .output()
        .unwrap_or_else(|e| panic!("run the scraper binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unrecognized subcommand"), "{stderr}");
    assert!(
        !stderr.contains("Error:"),
        "the default anyhow prefix must not appear: {stderr}"
    );
}

#[test]
fn the_binary_prints_help_with_exit_code_0() {
    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scraper"))
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("run the scraper binary: {e}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "{stdout}");
}

// -- helpers ----------------------------------------------------------------

// an entry without a code: parsed as one anomaly, counted in total_results
const MALFORMED_ENTRY: &str = concat!(
    r#"<a class="cours-element--lien" href="/etudes/cours/x">"#,
    r#"<span class="cours-element--titre">Sans sigle</span></a>"#,
);

async fn run_catalogue(dir: &Path, url: &str) -> anyhow::Result<()> {
    cli::run(vec![
        "catalogue".to_string(),
        "--output-dir".to_string(),
        dir.display().to_string(),
        "--url".to_string(),
        url.to_string(),
    ])
    .await
}

async fn run_program(dir: &Path, urls: &[String]) -> anyhow::Result<()> {
    let mut args = vec![
        "program".to_string(),
        "--output-dir".to_string(),
        dir.display().to_string(),
    ];
    args.extend(urls.iter().cloned());
    cli::run(args).await
}

// serves the frozen page of a parser fixture, so the whole chain runs on
// real ULaval markup rather than on a builder's idea of it
async fn mount_fixture(server: &MockServer, name: &str) {
    let html = read(&program_fixture(&format!("{name}.html")));
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(server)
        .await;
}

fn program_fixture(name: &str) -> PathBuf {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/test_cases/programs",
    ))
    .join(name)
}

fn entry(code: &str) -> CatalogueEntry {
    CatalogueEntry {
        code: code.to_string(),
        title: format!("Cours {code}"),
        // the parser absolutizes relative hrefs (ADR urls-absolues)
        url: format!("https://www.ulaval.ca/etudes/cours/{code}"),
    }
}

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ulaval-scraper-{name}"));
    // leftovers from an earlier failed run
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn cleanup(dir: &PathBuf) {
    fs::remove_dir_all(dir)
        .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
}
