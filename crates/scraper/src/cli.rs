use std::{path::Path, time::Duration};

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};

use crate::course::{self, CourseError, Snapshot};
use crate::program::{self, ProgramError};
use crate::{catalogue, fetch::Fetcher, parser::ParseError, print};
use ulaval_scheduler_core::{
    preparatory_rule, Catalogue, CatalogueEntry, Course, Program, Season,
    Semester,
};

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
        // the vintage the run captures (e.g. `A26`) — defaults to the
        // scrape date's rule (`semester_after`); an explicit value pins
        // re-runs and byte-exact tests to a frozen vintage
        #[arg(long)]
        semester: Option<Semester>,
        // where slugs resolve to pages; overridable so a refresh can be
        // tested against a mock server
        #[arg(
            long,
            default_value = "https://www.ulaval.ca/etudes/programmes"
        )]
        base_url: String,
        // A program page URL is a slug no course code can rebuild — but a
        // snapshot's `slug` field carries it, so an empty list means
        // « refresh every program already in the directory » (ADR
        // `2026-08-code-officiel-de-programme-et-slug`).
        #[arg(num_args = 1..)]
        urls: Vec<String>,
    },
}

// the one field the refresh needs from a snapshot; serde ignores the rest
#[derive(serde::Deserialize)]
struct ProgramSlug {
    slug: String,
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
            let (snapshot, anomalies) =
                get_courses(&output_dir, &subjects).await?;
            write_courses(
                snapshot,
                anomalies,
                &output_dir,
                &subjects,
                std::time::SystemTime::now(),
            )
        }
        Command::Program {
            output_dir,
            semester,
            base_url,
            urls,
        } => {
            let semester = semester.unwrap_or_else(|| {
                semester_after(std::time::SystemTime::now())
            });
            // the courses feed the « Scolarité préparatoire » rule: read
            // before anything is fetched, so a missing snapshot fails the
            // run immediately (ADR `2026-08-regle-scolarite-preparatoire`)
            let snapshot = read_snapshot(Path::new(&output_dir))?;
            let urls = if urls.is_empty() {
                refresh_urls(&output_dir, &base_url)?
            } else {
                urls
            };
            let (programs, anomalies) = get_programs(&urls, semester).await;
            let programs = add_preparatory_rules(programs, &snapshot.courses)?;
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
) -> anyhow::Result<(Snapshot, Vec<CourseError>)> {
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
    let (courses, anomalies, tally) =
        course::scrape(&fetcher, &entries, &cache_dir).await;
    // the split, not just the total: a cache the parser can no longer read
    // is silently a cold run, and only this line tells the two apart
    task.done_with(&format!(
        "Scraped {} courses ({} cached, {} fetched).",
        entries.len(),
        tally.cached,
        tally.fetched,
    ));

    Ok((course::snapshot(courses), anomalies))
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

// the course snapshot feeds the préuniversitaire walk of every scraped
// program, so `program` needs it up front — same fail-fast contract as
// `read_catalogue` (ADR `2026-08-regle-scolarite-preparatoire`)
fn read_snapshot(dir: &Path) -> anyhow::Result<Snapshot> {
    let path = dir.join("cours.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| {
        anyhow::anyhow!(
            "Reading {}: {source}\nRun `ulaval-scraper courses` first.",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|source| {
        anyhow::anyhow!("Parsing {}: {source}", path.display())
    })
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
        .filter(|entry| has_wanted_subject(&entry.code, &wanted))
        .collect())
}

// `wanted` holds upper-case subjects, as course codes do
fn has_wanted_subject(code: &str, wanted: &[String]) -> bool {
    subject_of(code)
        .is_some_and(|subject| wanted.iter().any(|wanted| wanted == subject))
}

// « matière » = the course-code prefix, so filtering needs no facet
fn subject_of(code: &str) -> Option<&str> {
    code.split_once('-').map(|(subject, _)| subject)
}

fn write_courses(
    snapshot: Snapshot,
    anomalies: Vec<CourseError>,
    output_dir: &str,
    subjects: &[String],
    now: std::time::SystemTime,
) -> anyhow::Result<()> {
    let dir = Path::new(output_dir);
    let path = dir.join("cours.json");
    let task = print::task(
        &format!("Writing courses to {}...", path.display()),
        &format!("Wrote courses in {}.", path.display()),
    );

    // only a full run has seen the whole catalogue, so only a full run may
    // replace the snapshot outright — the atomic rename leaves nothing
    // stale. A `--subjects` run knows nothing of the other subjects'
    // courses, so it merges its own into what is already on disk instead
    // of overwriting. `data/cours.manuel.json` is untouched either way: the
    // scraper only ever writes `cours.json`.
    let snapshot = if subjects.is_empty() {
        snapshot
    } else {
        merge_into_existing(&path, snapshot, subjects)?
    };

    // no create_dir_all: the catalogue was read from `dir` earlier in this
    // same run, so it exists; a vanished directory fails the atomic write
    // below with a path-carrying error.
    // expect over `?`: serializing strings, maps and vecs provably
    // cannot fail
    let json = serde_json::to_string_pretty(&snapshot)
        .expect("Snapshot serialization always succeeds");
    let meta = snapshot_meta(now, snapshot.courses.len());
    write_atomic(&path, &(json + "\n"))?;
    // the UI shows « données du … » from this file: git keeps no mtime and
    // Pages' Last-Modified is the deploy time, so the scrape stamps itself
    // (ADR `2026-08-meta-json-provenance-du-snapshot`)
    write_atomic(&dir.join("meta.json"), &(meta + "\n"))?;
    write_error_log(&dir.join("cours_errors.log"), &anomalies)?;

    task.done();
    Ok(())
}

fn snapshot_meta(now: std::time::SystemTime, course_count: usize) -> String {
    #[derive(serde::Serialize)]
    struct SnapshotMeta {
        scraped_at: String,
        course_count: usize,
    }
    let meta = SnapshotMeta {
        scraped_at: iso_utc(now),
        course_count,
    };
    // expect over `?`: serializing a string and an integer provably
    // cannot fail
    serde_json::to_string_pretty(&meta)
        .expect("Meta serialization always succeeds")
}

// UTC ISO-8601 with an explicit Z: the stamp travels through JSON, exports
// and screenshots, so it must carry its own timezone
fn iso_utc(now: std::time::SystemTime) -> String {
    // a pre-1970 clock is a broken host; flooring it to the epoch keeps the
    // stamp total and the bogus date visible in the file
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    let (year, month, day) = civil_from_days(secs / 86_400);
    let in_day = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        in_day / 3_600,
        in_day % 3_600 / 60,
        in_day % 60
    )
}

