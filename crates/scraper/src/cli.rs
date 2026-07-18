use std::{path::Path, time::Duration};

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

use crate::{catalogue, fetch::Fetcher, parser::ParseError, print};
use ulaval_scheduler_core::Catalogue;

const cli_styles: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default())
    .usage(AnsiColor::Green.on_default())
    .literal(AnsiColor::Blue.on_default())
    .placeholder(AnsiColor::Blue.on_default())
    .error(AnsiColor::Red.on_default())
    .invalid(AnsiColor::Yellow.on_default())
    .valid(AnsiColor::Green.on_default());

#[derive(Parser)]
#[command(
    name = "ulaval-scraper",
    styles = cli_styles,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Catalogue {
        #[arg(long, default_value = "data")]
        output_dir: String,
        #[arg(long, default_value = "https://www.ulaval.ca/etudes/cours")]
        url: String,
    },
}

pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    let argv = std::iter::once("ulaval-scraper".to_string()).chain(args);
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        // help and version are successful outcomes, not errors
        Err(error) if error.exit_code() == 0 => {
            // display is non-critical: a broken pipe must never kill the run
            error.print().ok();
            return Ok(());
        }
        // usage errors carry clap's rendered message through the anyhow
        // frontier: main prints it to stderr and exits 2
        Err(error) => {
            anyhow::bail!("{}", error.render().ansi().to_string().trim_end())
        }
    };
    match cli.command {
        Command::Catalogue { output_dir, url } => {
            let (catalogue, anomalies) = get_catalogue(&url).await?;
            write_catalogue(catalogue, anomalies, &output_dir)
        }
    }
}

async fn get_catalogue(
    url: &str,
) -> anyhow::Result<(Catalogue, Vec<ParseError>)> {
    let task = print::task(
        &format!("Scraping catalogue from {url}..."),
        &format!("Scraped catalogue from {url}."),
    );
    // expect over `?`: this static config provably builds (the failure path
    // needs an injected bad builder — seam-tested in fetch.rs)
    let fetcher =
        Fetcher::new(Duration::from_millis(100), Duration::from_secs(1))
            .expect("static fetcher config always builds");
    let page = catalogue::scrape(&fetcher, url).await?;
    let catalogue = Catalogue::from_entries(page.entries);
    task.done();
    Ok((catalogue, page.anomalies))
}

