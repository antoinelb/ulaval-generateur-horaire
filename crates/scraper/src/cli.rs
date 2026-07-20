use std::collections::BTreeMap;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

use crate::course::{self, CourseError, SessionSnapshot};
use crate::program::{self, ProgramError};
use crate::{catalogue, fetch::Fetcher, parser::ParseError, print};
use ulaval_scheduler_core::{Catalogue, CatalogueEntry, Program};

// ~10 requests/second, the politeness budget the whole scraper shares
// (ADR `2026-07-conception-du-fetcher`)
const min_interval: Duration = Duration::from_millis(100);
const backoff: Duration = Duration::from_secs(1);

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
    Courses {
        #[arg(long, default_value = "data")]
        output_dir: String,
        // narrowing filter only: no subject means the whole catalogue.
        // `num_args`/`value_delimiter` accept both `--subjects gex gci` and
        // `--subjects "gex gci"`
        #[arg(long, num_args = 1.., value_delimiter = ' ')]
        subjects: Vec<String>,
    },
    Program {
        #[arg(long, default_value = "data")]
        output_dir: String,
        // Required, unlike every other work queue in this binary: a program
        // page URL is a slug no course code can rebuild, and only the
        // programs whose rules are wanted need their page scraped at all —
        // so the list is the caller's to give.
        #[arg(required = true, num_args = 1..)]
        urls: Vec<String>,
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
        Command::Courses {
            output_dir,
            subjects,
        } => {
            let (sessions, anomalies) =
                get_courses(&output_dir, &subjects).await?;
            write_courses(sessions, anomalies, &output_dir, &subjects)
        }
        Command::Program { output_dir, urls } => {
            let (programs, anomalies) = get_programs(&urls).await;
            write_programs(programs, anomalies, &output_dir)
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
    let fetcher = Fetcher::new(min_interval, backoff)
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
    // expect over `?`: serializing strings and vecs provably cannot fail
    let json = serde_json::to_string_pretty(&catalogue)
        .expect("Catalogue serialization always succeeds");
    write_atomic(&path, &(json + "\n"))?;
    write_error_log(&dir.join("catalogue_errors.log"), &anomalies)?;

    task.done();
    Ok(())
}

async fn get_courses(
    output_dir: &str,
    subjects: &[String],
) -> anyhow::Result<(BTreeMap<String, SessionSnapshot>, Vec<CourseError>)> {
    let dir = Path::new(output_dir);
    let entries = filter_by_subject(read_catalogue(dir)?.courses, subjects)?;

    // created up front so an unusable path fails now rather than once per
    // course, minutes into the run
    let cache_dir = dir.join("cache").join("cours");
    std::fs::create_dir_all(&cache_dir)?;

    let task = print::task(
        &format!("Scraping {} courses...", entries.len()),
        &format!("Scraped {} courses.", entries.len()),
    );
    // expect over `?`: this static config provably builds (the failure path
    // needs an injected bad builder — seam-tested in fetch.rs)
    let fetcher = Fetcher::new(min_interval, backoff)
        .expect("static fetcher config always builds");
    let (courses, anomalies) =
        course::scrape(&fetcher, &entries, &cache_dir).await;
    task.done();

    Ok((course::group_by_session(courses), anomalies))
}

// the catalogue is the work queue, written by an earlier `catalogue` run:
// course URLs are slugs that cannot be derived from a code
// (ADR `2026-07-catalogue-artefact-commite`)
fn read_catalogue(dir: &Path) -> anyhow::Result<Catalogue> {
    let path = dir.join("catalogue.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| {
        anyhow::anyhow!(
            "Reading {}: {source}\nRun `ulaval-scraper catalogue` first.",
            path.display()
        )
    })?;
    Ok(serde_json::from_str(&raw)?)
}