// A `--subjects` run rewrites exactly its own subjects' courses inside the
// snapshot, and nothing else. A missing file (first run) holds nothing; an
// unreadable one stops the run — merging on regardless would drop every
// subject the file held, which is the very loss the merge exists to
// prevent.
fn merge_into_existing(
    path: &Path,
    produced: Snapshot,
    subjects: &[String],
) -> anyhow::Result<Snapshot> {
    let wanted: Vec<String> =
        subjects.iter().map(|s| s.to_uppercase()).collect();
    let mut courses = produced.courses;

    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let existing: Snapshot =
                serde_json::from_str(&raw).map_err(|source| {
                    anyhow::anyhow!("Parsing {}: {source}", path.display())
                })?;
            courses.extend(
                existing.courses.into_iter().filter(|course| {
                    !has_wanted_subject(&course.code, &wanted)
                }),
            );
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => anyhow::bail!("Reading {}: {source}", path.display()),
    }

    // the order a full run writes (`course::snapshot`), so a subject run
    // leaves a diff holding its own courses and nothing else
    Ok(course::snapshot(courses))
}

// An empty URL list means « refresh what is already there »: file names now
// carry the official code (`B-GEX-A26.json`), which no URL can be rebuilt
// from, so the slug is read from each snapshot's own `slug` field (ADR
// `2026-08-code-officiel-de-programme-et-slug`, superseding
// `2026-07-programs-sans-url-rafraichit-par-slug`). Nothing to refresh is an
// error, never a silent no-op.
fn refresh_urls(
    output_dir: &str,
    base_url: &str,
) -> anyhow::Result<Vec<String>> {
    let dir = Path::new(output_dir).join("programmes");
    let slugs = program_slugs(&dir)?;
    anyhow::ensure!(
        !slugs.is_empty(),
        "No urls given and no programs to refresh in {}.",
        dir.display()
    );
    Ok(slugs
        .into_iter()
        .map(|slug| format!("{base_url}/{slug}"))
        .collect())
}

