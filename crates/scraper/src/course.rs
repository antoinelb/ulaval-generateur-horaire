use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use futures::stream::{self, StreamExt};

use crate::fetch::{FetchError, Fetcher};
use crate::parser::{self, ParseError};
use crate::print;
use ulaval_scheduler_core::{CatalogueEntry, Course, Cycle, Season};

const n_concurrent: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum CourseError {
    // `FetchError` already names the URL it failed on
    #[error(transparent)]
    Fetch(#[from] FetchError),
    // `ParseError` only names a selector, so the page has to be added for
    // the log line to be actionable
    #[error("Parsing {url}: {source}")]
    Parse {
        url: String,
        #[source]
        source: ParseError,
    },
    #[error("Caching {path}: {source}")]
    Cache {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

// What a cache file holds. `years` records which year each retained season
// was read from: `Course` is keyed by season alone, but the snapshot files
// are named per session (`a2026`), so the year has to survive the trip.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CachedCourse {
    pub course: Course,
    pub years: BTreeMap<Season, u16>,
}

// A cache file is one of two disjoint shapes, read untagged: a parsed
// course (`{course, years}`), or the verdict that a page yields none —
// stamped with the scope rule that reached it. Untagged is safe because the
// two carry disjoint required fields, the same argument as `Credits`; a file
// matching neither is a miss, so a corrupt cache refetches rather than lies.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum CacheEntry {
    Course(CachedCourse),
    // an out-of-scope page has no `Course` to hold, so caching it needs its
    // own shape; `out_of_scope` is the scope fingerprint at write time
    OutOfScope { out_of_scope: String },
}

// A fingerprint of the in-scope rule as it stands in the code — the cycle
// levels `Cycle` accepts — not an enumeration of reality. A sentinel is
// trusted only while its fingerprint still matches: add a third cycle and
// every sentinel written under « first and second only » stops matching, so
// those pages are read again instead of staying wrongly excluded, with no
// hand-purge of the cache. Bounded scan over `u8`, no recursion.
fn scope_tag() -> String {
    (0u8..=u8::MAX)
        .filter(|&level| Cycle::try_from(level).is_ok())
        .map(|level| level.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

// Where one course came from. A cache the parser can no longer read — a
// format change invalidates every file at once — behaves exactly like a
// cold cache, so the run has to be able to say which it was.
enum Origin {
    Cache,
    Network,
}

// What a whole course scrape cost, for its closing line.
pub struct CacheTally {
    pub cached: usize,
    pub fetched: usize,
}

// One `data/cours/{session}.json`, mirroring the catalogue's shape. A
// struct rather than a `json!` literal so serde keeps `Course`'s field
// order: these snapshots are committed, and alphabetized keys would churn
// the diffs and diverge from the `courses/*.json` fixtures.
// `Deserialize` too: a `--subjects` run reads back the snapshot it is about
// to rewrite, to keep the subjects it knows nothing about.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SessionSnapshot {
    pub courses: Vec<Course>,
}

pub async fn scrape(
    fetcher: &Fetcher,
    entries: &[CatalogueEntry],
    cache_dir: &Path,
) -> (Vec<CachedCourse>, Vec<CourseError>, CacheTally) {
    let task = print::progress_task(
        "Scraping courses...",
        "Scraped courses.",
        entries.len(),
    );
    let progress = &task;

    // `collect`, not `try_collect` as the catalogue does: at ~10 req/s a
    // full run is ~17 min, and one unreachable page must not throw all of
    // it away (ADR `2026-07-echec-de-page-cours-non-bloquant`)
    let scraped: Vec<(Option<CachedCourse>, Vec<CourseError>, Origin)> =
        stream::iter(entries)
            .map(|entry| async move {
                let scraped = scrape_course(fetcher, entry, cache_dir).await;
                progress.increment();
                scraped
            })
            .buffer_unordered(n_concurrent)
            .collect()
            .await;
    task.done();

    let mut courses = Vec::with_capacity(scraped.len());
    let mut anomalies = Vec::new();
    let mut tally = CacheTally {
        cached: 0,
        fetched: 0,
    };
    for (course, mut errors, origin) in scraped {
        courses.extend(course);
        anomalies.append(&mut errors);
        match origin {
            Origin::Cache => tally.cached += 1,
            Origin::Network => tally.fetched += 1,
        }
    }
    (courses, anomalies, tally)
}

async fn scrape_course(
    fetcher: &Fetcher,
    entry: &CatalogueEntry,
    cache_dir: &Path,
) -> (Option<CachedCourse>, Vec<CourseError>, Origin) {
    let path = cache_path(cache_dir, &entry.code);
    match read_cache(&path) {
        Some(CacheEntry::Course(cached)) => {
            return (Some(cached), Vec::new(), Origin::Cache);
        }
        // the verdict holds only while the rule that produced it does; a
        // stale fingerprint falls through and the page is fetched again
        Some(CacheEntry::OutOfScope { out_of_scope })
            if out_of_scope == scope_tag() =>
        {
            return (None, Vec::new(), Origin::Cache);
        }
        _ => {}
    }

    let html = match fetcher.fetch(&entry.url).await {
        Ok(html) => html,
        Err(source) => return (None, vec![source.into()], Origin::Network),
    };
    // an unrecognized page shape yields no course at all, so nothing is
    // cached and the next run fetches it again
    let page = match parser::course::parse(&html) {
        Ok(Some(page)) => page,
        // a page read perfectly and dropped on purpose — a doctoral or
        // post-doctoral activity — is no course, but its verdict is cached
        // so the next run skips the request (ADR
        // `2026-07-cache-du-verdict-hors-perimetre`)
        Ok(None) => {
            let sentinel = CacheEntry::OutOfScope {
                out_of_scope: scope_tag(),
            };
            let anomalies = match write_cache(&path, &sentinel) {
                Ok(()) => Vec::new(),
                Err(source) => vec![CourseError::Cache {
                    path: path.display().to_string(),
                    source,
                }],
            };
            return (None, anomalies, Origin::Network);
        }
        Err(source) => {
            let error = CourseError::Parse {
                url: entry.url.clone(),
                source,
            };
            return (None, vec![error], Origin::Network);
        }
    };

    let course = CachedCourse {
        course: page.course,
        years: page.years,
    };
    let mut anomalies: Vec<CourseError> = page
        .anomalies
        .into_iter()
        .map(|source| CourseError::Parse {
            url: entry.url.clone(),
            source,
        })
        .collect();

    // only a clean parse is cached: a course parsed with anomalies must be
    // fetched again next run so a parser fix reaches it without anyone
    // having to purge the cache by hand
    if anomalies.is_empty() {
        if let Err(source) = write_cache(&path, &course) {
            anomalies.push(CourseError::Cache {
                path: path.display().to_string(),
                source,
            });
        }
    }

    (Some(course), anomalies, Origin::Network)
}

fn cache_path(cache_dir: &Path, code: &str) -> PathBuf {
    cache_dir.join(format!("{}.json", code.to_lowercase()))
}

fn read_cache(path: &Path) -> Option<CacheEntry> {
    // a missing, truncated or outdated-format file is a miss, not a
    // failure: the page is fetched again and the file overwritten, which
    // is also why the write below needs no temp-file dance
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

// generic over what is cached: a `CachedCourse` writes `{course, years}`, a
// `CacheEntry::OutOfScope` writes `{out_of_scope}`, and `read_cache` reads
// either back through the untagged enum
fn write_cache<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), std::io::Error> {
    // expect over `?`: serializing strings, maps and vecs provably cannot
    // fail
    let json = serde_json::to_string(value)
        .expect("cache entry serialization always succeeds");
    std::fs::write(path, json)
}

// One course page can feed several session snapshots: a page lists up to
// three seasons at once, each with its own year (ECN-4901 carries Hiver
// 2026 and Été 2026).
pub fn group_by_session(
    courses: Vec<CachedCourse>,
) -> BTreeMap<String, SessionSnapshot> {
    let mut sessions: BTreeMap<String, SessionSnapshot> = BTreeMap::new();

    for CachedCourse { course, years } in courses {
        for (season, offering) in &course.seasons {
            // a season with no recorded year belongs to no session, so no
            // file could hold it; only a hand-edited cache file gets here
            let Some(year) = years.get(season) else {
                continue;
            };
            sessions
                .entry(session_name(*season, *year))
                .or_default()
                .courses
                .push(Course {
                    // the snapshot is already named after the session, so
                    // it carries that season alone
                    seasons: BTreeMap::from([(*season, offering.clone())]),
                    ..course.clone()
                });
        }
    }

    // `buffer_unordered` yields in completion order, which network timing
    // makes arbitrary; these snapshots are committed, so they are sorted by
    // code like the catalogue is, to keep the git diffs meaningful
    for snapshot in sessions.values_mut() {
        snapshot.courses.sort_by(|a, b| a.code.cmp(&b.code));
    }

    sessions
}

fn session_name(season: Season, year: u16) -> String {
    let season = match season {
        Season::Fall => 'a',
        Season::Winter => 'h',
        Season::Summer => 'e',
    };
    format!("{season}{year}")
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
pub(crate) mod tests {
    // TEST_STATE_LOCK serializes whole tests around the global print state,
    // so holding it across await points is the intent, not an oversight
    #![allow(clippy::await_holding_lock)]

    use std::time::Duration;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use ulaval_scheduler_core::{Credits, Cycle, SeasonOffering};

    #[test]
    fn a_session_is_named_by_its_season_letter_and_year() {
        for (season, year, expected) in [
            (Season::Fall, 2026, "a2026"),
            (Season::Winter, 2026, "h2026"),
            (Season::Summer, 2026, "e2026"),
            (Season::Fall, 2021, "a2021"),
        ] {
            assert_eq!(session_name(season, year), expected);
        }
    }

    #[test]
    fn a_course_feeds_one_snapshot_per_season_it_is_offered_in() {
        // ECN-4901's shape: one page, two seasons, and here two different
        // years — so two session files, each carrying its own season alone
        let cached = cached_course(
            "ECN-4901",
            &[(Season::Winter, 2026), (Season::Summer, 2025)],
        );

        let sessions = group_by_session(vec![cached]);

        assert_eq!(
            sessions.keys().collect::<Vec<_>>(),
            ["e2025", "h2026"],
            "one file per season+year pair"
        );
        let winter = &sessions["h2026"].courses[0];
        assert_eq!(winter.code, "ECN-4901");
        assert_eq!(
            winter.seasons.keys().collect::<Vec<_>>(),
            [&Season::Winter],
            "the snapshot is named after the session, so it carries that \
             season alone"
        );
        assert_eq!(
            sessions["e2025"].courses[0]
                .seasons
                .keys()
                .collect::<Vec<_>>(),
            [&Season::Summer]
        );
    }

    #[test]
    fn a_snapshot_is_sorted_by_code_whatever_order_courses_arrive_in() {
        // courses come back in completion order, which network timing makes
        // arbitrary; the snapshots are committed, so the file must not
        // depend on which page answered first
        let arrived = vec![
            cached_course("GEX-2000", &[(Season::Fall, 2026)]),
            cached_course("GCI-1007", &[(Season::Fall, 2026)]),
            cached_course("GEX-1000", &[(Season::Fall, 2026)]),
        ];

        let sessions = group_by_session(arrived);

        let codes: Vec<&str> = sessions["a2026"]
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["GCI-1007", "GEX-1000", "GEX-2000"]);
    }

    #[test]
    fn a_season_with_no_recorded_year_belongs_to_no_session() {
        // only a hand-edited or truncated cache file gets here: the season
        // names no snapshot, so there is no file it could go in
        let mut cached = cached_course("GEX-1000", &[(Season::Fall, 2026)]);
        cached.years.clear();

        assert!(group_by_session(vec![cached]).is_empty());
    }

    #[test]
    fn the_cache_path_is_the_lowercased_code() {
        assert_eq!(
            cache_path(Path::new("/cache"), "GEX-1000"),
            Path::new("/cache/gex-1000.json")
        );
    }

    #[test]
    fn a_missing_or_corrupt_cache_file_is_a_miss() {
        let dir = test_dir("cache-miss");

        assert!(read_cache(&dir.join("absent.json")).is_none());

        let corrupt = dir.join("corrupt.json");
        std::fs::write(&corrupt, "{ truncated")
            .unwrap_or_else(|e| panic!("plant a corrupt cache file: {e}"));
        assert!(
            read_cache(&corrupt).is_none(),
            "a corrupt file must re-fetch, not fail the run"
        );

        cleanup(&dir);
    }

    #[test]
    fn a_written_cache_file_reads_back() {
        let dir = test_dir("cache-roundtrip");
        let path = dir.join("gex-1000.json");
        let course = cached_course("GEX-1000", &[(Season::Fall, 2026)]);

        write_cache(&path, &course)
            .unwrap_or_else(|e| panic!("write the cache file: {e}"));

        let read = read_cache(&path).expect("the file was just written");
        let CacheEntry::Course(read) = read else {
            panic!("a course file must read back as a course, not a verdict");
        };
        assert_eq!(read.course, course.course);
        assert_eq!(read.years, course.years);
        cleanup(&dir);
    }

    #[test]
    fn a_cache_write_onto_a_directory_is_an_error() {
        let dir = test_dir("cache-blocked");
        let path = dir.join("gex-1000.json");
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|e| panic!("block the cache path: {e}"));

        let result = write_cache(&path, &cached_course("GEX-1000", &[]));

        assert!(result.is_err(), "writing over a directory must fail");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_scraped_course_is_returned_and_cached() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "/gex-1000", course_html("GEX-1000"), 1).await;
        let dir = test_dir("scrape-happy");

        let (courses, anomalies, _) =
            scrape_one(&server, "GEX-1000", &dir).await;

        assert!(anomalies.is_empty(), "{anomalies:?}");
        assert_eq!(courses[0].course.code, "GEX-1000");
        assert_eq!(courses[0].years[&Season::Fall], 2026);
        assert!(
            dir.join("gex-1000.json").exists(),
            "a clean parse must be cached"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_cached_course_is_not_fetched_again() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // expect(0): a cache hit must issue no request at all
        mount(&server, "/gex-1000", course_html("GEX-1000"), 0).await;
        let dir = test_dir("scrape-cache-hit");
        write_cache(
            &dir.join("gex-1000.json"),
            &cached_course("GEX-1000", &[(Season::Fall, 2026)]),
        )
        .unwrap_or_else(|e| panic!("prime the cache: {e}"));

        let (courses, anomalies, _) =
            scrape_one(&server, "GEX-1000", &dir).await;

        assert!(anomalies.is_empty(), "{anomalies:?}");
        assert_eq!(courses[0].course.code, "GEX-1000");
        cleanup(&dir);
    }

    #[tokio::test]
    async fn the_tally_separates_cache_hits_from_requests() {
        // a cache the parser can no longer read is a cold run wearing a
        // full cache directory; the totals alone cannot tell them apart
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "/gex-1000", course_html("GEX-1000"), 0).await;
        mount(&server, "/gex-2000", course_html("GEX-2000"), 1).await;
        let dir = test_dir("scrape-tally");
        write_cache(
            &dir.join("gex-1000.json"),
            &cached_course("GEX-1000", &[(Season::Fall, 2026)]),
        )
        .unwrap_or_else(|e| panic!("prime the cache: {e}"));
        let entries = [entry(&server, "GEX-1000"), entry(&server, "GEX-2000")];

        let (_, anomalies, tally) = scrape_with(&entries, &dir).await;

        assert!(anomalies.is_empty(), "{anomalies:?}");
        assert_eq!((tally.cached, tally.fetched), (1, 1));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unreachable_page_is_an_anomaly_and_the_run_continues() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "/gex-1000", course_html("GEX-1000"), 1).await;
        // nothing mounted for the second course, so it 404s
        let dir = test_dir("scrape-404");
        let entries = [entry(&server, "GEX-1000"), entry(&server, "GEX-9999")];

        let (courses, anomalies, _) = scrape_with(&entries, &dir).await;

        assert_eq!(courses.len(), 1, "the reachable course still lands");
        assert!(
            matches!(&anomalies[0], CourseError::Fetch(error)
                if error.to_string().contains("gex-9999")),
            "got {anomalies:?}"
        );
        assert!(!dir.join("gex-9999.json").exists());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn an_unrecognized_page_is_an_anomaly_and_caches_nothing() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "/gex-1000", "<html></html>".to_string(), 1).await;
        let dir = test_dir("scrape-unparseable");

        let (courses, anomalies, _) =
            scrape_one(&server, "GEX-1000", &dir).await;

        assert!(courses.is_empty(), "no course can be built from the page");
        assert!(
            matches!(&anomalies[0], CourseError::Parse { url, .. }
                if url.contains("gex-1000")),
            "got {anomalies:?}"
        );
        assert!(!dir.join("gex-1000.json").exists());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_page_out_of_scope_yields_no_course_and_no_anomaly() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // a doctoral activity (PSY-7851's shape): recognized, then dropped
        // on purpose, so there is nothing to log — but the verdict is cached
        let html = course_html("PSY-7851")
            .replace("Premier cycle", "Troisième cycle");
        mount(&server, "/psy-7851", html, 1).await;
        let dir = test_dir("scrape-out-of-scope");

        let (courses, anomalies, _) =
            scrape_one(&server, "PSY-7851", &dir).await;

        assert!(courses.is_empty(), "nothing this generator schedules");
        assert!(
            anomalies.is_empty(),
            "dropping on purpose is not an anomaly: {anomalies:?}"
        );
        // the verdict is cached under the scope fingerprint that reached it
        let cached = read_cache(&dir.join("psy-7851.json"))
            .expect("the verdict is cached");
        assert!(
            matches!(cached, CacheEntry::OutOfScope { out_of_scope }
                if out_of_scope == scope_tag()),
            "a cached verdict carries the live scope fingerprint"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_cached_out_of_scope_verdict_skips_the_request() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // expect(1): the second scrape must be served from the sentinel, so
        // the out-of-scope pages stop refetching every run
        let html = course_html("PSY-7851")
            .replace("Premier cycle", "Troisième cycle");
        mount(&server, "/psy-7851", html, 1).await;
        let dir = test_dir("scrape-out-of-scope-cached");

        let (_, _, first) = scrape_one(&server, "PSY-7851", &dir).await;
        assert_eq!((first.cached, first.fetched), (0, 1), "cold: fetched");

        let (courses, anomalies, second) =
            scrape_one(&server, "PSY-7851", &dir).await;
        assert_eq!((second.cached, second.fetched), (1, 0), "warm: cached");
        assert!(courses.is_empty() && anomalies.is_empty());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_verdict_under_a_stale_scope_fingerprint_is_refetched() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // a sentinel left by an older scope rule: the page is read again so
        // a rule change never leaves a page wrongly excluded, no hand-purge
        let html = course_html("PSY-7851")
            .replace("Premier cycle", "Troisième cycle");
        mount(&server, "/psy-7851", html, 1).await;
        let dir = test_dir("scrape-out-of-scope-stale");
        write_cache(
            &dir.join("psy-7851.json"),
            &CacheEntry::OutOfScope {
                out_of_scope: "1,2,3".to_string(),
            },
        )
        .unwrap_or_else(|e| panic!("plant a stale sentinel: {e}"));

        let (_, _, tally) = scrape_one(&server, "PSY-7851", &dir).await;

        assert_eq!(
            (tally.cached, tally.fetched),
            (0, 1),
            "a stale fingerprint must not be trusted"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_course_parsed_with_anomalies_is_kept_but_not_cached() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        // « Printemps » is no season the parser knows: the session block is
        // dropped and surfaced, but the course itself still parses
        let html =
            course_html("GEX-1000").replace("Automne 2026", "Printemps 2026");
        mount(&server, "/gex-1000", html, 1).await;
        let dir = test_dir("scrape-soft-anomaly");

        let (courses, anomalies, _) =
            scrape_one(&server, "GEX-1000", &dir).await;

        assert_eq!(courses[0].course.code, "GEX-1000", "the course is kept");
        assert_eq!(anomalies.len(), 1, "and its anomaly is surfaced");
        assert!(
            !dir.join("gex-1000.json").exists(),
            "a degraded parse must re-fetch next run, so it is not cached"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_failing_cache_write_is_an_anomaly() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        mount(&server, "/gex-1000", course_html("GEX-1000"), 1).await;
        let dir = test_dir("scrape-cache-blocked");
        std::fs::create_dir_all(dir.join("gex-1000.json"))
            .unwrap_or_else(|e| panic!("block the cache path: {e}"));

        let (courses, anomalies, _) =
            scrape_one(&server, "GEX-1000", &dir).await;

        assert_eq!(courses.len(), 1, "the course is still produced");
        assert!(
            matches!(&anomalies[0], CourseError::Cache { path, .. }
                if path.contains("gex-1000")),
            "got {anomalies:?}"
        );
        cleanup(&dir);
    }

    #[tokio::test]
    async fn a_failing_sentinel_write_is_an_anomaly() {
        let _guard = lock_print();
        let server = MockServer::start().await;
        let html = course_html("PSY-7851")
            .replace("Premier cycle", "Troisième cycle");
        mount(&server, "/psy-7851", html, 1).await;
        let dir = test_dir("scrape-sentinel-blocked");
        std::fs::create_dir_all(dir.join("psy-7851.json"))
            .unwrap_or_else(|e| panic!("block the cache path: {e}"));

        let (courses, anomalies, _) =
            scrape_one(&server, "PSY-7851", &dir).await;

        assert!(courses.is_empty(), "still out of scope, no course");
        assert!(
            matches!(&anomalies[0], CourseError::Cache { path, .. }
                if path.contains("psy-7851")),
            "a sentinel that cannot be written is logged like any cache miss \
             to write: {anomalies:?}"
        );
        cleanup(&dir);
    }

    async fn scrape_one(
        server: &MockServer,
        code: &str,
        cache_dir: &Path,
    ) -> (Vec<CachedCourse>, Vec<CourseError>, CacheTally) {
        scrape_with(&[entry(server, code)], cache_dir).await
    }

    async fn scrape_with(
        entries: &[CatalogueEntry],
        cache_dir: &Path,
    ) -> (Vec<CachedCourse>, Vec<CourseError>, CacheTally) {
        // zero intervals: throttle timing is unit-tested on a virtual
        // clock in fetch.rs; these tests assert orchestration and must
        // stay fast
        let fetcher = Fetcher::new(Duration::ZERO, Duration::ZERO)
            .unwrap_or_else(|e| panic!("build fetcher: {e}"));
        scrape(&fetcher, entries, cache_dir).await
    }

    fn entry(server: &MockServer, code: &str) -> CatalogueEntry {
        CatalogueEntry {
            code: code.to_string(),
            title: format!("Cours {code}"),
            url: format!("{}/{}", server.uri(), code.to_lowercase()),
        }
    }

    async fn mount(
        server: &MockServer,
        route: &str,
        html: String,
        expected: u64,
    ) {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_string(html))
            .expect(expected)
            .mount(server)
            .await;
    }

    // the smallest page the course parser accepts: code, title, credits,
    // cycle, and one session holding one section
    pub(crate) fn course_html(code: &str) -> String {
        format!(
            concat!(
                r#"<html><body>"#,
                r#"<span class="fe--titre-type">{code}</span>"#,
                r#"<span class="fe--titre-nom">Cours {code}</span>"#,
                r#"<ul class="fe--faits-rapides"><li>"#,
                r#"<span class="promo-entete--titre">3</span>"#,
                r#"<span class="promo-entete--contenu">Crédits</span>"#,
                r#"</li></ul>"#,
                r#"<ul class="fe--faits-rapides"><li>"#,
                r#"<p class="promo-paragraphe">Cycle du cours</p>"#,
                r#"<ul class="promo-entete--contenu">"#,
                r#"<li><strong>Premier cycle</strong></li></ul>"#,
                r#"</li></ul>"#,
                r#"<div class="collapsible-sections">"#,
                r#"<div class="sections-controls">"#,
                r#"<p class="controls-title">"#,
                r#"<strong>Automne 2026 –</strong> 1 section offerte</p>"#,
                r#"</div>"#,
                r#"<div class="toggle-section">"#,
                r#"<p class="toggle-section--header">"#,
                r#"<button class="header-wrapper">"#,
                r#"<span class="header--content-details">"#,
                r#"<span class="item">{code}</span>"#,
                r#"<span class="item"></span>"#,
                r#"<span class="item">En classe</span>"#,
                r#"</span></button></p>"#,
                r#"<div class="toggle-section--content">"#,
                r#"<div class="toggle-section--content-wrapper">"#,
                r#"<strong class="section-cours--nrc">"#,
                r#"<span class="section-cours--nrc-el">NRC</span>"#,
                r#"<span class="section-cours--nrc-el">12345</span>"#,
                r#"</strong></div></div></div></div>"#,
                r#"</body></html>"#,
            ),
            code = code
        )
    }

    fn cached_course(code: &str, years: &[(Season, u16)]) -> CachedCourse {
        CachedCourse {
            course: Course {
                code: code.to_string(),
                title: format!("Cours {code}"),
                credits: Credits::Fixed(3),
                cycle: Cycle::First,
                prerequisites: None,
                equivalents: Vec::new(),
                seasons: years
                    .iter()
                    .map(|(season, _)| {
                        (
                            *season,
                            SeasonOffering {
                                options: Vec::new(),
                            },
                        )
                    })
                    .collect(),
            },
            years: years.iter().copied().collect(),
        }
    }

    fn lock_print() -> std::sync::MutexGuard<'static, ()> {
        print::TEST_STATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ulaval-scraper-course-{name}"));
        // leftovers from an earlier failed run
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        dir
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir)
            .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
    }
}