fn filter_by_subject(
    entries: Vec<CatalogueEntry>,
    subjects: &[String],
) -> anyhow::Result<Vec<CatalogueEntry>> {
    if subjects.is_empty() {
        return Ok(entries);
    }
    let wanted: Vec<String> =
        subjects.iter().map(|s| s.to_uppercase()).collect();

    // a subject nobody offers is a typo, and scraping nothing at all is a
    // worse answer than saying so
    let unknown: Vec<&str> = wanted
        .iter()
        .filter(|subject| {
            !entries
                .iter()
                .any(|entry| subject_of(&entry.code) == Some(subject.as_str()))
        })
        .map(String::as_str)
        .collect();
    anyhow::ensure!(
        unknown.is_empty(),
        "No course in the catalogue for subject(s): {}",
        unknown.join(", ")
    );

    Ok(entries
        .into_iter()
        .filter(|entry| {
            subject_of(&entry.code).is_some_and(|subject| {
                wanted.iter().any(|wanted| wanted == subject)
            })
        })
        .collect())
}

// « matière » = the course-code prefix, so filtering needs no facet
fn subject_of(code: &str) -> Option<&str> {
    code.split_once('-').map(|(subject, _)| subject)
}

fn write_courses(
    sessions: BTreeMap<String, SessionSnapshot>,
    anomalies: Vec<CourseError>,
    output_dir: &str,
    subjects: &[String],
) -> anyhow::Result<()> {
    let dir = Path::new(output_dir);
    let sessions_dir = dir.join("cours");
    let task = print::task(
        &format!("Writing courses to {}...", sessions_dir.display()),
        &format!("Wrote courses in {}.", sessions_dir.display()),
    );

    // only a full run has seen the whole catalogue, so only a full run may
    // remove what it did not produce: a `--subjects` run knows nothing of
    // the other subjects' sessions and must leave their files alone.
    // Listed before writing, deleted after, so nothing is lost if the run
    // dies midway.
    let stale = if subjects.is_empty() {
        stale_sessions(&sessions_dir, &sessions)
    } else {
        Vec::new()
    };
    std::fs::create_dir_all(&sessions_dir)?;

    for (session, snapshot) in sessions {
        // expect over `?`: serializing strings, maps and vecs provably
        // cannot fail
        let json = serde_json::to_string_pretty(&snapshot)
            .expect("SessionSnapshot serialization always succeeds");
        let path = sessions_dir.join(format!("{session}.json"));
        write_atomic(&path, &(json + "\n"))?;
    }
    for path in stale {
        std::fs::remove_file(path)?;
    }
    write_error_log(&dir.join("cours_errors.log"), &anomalies)?;

    task.done();
    Ok(())
}

// A course moves session when its offering changes — GCI-7077 sat in
// `a2020.json` only because its Automne 2026 block was unreadable — so a
// snapshot the run no longer produces is stale, and leaving it behind
// would advertise a session the course is not offered in.
fn stale_sessions(
    dir: &Path,
    produced: &BTreeMap<String, SessionSnapshot>,
) -> Vec<PathBuf> {
    // a missing directory (first run) simply holds nothing stale
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // `{session}.manuel.json` is hand-maintained and never touched
            // by the scraper (ADR `2026-07-contribution-de-cours-manuels`)
            !name.ends_with(".manuel.json")
                && name
                    .strip_suffix(".json")
                    .is_some_and(|session| !produced.contains_key(session))
        })
        .map(|entry| entry.path())
        .collect()
}

// no `Result`, unlike its two siblings: there is no work queue to read and
// no cache directory to create, so nothing can fail before the first request
async fn get_programs(urls: &[String]) -> (Vec<Program>, Vec<ProgramError>) {
    // expect over `?`: this static config provably builds (the failure path
    // needs an injected bad builder — seam-tested in fetch.rs)
    let fetcher = Fetcher::new(min_interval, backoff)
        .expect("static fetcher config always builds");
    program::scrape(&fetcher, urls).await
}