// `{code}-{semester}.manuel.json` is hand-maintained and never scraped (ADR
// `2026-07-cheminement-type-en-fichier-manuel`); several vintages of one
// program fold into a single slug; a missing directory holds nothing. A
// snapshot that cannot yield its slug is a hard error naming the file: a
// silently skipped program would quietly stop being refreshed.
fn program_slugs(
    dir: &Path,
) -> anyhow::Result<std::collections::BTreeSet<String>> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".manuel.json") || !name.ends_with(".json") {
                return None;
            }
            Some(entry.path())
        })
        .map(|path| {
            let content =
                std::fs::read_to_string(&path).map_err(|source| {
                    anyhow::anyhow!("Reading {}: {source}", path.display())
                })?;
            let program: ProgramSlug = serde_json::from_str(&content)
                .map_err(|source| {
                    anyhow::anyhow!(
                        "No usable `slug` in {}: {source}",
                        path.display()
                    )
                })?;
            Ok(program.slug)
        })
        .collect()
}

// The vintage a scrape captures: the session that follows the run, since a
// run prepares the coming session's version — and a program is only ever
// defined for automne or hiver, never été, so September–December prepares
// the coming hiver, which belongs to the next civil year, and every other
// month prepares the current year's automne
// (ADR `2026-08-millesime-automne-ou-hiver-jamais-ete`).
fn semester_after(now: std::time::SystemTime) -> Semester {
    // a pre-1970 clock is a broken host; flooring it to day zero keeps the
    // rule total and the bogus vintage visible in the file name
    let days = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() / 86_400)
        .unwrap_or(0);
    let (year, month, _) = civil_from_days(days);
    match month {
        9..=12 => Semester {
            season: Season::Winter,
            year: year + 1,
        },
        _ => Semester {
            season: Season::Fall,
            year,
        },
    }
}

// days since 1970-01-01 → (civil year, month, day), by Howard Hinnant's
// `civil_from_days` (branchless era arithmetic, exact for any day ≥ 0)
fn civil_from_days(days: u64) -> (u16, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    // the algorithm's year starts in March; January and February belong to
    // the next civil year
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year as u16, month, day)
}

async fn get_programs(
    urls: &[String],
    semester: Semester,
) -> (Vec<Program>, Vec<ProgramError>) {
    // expect over `?`: this static config provably builds (the failure path
    // needs an injected bad builder — seam-tested in fetch.rs)
    let fetcher = Fetcher::new(min_interval, backoff)
        .expect("static fetcher config always builds");
    program::scrape(&fetcher, urls, semester).await
}

// The computed « Scolarité préparatoire » rule, appended last: no page
// lists the 0xxx cours d'appoint a program's mandatory prerequisites can
// require, so the walk over the course snapshot supplies them (ADR
// `2026-08-regle-scolarite-preparatoire`)
fn add_preparatory_rules(
    mut programs: Vec<Program>,
    courses: &[Course],
) -> anyhow::Result<Vec<Program>> {
    for program in &mut programs {
        match preparatory_rule(&program.mandatory, courses) {
            Ok(Some(rule)) => program.rules.push(rule),
            // nothing reachable: the rule is omitted, not emitted empty
            Ok(None) => {}
            Err(error) => anyhow::bail!("{}: {error}", program.code),
        }
    }
    Ok(programs)
}