fn write_catalogue(
    catalogue: Catalogue,
    anomalies: Vec<ParseError>,
    output_dir: &str,
) -> anyhow::Result<()> {
    let task = print::task(
        &format!("Writing catalogue to {output_dir}..."),
        &format!("Wrote catalogue in {output_dir}."),
    );
    let dir = Path::new(output_dir);
    std::fs::create_dir_all(dir)?;
    let path = dir.join("catalogue.json");
    let error_path = dir.join("catalogue_errors.log");
    // expect over `?`: serializing strings and vecs provably cannot fail
    let json = serde_json::to_string_pretty(&catalogue)
        .expect("Catalogue serialization always succeeds");
    write_atomic(&path, &(json + "\n"))?;
    let error_log: String = anomalies
        .iter()
        .map(|anomaly| format!("{anomaly}\n"))
        .collect();
    if error_log.is_empty() {
        if error_path.exists() {
            std::fs::remove_file(error_path)?;
        }
    } else {
        write_atomic(&error_path, &error_log)?;
        print::warn_print(&format!(
            "There were {} anomalies. See {}",
            anomalies.len(),
            error_path.display()
        ));
    }

    task.done();
    Ok(())
}

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    // TEST_STATE_LOCK serializes whole tests around the global print state,
    // so holding it across await points is the intent, not an oversight:
    // each test owns its thread, the holder keeps making progress, and
    // waiters block without any lock-ordering cycle
    #![allow(clippy::await_holding_lock)]

    use std::path::PathBuf;

    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn a_scraped_catalogue_is_written_to_the_output_dir() {
        let _guard = print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(PAGE_HTML),
            )
            .mount(&server)
            .await;
        let dir = test_dir("scrape-happy");

        run(catalogue_args(&dir.display().to_string(), &server.uri()))
            .await
            .unwrap_or_else(|e| panic!("scrape one page: {e}"));

        assert!(dir.join("catalogue.json").exists());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_failing_scrape_is_an_error() {
        let _guard = print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // no mock mounted: every request 404s
        let server = MockServer::start().await;
        let dir = test_dir("scrape-fails");

        let result =
            run(catalogue_args(&dir.display().to_string(), &server.uri()))
                .await;

        assert!(result.is_err(), "a 404 catalogue must fail");
        assert!(!dir.join("catalogue.json").exists());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_output_dir_is_an_error() {
        let _guard = print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(PAGE_HTML),
            )
            .mount(&server)
            .await;
        let dir = test_dir("output-is-file");
        let blocked = dir.join("blocked");
        std::fs::write(&blocked, "in the way")
            .unwrap_or_else(|e| panic!("plant the blocking file: {e}"));

        let result = run(catalogue_args(
            &blocked.display().to_string(),
            &server.uri(),
        ))
        .await;

        assert!(result.is_err(), "an unusable output dir must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn no_args_is_a_usage_error_showing_help() {
        // clap/uv convention: a missing subcommand is an error (exit 2)
        // whose message is the full help
        let error = run(Vec::new())
            .await
            .expect_err("bare invocation must fail");

        assert!(error.to_string().contains("Usage:"), "{error}");
    }

    #[tokio::test]
    async fn help_flags_print_help_and_succeed() {
        for flag in ["--help", "-h"] {
            let result = run(vec![flag.to_string()]).await;

            assert!(result.is_ok(), "{flag} is a help request");
        }
    }

    #[tokio::test]
    async fn unknown_command_is_an_error_naming_the_command() {
        let error = run(vec!["catalgoue".to_string()])
            .await
            .expect_err("a typoed command must fail");

        let message = error.to_string();
        assert!(message.contains("unrecognized subcommand"), "{message}");
        assert!(message.contains("catalgoue"), "{message}");
    }

    #[tokio::test]
    async fn catalogue_help_prints_help_and_succeeds() {
        for flag in ["--help", "-h"] {
            let result =
                run(vec!["catalogue".to_string(), flag.to_string()]).await;

            assert!(result.is_ok(), "catalogue {flag} is a help request");
        }
    }

    #[tokio::test]
    async fn catalogue_with_a_stray_argument_is_an_error() {
        let args: Vec<String> = ["catalogue", "stray"]
            .iter()
            .map(|arg| arg.to_string())
            .collect();

        let error = run(args).await.expect_err("a stray argument must fail");

        let message = error.to_string();
        assert!(message.contains("unexpected argument"), "{message}");
        assert!(message.contains("stray"), "{message}");
    }

    fn catalogue_args(output_dir: &str, url: &str) -> Vec<String> {
        ["catalogue", "--output-dir", output_dir, "--url", url]
            .iter()
            .map(|arg| arg.to_string())
            .collect()
    }

    #[test]
    fn atomic_write_creates_then_replaces_and_leaves_no_tmp() {
        let dir = test_dir("atomic-write-replaces");
        let path = dir.join("file.json");

        write_atomic(&path, "first")
            .unwrap_or_else(|e| panic!("first write: {e}"));
        write_atomic(&path, "second")
            .unwrap_or_else(|e| panic!("replacing write: {e}"));

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read back: {e}"));
        assert_eq!(content, "second");
        assert!(
            !dir.join("file.tmp").exists(),
            "the temp file must be renamed away"
        );
        cleanup(&dir);
    }

    #[test]
    fn atomic_write_with_a_blocked_tmp_path_is_an_error() {
        let dir = test_dir("atomic-write-blocked-tmp");
        // a directory where the temp file must go makes fs::write fail
        std::fs::create_dir_all(dir.join("file.tmp"))
            .unwrap_or_else(|e| panic!("block the tmp path: {e}"));

        let result = write_atomic(&dir.join("file.json"), "content");

        assert!(result.is_err(), "writing over a directory must fail");
        cleanup(&dir);
    }

    #[test]
    fn atomic_write_onto_a_directory_target_is_an_error() {
        let dir = test_dir("atomic-write-dir-target");
        // a directory at the target path makes the rename fail
        std::fs::create_dir_all(dir.join("file.json"))
            .unwrap_or_else(|e| panic!("block the target path: {e}"));

        let result = write_atomic(&dir.join("file.json"), "content");

        assert!(result.is_err(), "renaming onto a directory must fail");
        cleanup(&dir);
    }

    const PAGE_HTML: &str = concat!(
        r#"<div class="total-resultats"><p>1 résultats</p></div>"#,
        r#"<a class="cours-element--lien" href="/etudes/cours/gex-1000">"#,
        r#"<span class="cours-element--sigle">GEX-1000</span>"#,
        r#"<span class="cours-element--titre">Cours GEX-1000</span></a>"#,
    );

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ulaval-scraper-cli-{name}"));
        // leftovers from an earlier failed run
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        dir
    }

    fn cleanup(dir: &PathBuf) {
        std::fs::remove_dir_all(dir)
            .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
    }
}