// One file per program rather than one snapshot holding them all: a run is
// restricted to the URLs it was handed, so it writes exactly those and
// leaves every other program's file — including the hand-maintained
// `{code}.manuel.json` — alone (ADR `2026-07-un-fichier-par-programme`).
fn write_programs(
    programs: Vec<Program>,
    anomalies: Vec<ProgramError>,
    output_dir: &str,
) -> anyhow::Result<()> {
    let dir = Path::new(output_dir);
    let programs_dir = dir.join("programmes");
    let task = print::task(
        &format!("Writing programs to {}...", programs_dir.display()),
        &format!("Wrote programs in {}.", programs_dir.display()),
    );
    std::fs::create_dir_all(&programs_dir)?;

    for program in programs {
        // expect over `?`: serializing strings, vecs and options provably
        // cannot fail
        let json = serde_json::to_string_pretty(&program)
            .expect("Program serialization always succeeds");
        let path = programs_dir.join(format!("{}.json", program.code));
        write_atomic(&path, &(json + "\n"))?;
    }
    write_error_log(&dir.join("programmes_errors.log"), &anomalies)?;

    task.done();
    Ok(())
}

fn write_error_log(
    path: &Path,
    anomalies: &[impl std::fmt::Display],
) -> anyhow::Result<()> {
    let error_log: String = anomalies
        .iter()
        .map(|anomaly| format!("{anomaly}\n"))
        .collect();
    if error_log.is_empty() {
        // a log left over from an earlier run would misreport a clean one
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    } else {
        write_atomic(path, &error_log)?;
        print::warn_print(&format!(
            "There were {} anomalies. See {}",
            anomalies.len(),
            path.display()
        ));
    }
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

    #[tokio::test]
    async fn scraped_courses_are_written_one_file_per_session() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-happy");
        plant_catalogue(&dir, &server, &["GEX-1000"]);

        run(courses_args(&dir, &["gex"]))
            .await
            .unwrap_or_else(|e| panic!("scrape one course: {e}"));

        let snapshot = std::fs::read_to_string(dir.join("cours/a2026.json"))
            .unwrap_or_else(|e| panic!("read the session snapshot: {e}"));
        assert!(snapshot.contains("GEX-1000"), "{snapshot}");
        // declaration order, not alphabetical: these snapshots are
        // committed and the diffs have to stay readable
        assert!(
            snapshot.find("\"code\"") < snapshot.find("\"title\""),
            "{snapshot}"
        );
        assert!(!dir.join("cours_errors.log").exists(), "clean run");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_lowercase_subject_selects_the_same_courses_as_uppercase() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        // GCI-1000 is in the catalogue but out of the requested subject, so
        // no request is ever made for it
        let dir = test_dir("courses-subject-case");
        plant_catalogue(&dir, &server, &["GEX-1000", "GCI-1000"]);

        for subject in ["gex", "GEX", "Gex"] {
            run(courses_args(&dir, &[subject]))
                .await
                .unwrap_or_else(|e| panic!("scrape for {subject}: {e}"));
        }

        cleanup(&dir);
    }

    #[tokio::test]
    async fn no_subject_scrapes_the_whole_catalogue() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        mount_course(&server, "GCI-1000").await;
        let dir = test_dir("courses-no-subject");
        plant_catalogue(&dir, &server, &["GEX-1000", "GCI-1000"]);

        run(courses_args(&dir, &[]))
            .await
            .unwrap_or_else(|e| panic!("scrape every course: {e}"));

        let snapshot = std::fs::read_to_string(dir.join("cours/a2026.json"))
            .unwrap_or_else(|e| panic!("read the session snapshot: {e}"));
        assert!(snapshot.contains("GEX-1000"), "{snapshot}");
        assert!(snapshot.contains("GCI-1000"), "{snapshot}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_subject_no_course_belongs_to_is_an_error_naming_it() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        let dir = test_dir("courses-unknown-subject");
        plant_catalogue(&dir, &server, &["GEX-1000"]);

        let error = run(courses_args(&dir, &["gxe", "gex"]))
            .await
            .expect_err("a typoed subject must fail");

        let message = error.to_string();
        assert!(message.contains("GXE"), "{message}");
        assert!(!message.contains("GEX"), "the valid one is not: {message}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_missing_catalogue_says_to_scrape_it_first() {
        let _guard = lock_print();
        let dir = test_dir("courses-no-catalogue");

        let error = run(courses_args(&dir, &[]))
            .await
            .expect_err("courses without a catalogue must fail");

        let message = error.to_string();
        assert!(message.contains("catalogue.json"), "{message}");
        assert!(message.contains("ulaval-scraper catalogue"), "{message}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unreadable_catalogue_is_an_error() {
        let _guard = lock_print();
        let dir = test_dir("courses-bad-catalogue");
        std::fs::write(dir.join("catalogue.json"), "{ truncated")
            .unwrap_or_else(|e| panic!("plant a corrupt catalogue: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "a corrupt catalogue must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unusable_cache_dir_fails_before_any_request() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        let dir = test_dir("courses-blocked-cache");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a file where the cache directory must go
        std::fs::write(dir.join("cache"), "in the way")
            .unwrap_or_else(|e| panic!("block the cache dir: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unusable cache dir must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unusable_sessions_dir_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-blocked-sessions");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a file where the session snapshots must go
        std::fs::write(dir.join("cours"), "in the way")
            .unwrap_or_else(|e| panic!("block the sessions dir: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unusable sessions dir must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_full_run_removes_snapshots_it_no_longer_produces() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-sweep-full");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        let cours = dir.join("cours");
        std::fs::create_dir_all(&cours)
            .unwrap_or_else(|e| panic!("create cours dir: {e}"));
        for name in ["a2020.json", "a2026.manuel.json", "notes.txt"] {
            std::fs::write(cours.join(name), "leftover")
                .unwrap_or_else(|e| panic!("plant {name}: {e}"));
        }

        run(courses_args(&dir, &[]))
            .await
            .unwrap_or_else(|e| panic!("full scrape: {e}"));

        assert!(cours.join("a2026.json").exists(), "the run's own output");
        assert!(
            !cours.join("a2020.json").exists(),
            "a session the run no longer produces is stale"
        );
        // ADR `2026-07-contribution-de-cours-manuels`: hand-maintained,
        // never touched by the scraper
        assert!(cours.join("a2026.manuel.json").exists());
        assert!(cours.join("notes.txt").exists(), "not a snapshot");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_scoped_run_leaves_other_snapshots_alone() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-sweep-scoped");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        let cours = dir.join("cours");
        std::fs::create_dir_all(&cours)
            .unwrap_or_else(|e| panic!("create cours dir: {e}"));
        // another subject's session, invisible to a --subjects gex run
        std::fs::write(cours.join("h2024.json"), "other subjects")
            .unwrap_or_else(|e| panic!("plant h2024: {e}"));

        run(courses_args(&dir, &["gex"]))
            .await
            .unwrap_or_else(|e| panic!("scoped scrape: {e}"));

        assert!(
            cours.join("h2024.json").exists(),
            "a scoped run has not seen the other subjects' courses, so it \
             must not judge their snapshots stale"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unremovable_stale_snapshot_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-sweep-blocked");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a directory named like a stale snapshot: remove_file must fail
        std::fs::create_dir_all(dir.join("cours").join("a2020.json"))
            .unwrap_or_else(|e| panic!("plant the blocked path: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unremovable stale file must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_session_snapshot_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-blocked-snapshot");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a directory at the target path makes the rename fail
        std::fs::create_dir_all(dir.join("cours").join("a2026.json"))
            .unwrap_or_else(|e| panic!("block the snapshot path: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unwritable snapshot must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_error_log_is_an_error() {
        let _guard = lock_print();
        // nothing mounted: the only course 404s, so there is something to
        // log — and the path it must be logged to is blocked
        let server = MockServer::start().await;
        let dir = test_dir("courses-blocked-log");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        std::fs::create_dir_all(dir.join("cours_errors.log"))
            .unwrap_or_else(|e| panic!("block the log path: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unwritable error log must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn course_anomalies_are_logged_and_the_log_is_cleared_when_clean() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        let dir = test_dir("courses-error-log");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        let log = dir.join("cours_errors.log");

        // nothing mounted yet: the only course 404s
        run(courses_args(&dir, &[]))
            .await
            .unwrap_or_else(|e| panic!("a 404 must not fail the run: {e}"));
        let logged = std::fs::read_to_string(&log)
            .unwrap_or_else(|e| panic!("read the error log: {e}"));
        assert!(logged.contains("gex-1000"), "{logged}");

        // the page comes back: the stale log must not outlive it
        mount_course(&server, "GEX-1000").await;
        run(courses_args(&dir, &[]))
            .await
            .unwrap_or_else(|e| panic!("second run: {e}"));
        assert!(!log.exists(), "a clean run clears the previous log");

        cleanup(&dir);
    }

    #[test]
    fn a_code_with_no_subject_prefix_belongs_to_no_subject() {
        // the catalogue is a file on disk, so a code that does not split
        // into subject and number is possible input: it must be filtered
        // out, and must not make a requested subject look unknown
        let entries =
            vec![catalogue_entry("GEX-1000"), catalogue_entry("NOHYPHEN")];

        let filtered = filter_by_subject(entries, &["gex".to_string()])
            .unwrap_or_else(|e| panic!("filter by subject: {e}"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].code, "GEX-1000");
    }

    fn catalogue_entry(code: &str) -> CatalogueEntry {
        CatalogueEntry {
            code: code.to_string(),
            title: format!("Cours {code}"),
            url: format!("https://ulaval.ca/etudes/cours/{code}"),
        }
    }

    #[tokio::test]
    async fn courses_help_prints_help_and_succeeds() {
        for flag in ["--help", "-h"] {
            let result =
                run(vec!["courses".to_string(), flag.to_string()]).await;

            assert!(result.is_ok(), "courses {flag} is a help request");
        }
    }

    fn courses_args(dir: &Path, subjects: &[&str]) -> Vec<String> {
        let mut args = vec![
            "courses".to_string(),
            "--output-dir".to_string(),
            dir.display().to_string(),
        ];
        if !subjects.is_empty() {
            args.push("--subjects".to_string());
            args.extend(subjects.iter().map(|s| s.to_string()));
        }
        args
    }

    fn plant_catalogue(dir: &Path, server: &MockServer, codes: &[&str]) {
        let entries: Vec<CatalogueEntry> = codes
            .iter()
            .map(|code| CatalogueEntry {
                code: code.to_string(),
                title: format!("Cours {code}"),
                url: format!("{}/{}", server.uri(), code.to_lowercase()),
            })
            .collect();
        let json = serde_json::to_string(&Catalogue { courses: entries })
            .unwrap_or_else(|e| panic!("serialize the catalogue: {e}"));
        std::fs::write(dir.join("catalogue.json"), json)
            .unwrap_or_else(|e| panic!("plant the catalogue: {e}"));
    }

    async fn mount_course(server: &MockServer, code: &str) {
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(format!(
                "/{}",
                code.to_lowercase()
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(crate::course::tests::course_html(code)),
            )
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn scraped_programs_are_written_one_file_each() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil").await;
        mount_program(&server, "genie-des-eaux").await;
        let dir = test_dir("programs-happy");

        run(programs_args(
            &dir,
            &[
                &program_url(&server, "genie-civil"),
                &program_url(&server, "genie-des-eaux"),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("scrape two programs: {e}"));

        let programmes = dir.join("programmes");
        let civil =
            std::fs::read_to_string(programmes.join("genie-civil.json"))
                .unwrap_or_else(|e| panic!("read the program file: {e}"));
        assert!(civil.contains("genie-civil"), "{civil}");
        assert!(programmes.join("genie-des-eaux.json").exists());
        // declaration order, not alphabetical: these files are committed and
        // the diffs have to stay readable
        assert!(civil.find("\"code\"") < civil.find("\"title\""), "{civil}");
        assert!(!dir.join("programmes_errors.log").exists(), "clean run");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn program_without_any_url_is_a_usage_error() {
        // there is no work queue to fall back on, so an empty list can only
        // mean the caller forgot — never « scrape everything »
        let error = run(vec!["program".to_string()])
            .await
            .expect_err("the URL list is mandatory");

        let message = error.to_string();
        assert!(message.contains("URLS"), "{message}");
        assert!(message.contains("required"), "{message}");
    }

    #[tokio::test]
    async fn program_help_prints_help_and_succeeds() {
        for flag in ["--help", "-h"] {
            let result =
                run(vec!["program".to_string(), flag.to_string()]).await;

            assert!(result.is_ok(), "program {flag} is a help request");
        }
    }

    #[tokio::test]
    async fn a_failing_url_is_logged_and_the_reachable_programs_still_land() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil").await;
        // nothing mounted for the second URL, so it 404s
        let dir = test_dir("programs-error-log");

        run(programs_args(
            &dir,
            &[
                &program_url(&server, "genie-civil"),
                &program_url(&server, "genie-absent"),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("a 404 must not fail the run: {e}"));

        assert!(
            dir.join("programmes").join("genie-civil.json").exists(),
            "the reachable program still lands"
        );
        let logged =
            std::fs::read_to_string(dir.join("programmes_errors.log"))
                .unwrap_or_else(|e| panic!("read the error log: {e}"));
        assert!(logged.contains("genie-absent"), "{logged}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unusable_programs_dir_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil").await;
        let dir = test_dir("programs-blocked-dir");
        // a file where the program files must go
        std::fs::write(dir.join("programmes"), "in the way")
            .unwrap_or_else(|e| panic!("block the programs dir: {e}"));

        let result =
            run(programs_args(&dir, &[&program_url(&server, "genie-civil")]))
                .await;

        assert!(result.is_err(), "an unusable programs dir must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_program_file_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil").await;
        let dir = test_dir("programs-blocked-file");
        // a directory at the target path makes the rename fail
        std::fs::create_dir_all(
            dir.join("programmes").join("genie-civil.json"),
        )
        .unwrap_or_else(|e| panic!("block the program path: {e}"));

        let result =
            run(programs_args(&dir, &[&program_url(&server, "genie-civil")]))
                .await;

        assert!(result.is_err(), "an unwritable program file must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_program_error_log_is_an_error() {
        let _guard = lock_print();
        // nothing mounted: the only URL 404s, so there is something to log —
        // and the path it must be logged to is blocked
        let server = MockServer::start().await;
        let dir = test_dir("programs-blocked-log");
        std::fs::create_dir_all(dir.join("programmes_errors.log"))
            .unwrap_or_else(|e| panic!("block the log path: {e}"));

        let result =
            run(programs_args(&dir, &[&program_url(&server, "absent")])).await;

        assert!(result.is_err(), "an unwritable error log must fail");
        cleanup(&dir);
    }

    fn programs_args(dir: &Path, urls: &[&str]) -> Vec<String> {
        let mut args = vec![
            "program".to_string(),
            "--output-dir".to_string(),
            dir.display().to_string(),
        ];
        args.extend(urls.iter().map(|url| url.to_string()));
        args
    }

    fn program_url(server: &MockServer, slug: &str) -> String {
        format!("{}/{slug}", server.uri())
    }

    async fn mount_program(server: &MockServer, slug: &str) {
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(format!("/{slug}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    crate::program::tests::program_html(slug),
                ),
            )
            .mount(server)
            .await;
    }

    fn lock_print() -> std::sync::MutexGuard<'static, ()> {
        print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