// One file per program rather than one snapshot holding them all: a run is
// restricted to the URLs it was handed, so it writes exactly those and
// leaves every other program's file — including the hand-maintained
// `{code}-{semester}.manuel.json` — alone (ADR
// `2026-07-un-fichier-par-programme`).
// The name carries the semester vintage so students keep the version they
// enrolled under (ADR `2026-08-millesime-de-programme-en-semestre`).
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
        let path = programs_dir
            .join(format!("{}-{}.json", program.code, program.semester));
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
    async fn scraped_courses_are_written_to_one_snapshot() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-happy");
        plant_catalogue(&dir, &server, &["GEX-1000"]);

        run(courses_args(&dir, &["gex"]))
            .await
            .unwrap_or_else(|e| panic!("scrape one course: {e}"));

        let snapshot = std::fs::read_to_string(dir.join("cours.json"))
            .unwrap_or_else(|e| panic!("read the snapshot: {e}"));
        assert!(snapshot.contains("GEX-1000"), "{snapshot}");
        // the vintage rides inside the offering, not in a file name
        assert!(snapshot.contains("\"last_offered\": 2026"), "{snapshot}");
        // declaration order, not alphabetical: the snapshot is committed
        // and the diffs have to stay readable
        assert!(
            snapshot.find("\"code\"") < snapshot.find("\"title\""),
            "{snapshot}"
        );
        assert!(!dir.join("cours_errors.log").exists(), "clean run");
        // the run stamps its own provenance beside the snapshot
        let meta = std::fs::read_to_string(dir.join("meta.json"))
            .unwrap_or_else(|e| panic!("read the meta: {e}"));
        assert!(meta.contains("\"course_count\": 1"), "{meta}");
        assert!(meta.contains("\"scraped_at\""), "{meta}");
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

        let snapshot = std::fs::read_to_string(dir.join("cours.json"))
            .unwrap_or_else(|e| panic!("read the snapshot: {e}"));
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
    async fn a_full_run_replaces_the_snapshot_wholesale() {
        // only a full run has seen the whole catalogue, so a course gone
        // from it is gone from the file — the atomic rename leaves nothing
        // stale (supersedes ADR `2026-07-nettoyage-des-snapshots-perimes`)
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-replace-full");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        plant_snapshot(&dir, &["GCI-1000"]);
        // ADR `2026-07-contribution-de-cours-manuels`: hand-maintained,
        // never touched by the scraper
        std::fs::write(dir.join("cours.manuel.json"), "{}")
            .unwrap_or_else(|e| panic!("plant the manuel sidecar: {e}"));

        run(courses_args(&dir, &[]))
            .await
            .unwrap_or_else(|e| panic!("full scrape: {e}"));

        assert_eq!(
            snapshot_codes(&dir),
            ["GEX-1000"],
            "a course the catalogue no longer lists is gone from the file"
        );
        let manuel = std::fs::read_to_string(dir.join("cours.manuel.json"))
            .unwrap_or_else(|e| panic!("read the manuel sidecar: {e}"));
        assert_eq!(manuel, "{}", "the sidecar is never touched");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_scoped_run_rewrites_its_own_subject_inside_the_snapshot() {
        // the regression: writing the filtered snapshot whole deleted every
        // other subject in the file — 4151 courses down to 15 on a real run
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-scoped-merge");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // GEX-9999 is no longer offered; the two others are not this run's
        // business and must survive it. They bracket GEX-1000
        // alphabetically, so appending instead of merging would show.
        plant_snapshot(&dir, &["GCI-1000", "GEX-9999", "GZZ-1000"]);

        run(courses_args(&dir, &["gex"]))
            .await
            .unwrap_or_else(|e| panic!("scoped scrape: {e}"));

        assert_eq!(
            snapshot_codes(&dir),
            ["GCI-1000", "GEX-1000", "GZZ-1000"],
            "a scoped run replaces its own subject's courses and sorts by \
             code, exactly as a full run writes the file"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_scoped_run_over_an_unreadable_snapshot_is_an_error() {
        // merging on regardless would drop every subject the file held —
        // the loss the merge exists to prevent. Unparsable and unreadable
        // are the two arms of the same guard, so both are planted; a
        // directory under the snapshot's name gives the second.
        let _guard = lock_print();
        for (name, plant) in [("unparsable", true), ("unreadable", false)] {
            let server = MockServer::start().await;
            mount_course(&server, "GEX-1000").await;
            let dir = test_dir(&format!("courses-scoped-{name}"));
            plant_catalogue(&dir, &server, &["GEX-1000"]);
            let path = dir.join("cours.json");
            if plant {
                std::fs::write(&path, "not a snapshot")
                    .unwrap_or_else(|e| panic!("plant {name}: {e}"));
            } else {
                std::fs::create_dir(&path)
                    .unwrap_or_else(|e| panic!("plant {name}: {e}"));
            }

            let result = run(courses_args(&dir, &["gex"])).await;

            assert!(result.is_err(), "an {name} snapshot must fail the run");
            cleanup(&dir);
        }
    }

    #[tokio::test]
    async fn an_unwritable_snapshot_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-blocked-snapshot");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a directory at the target path makes the rename fail
        std::fs::create_dir_all(dir.join("cours.json"))
            .unwrap_or_else(|e| panic!("block the snapshot path: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unwritable snapshot must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unwritable_meta_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_course(&server, "GEX-1000").await;
        let dir = test_dir("courses-blocked-meta");
        plant_catalogue(&dir, &server, &["GEX-1000"]);
        // a directory at the target path makes the rename fail
        std::fs::create_dir_all(dir.join("meta.json"))
            .unwrap_or_else(|e| panic!("block the meta path: {e}"));

        let result = run(courses_args(&dir, &[])).await;

        assert!(result.is_err(), "an unwritable meta must fail");
        cleanup(&dir);
    }

    #[test]
    fn a_stale_course_error_log_is_removed_on_a_clean_run() {
        // covers the CourseError instantiation of the generic
        // `write_error_log` whole in this compilation (llvm-cov scores the
        // best single compilation — ADR
        // `2026-07-couverture-par-instanciation-le-plus-petit-ecart`)
        let _guard = lock_print();
        let dir = test_dir("stale-error-log");
        let path = dir.join("cours_errors.log");
        std::fs::write(&path, "old anomalies")
            .unwrap_or_else(|e| panic!("plant the stale log: {e}"));

        let no_anomalies: Vec<CourseError> = Vec::new();
        write_error_log(&path, &no_anomalies)
            .unwrap_or_else(|e| panic!("clean run: {e}"));
        assert!(!path.exists(), "a stale log misreports a clean run");

        let anomalies = vec![CourseError::Cache {
            path: "gex-1000.json".to_string(),
            source: std::io::Error::other("boom"),
        }];
        write_error_log(&path, &anomalies)
            .unwrap_or_else(|e| panic!("write the log: {e}"));
        assert!(path.exists(), "anomalies must land in the log");
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

    // an earlier run's output: only the codes matter to a merge, so the
    // courses carry the bare minimum `Course` deserializes from
    fn plant_snapshot(dir: &Path, codes: &[&str]) {
        let courses: Vec<String> = codes
            .iter()
            .map(|code| {
                format!(
                    r#"{{"code":"{code}","title":"x","credits":3,"cycle":1,"seasons":{{}}}}"#
                )
            })
            .collect();
        let json = format!(r#"{{"courses":[{}]}}"#, courses.join(","));
        std::fs::write(dir.join("cours.json"), json)
            .unwrap_or_else(|e| panic!("plant the snapshot: {e}"));
    }

    // a snapshot whose courses carry prerequisite trees, for the
    // préuniversitaire walk: `prerequisites` is a raw JSON value, or None
    fn plant_snapshot_with_prereqs(
        dir: &Path,
        courses: &[(&str, Option<&str>)],
    ) {
        let courses: Vec<String> = courses
            .iter()
            .map(|(code, prerequisites)| {
                let prereq = prerequisites.map_or(String::new(), |json| {
                    format!(r#","prerequisites":{json}"#)
                });
                format!(
                    r#"{{"code":"{code}","title":"x","credits":3,"cycle":1{prereq},"seasons":{{}}}}"#
                )
            })
            .collect();
        let json = format!(r#"{{"courses":[{}]}}"#, courses.join(","));
        std::fs::write(dir.join("cours.json"), json)
            .unwrap_or_else(|e| panic!("plant the snapshot: {e}"));
    }

    fn snapshot_codes(dir: &Path) -> Vec<String> {
        let path = dir.join("cours.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read the snapshot: {e}"));
        let snapshot: Snapshot = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse the snapshot: {e}"));
        snapshot
            .courses
            .into_iter()
            .map(|course| course.code)
            .collect()
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

    #[test]
    fn the_semester_after_the_scrape_flips_twice_a_year() {
        // a run prepares the coming session, and a program is only defined
        // for automne or hiver: September–December → the next civil year's
        // hiver, every other month → the current year's automne — both
        // boundaries of each band are pinned
        for (date, secs, expected) in [
            ("2026-01-01", 1_767_225_600_u64, "A26"),
            ("2026-08-31", 1_788_134_400, "A26"),
            ("2026-09-01", 1_788_220_800, "H27"),
            ("2026-12-31", 1_798_675_200, "H27"),
        ] {
            let now =
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            assert_eq!(semester_after(now).to_string(), expected, "on {date}");
        }
    }

    #[test]
    fn a_pre_epoch_clock_floors_to_day_zero_instead_of_panicking() {
        let now = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
        assert_eq!(
            semester_after(now).to_string(),
            "A70",
            "January 1970 → its coming automne"
        );
    }

    #[test]
    fn the_snapshot_meta_stamps_an_utc_instant_and_the_count() {
        // 2026-09-01T00:00:00Z (pinned above) plus 01:02:03 into the day
        let now = std::time::UNIX_EPOCH
            + std::time::Duration::from_secs(1_788_220_800 + 3_723);
        assert_eq!(
            snapshot_meta(now, 8_834),
            "{\n  \"scraped_at\": \"2026-09-01T01:02:03Z\",\n  \"course_count\": 8834\n}"
        );
    }

    #[test]
    fn iso_utc_pins_the_epoch_a_leap_day_and_a_year_end() {
        for (secs, expected) in [
            (0_u64, "1970-01-01T00:00:00Z"),
            (1_709_164_800, "2024-02-29T00:00:00Z"),
            (1_735_689_599, "2024-12-31T23:59:59Z"),
        ] {
            let now =
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            assert_eq!(iso_utc(now), expected);
        }
    }

    #[test]
    fn a_pre_epoch_clock_floors_the_meta_stamp_to_the_epoch() {
        let now = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
        assert_eq!(iso_utc(now), "1970-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn scraped_programs_are_written_one_file_each() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil", "B-GCI").await;
        mount_program(&server, "genie-des-eaux", "B-GEX").await;
        let dir = test_dir("programs-happy");
        plant_snapshot(&dir, &[]);

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
        let semester = semester_after(std::time::SystemTime::now());
        // the file carries the official code, the slug lives inside it
        let civil = std::fs::read_to_string(
            programmes.join(format!("B-GCI-{semester}.json")),
        )
        .unwrap_or_else(|e| panic!("read the program file: {e}"));
        assert!(civil.contains("\"slug\": \"genie-civil\""), "{civil}");
        assert!(
            civil.contains(&format!("\"semester\": \"{semester}\"")),
            "{civil}"
        );
        assert!(programmes.join(format!("B-GEX-{semester}.json")).exists());
        // declaration order, not alphabetical: these files are committed and
        // the diffs have to stay readable
        assert!(civil.find("\"code\"") < civil.find("\"title\""), "{civil}");
        assert!(!dir.join("programmes_errors.log").exists(), "clean run");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn program_without_urls_refreshes_every_existing_program() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // expect(1): two vintages of the same slug must fold into a single
        // fetch, and the server itself proves it
        Mock::given(method("GET"))
            .and(wiremock::matchers::path("/genie-civil"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                crate::program::tests::program_html("genie-civil", "B-GCI"),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let dir = test_dir("programs-refresh");
        plant_snapshot(&dir, &[]);
        let programmes = dir.join("programmes");
        std::fs::create_dir_all(&programmes)
            .unwrap_or_else(|e| panic!("create the programs dir: {e}"));
        // the slug comes from the content, never the file name — the three
        // vintages (one legacy pre-code name among them) all fold into one
        for name in
            ["B-GCI-A25.json", "B-GCI-E25.json", "genie-civil-2025.json"]
        {
            std::fs::write(
                programmes.join(name),
                "{\"slug\": \"genie-civil\"}\n",
            )
            .unwrap_or_else(|e| panic!("plant {name}: {e}"));
        }
        std::fs::write(
            programmes.join("genie-civil.manuel.json"),
            "by hand\n",
        )
        .unwrap_or_else(|e| panic!("plant the manuel file: {e}"));
        std::fs::write(programmes.join("notes.txt"), "not a snapshot\n")
            .unwrap_or_else(|e| panic!("plant the stray file: {e}"));

        run(vec![
            "program".to_string(),
            "--output-dir".to_string(),
            dir.display().to_string(),
            "--semester".to_string(),
            "A26".to_string(),
            "--base-url".to_string(),
            server.uri(),
        ])
        .await
        .unwrap_or_else(|e| panic!("refresh the programs: {e}"));

        let refreshed =
            std::fs::read_to_string(programmes.join("B-GCI-A26.json"))
                .unwrap_or_else(|e| panic!("read the refreshed file: {e}"));
        assert!(refreshed.contains("\"semester\": \"A26\""), "{refreshed}");
        // earlier vintages are history, not targets: they survive untouched
        for name in
            ["B-GCI-A25.json", "B-GCI-E25.json", "genie-civil-2025.json"]
        {
            let content = std::fs::read_to_string(programmes.join(name))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            assert_eq!(
                content, "{\"slug\": \"genie-civil\"}\n",
                "{name} must survive a refresh"
            );
        }
        let manuel = std::fs::read_to_string(
            programmes.join("genie-civil.manuel.json"),
        )
        .unwrap_or_else(|e| panic!("read the manuel file: {e}"));
        assert_eq!(manuel, "by hand\n", "hand-maintained, never scraped");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn program_without_urls_and_no_existing_programs_is_an_error() {
        // an empty refresh list can only mean the caller pointed at the
        // wrong directory — never a silent no-op
        let dir = test_dir("programs-refresh-empty");
        plant_snapshot(&dir, &[]);

        let error = run(vec![
            "program".to_string(),
            "--output-dir".to_string(),
            dir.display().to_string(),
        ])
        .await
        .expect_err("nothing to refresh must be said");

        let message = error.to_string();
        assert!(message.contains("programmes"), "{message}");
        cleanup(&dir);
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
        mount_program(&server, "genie-civil", "B-GCI").await;
        // nothing mounted for the second URL, so it 404s
        let dir = test_dir("programs-error-log");
        plant_snapshot(&dir, &[]);

        run(programs_args(
            &dir,
            &[
                &program_url(&server, "genie-civil"),
                &program_url(&server, "genie-absent"),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("a 404 must not fail the run: {e}"));

        let semester = semester_after(std::time::SystemTime::now());
        assert!(
            dir.join("programmes")
                .join(format!("B-GCI-{semester}.json"))
                .exists(),
            "the reachable program still lands"
        );
        let logged =
            std::fs::read_to_string(dir.join("programmes_errors.log"))
                .unwrap_or_else(|e| panic!("read the error log: {e}"));
        assert!(logged.contains("genie-absent"), "{logged}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_snapshot_that_cannot_yield_its_slug_fails_the_refresh() {
        // both a file that is not JSON and JSON without a `slug` land on the
        // same rejection: skipping one would quietly stop refreshing it
        for (name, content) in
            [("broken.json", "stale\n"), ("no-slug.json", "{}\n")]
        {
            let dir = test_dir(&format!("programs-refresh-{name}"));
            plant_snapshot(&dir, &[]);
            let programmes = dir.join("programmes");
            std::fs::create_dir_all(&programmes)
                .unwrap_or_else(|e| panic!("create the programs dir: {e}"));
            std::fs::write(programmes.join(name), content)
                .unwrap_or_else(|e| panic!("plant {name}: {e}"));

            let error = run(vec![
                "program".to_string(),
                "--output-dir".to_string(),
                dir.display().to_string(),
            ])
            .await
            .expect_err("a slugless snapshot must be said");

            assert!(
                error.to_string().contains(name),
                "the error names the file: {error}"
            );
            cleanup(&dir);
        }
    }

    #[tokio::test]
    async fn a_snapshot_that_cannot_be_read_fails_the_refresh() {
        // a directory named like a snapshot defeats `read_to_string`
        let dir = test_dir("programs-refresh-unreadable");
        plant_snapshot(&dir, &[]);
        let programmes = dir.join("programmes");
        std::fs::create_dir_all(programmes.join("B-GCI-A26.json"))
            .unwrap_or_else(|e| panic!("plant the directory: {e}"));

        let error = run(vec![
            "program".to_string(),
            "--output-dir".to_string(),
            dir.display().to_string(),
        ])
        .await
        .expect_err("an unreadable snapshot must be said");

        assert!(
            error.to_string().contains("B-GCI-A26.json"),
            "the error names the file: {error}"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unusable_programs_dir_is_an_error() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil", "B-GCI").await;
        let dir = test_dir("programs-blocked-dir");
        plant_snapshot(&dir, &[]);
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
        mount_program(&server, "genie-civil", "B-GCI").await;
        let dir = test_dir("programs-blocked-file");
        plant_snapshot(&dir, &[]);
        // a directory at the target path makes the rename fail
        let semester = semester_after(std::time::SystemTime::now());
        std::fs::create_dir_all(
            dir.join("programmes")
                .join(format!("B-GCI-{semester}.json")),
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
        plant_snapshot(&dir, &[]);
        std::fs::create_dir_all(dir.join("programmes_errors.log"))
            .unwrap_or_else(|e| panic!("block the log path: {e}"));

        let result =
            run(programs_args(&dir, &[&program_url(&server, "absent")])).await;

        assert!(result.is_err(), "an unwritable error log must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn program_without_a_course_snapshot_fails_before_fetching() {
        // the préuniversitaire walk needs the courses: without them the run
        // must stop before a single page is fetched
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil", "B-GCI").await;
        let dir = test_dir("programs-no-course-snapshot");

        let error =
            run(programs_args(&dir, &[&program_url(&server, "genie-civil")]))
                .await
                .expect_err("a missing course snapshot must be said");

        let message = error.to_string();
        assert!(message.contains("cours.json"), "{message}");
        assert!(
            message.contains("Run `ulaval-scraper courses` first."),
            "{message}"
        );
        let requests = server.received_requests().await.unwrap_or_default();
        assert!(requests.is_empty(), "nothing may be fetched");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unparsable_course_snapshot_fails_the_run() {
        let server = MockServer::start().await;
        mount_program(&server, "genie-civil", "B-GCI").await;
        let dir = test_dir("programs-broken-course-snapshot");
        std::fs::write(dir.join("cours.json"), "not json")
            .unwrap_or_else(|e| panic!("plant the broken snapshot: {e}"));

        let error =
            run(programs_args(&dir, &[&program_url(&server, "genie-civil")]))
                .await
                .expect_err("an unparsable course snapshot must be said");

        let message = error.to_string();
        assert!(message.contains("Parsing"), "{message}");
        assert!(message.contains("cours.json"), "{message}");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_preparatory_rule_is_appended_from_the_prerequisite_trees() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // the mock page's mandatory list is ["GEX-1000"]
        mount_program(&server, "genie-des-eaux", "B-GEX").await;
        let dir = test_dir("programs-preparatory-rule");
        plant_snapshot_with_prereqs(
            &dir,
            &[
                // the cégep sigle in the OR branch is prose, not a code
                (
                    "GEX-1000",
                    Some(
                        r#"{"raw":"MAT-0150 OU BIO-NYA","tree":{"any":["MAT-0150",{"raw":"BIO-NYA"}]}}"#,
                    ),
                ),
                // préuniversitaire courses chain: the closure is transitive
                ("MAT-0150", Some(r#"{"raw":"MAT-0130","tree":"MAT-0130"}"#)),
            ],
        );

        run(programs_args(
            &dir,
            &[&program_url(&server, "genie-des-eaux")],
        ))
        .await
        .unwrap_or_else(|e| panic!("scrape the program: {e}"));

        let semester = semester_after(std::time::SystemTime::now());
        let written = std::fs::read_to_string(
            dir.join("programmes")
                .join(format!("B-GEX-{semester}.json")),
        )
        .unwrap_or_else(|e| panic!("read the program file: {e}"));
        let program: serde_json::Value = serde_json::from_str(&written)
            .unwrap_or_else(|e| panic!("parse the program file: {e}"));
        let rule = program["rules"]
            .as_array()
            .and_then(|rules| rules.last())
            .unwrap_or_else(|| panic!("a rule must be appended: {written}"));
        assert_eq!(rule["title"], "Scolarité préparatoire");
        assert_eq!(
            rule["courses"],
            serde_json::json!(["MAT-0130", "MAT-0150"])
        );
        assert!(
            rule.get("constraint").is_none(),
            "no constraint may be invented: {rule}"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_prerequisite_graph_over_budget_fails_and_names_the_program() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount_program(&server, "genie-des-eaux", "B-GEX").await;
        let dir = test_dir("programs-preparatory-over-budget");
        let leaves: Vec<String> =
            (0..10_000).map(|i| format!(r#""XXX-{i:04}""#)).collect();
        let tree = format!(
            r#"{{"raw":"x","tree":{{"all":[{}]}}}}"#,
            leaves.join(",")
        );
        plant_snapshot_with_prereqs(&dir, &[("GEX-1000", Some(&tree))]);

        let error = run(programs_args(
            &dir,
            &[&program_url(&server, "genie-des-eaux")],
        ))
        .await
        .expect_err("an over-budget graph must be said");

        let message = error.to_string();
        assert!(message.contains("B-GEX"), "{message}");
        assert!(message.contains("exceeds"), "{message}");
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

    async fn mount_program(server: &MockServer, slug: &str, code: &str) {
        Mock::given(method("GET"))
            .and(wiremock::matchers::path(format!("/{slug}")))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                crate::program::tests::program_html(slug, code),
            ))
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
