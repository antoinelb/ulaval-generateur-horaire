use futures::stream::{self, StreamExt, TryStreamExt};

use crate::fetch::{FetchError, Fetcher};
use crate::parser::catalogue::CataloguePage;
use crate::parser::{self, ParseError};
use crate::print;

const n_concurrent: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum CatalogueError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("The total number of entries is different than what was expected from the `total_results` field: {got} vs {expected}")]
    TotalMismatch { got: usize, expected: usize },
    #[error("The `total_results` is different on page {url} than the first page's: {got:?} vs {expected}")]
    PageTotalDrift {
        url: String,
        got: Option<usize>,
        expected: usize,
    },
    #[error("The calculated number of pages is larger than the allowed limit: {calculated} vs {cap}")]
    TooManyPages { calculated: usize, cap: usize },
    #[error("The first page didn't have any entries even though the total number should be {total}")]
    EmptyFirstPage { total: usize },
    #[error("Scraping matière '{matiere}': {source}")]
    Partition {
        matiere: String,
        #[source]
        source: Box<CatalogueError>,
    },
}

pub async fn scrape(
    fetcher: &Fetcher,
    base_url: &str,
) -> Result<CataloguePage, CatalogueError> {
    let task = print::task(
        "Fetching catalogue first page...",
        "Fetched catalogue first page.",
    );
    let first_page_html = fetcher.fetch(&get_page_url(base_url, 0)).await?;
    let first_page = parser::catalogue::parse(&first_page_html)?;
    task.done();

    // « Aucun résultat » : an empty catalogue has nothing to partition
    let Some(total) = first_page.total_results else {
        return Ok(first_page);
    };

    // the whole catalogue fits on the first page: already complete,
    // partitioning would only repeat it
    if total == (first_page.entries.len() + first_page.anomalies.len()) {
        return Ok(first_page);
    }

    // past this point the first page only provides the facet directory;
    // its entries reappear under their matière
    let (matieres, mut anomalies) =
        parser::catalogue::parse_matieres(&first_page_html)?;

    let task = print::progress_task(
        "Scraping subjects...",
        "Scraped subjects.",
        matieres.len(),
    );
    let progress = &task;
    let pages: Vec<CataloguePage> = stream::iter(matieres)
        .map(|matiere| async move {
            let url = get_matiere_url(base_url, &matiere.id);
            let page =
                scrape_partition(fetcher, &url).await.map_err(|source| {
                    CatalogueError::Partition {
                        matiere: matiere.label,
                        source: Box::new(source),
                    }
                })?;
            progress.increment();
            Ok::<CataloguePage, CatalogueError>(page)
        })
        .buffer_unordered(n_concurrent)
        .try_collect()
        .await?;
    task.done();

    let mut entries = Vec::new();
    for page in pages {
        entries.extend(page.entries);
        anomalies.extend(page.anomalies);
    }
    let total_results = Some(entries.len());
    Ok(CataloguePage {
        entries,
        anomalies,
        total_results,
    })
}

async fn scrape_partition(
    fetcher: &Fetcher,
    base_url: &str,
) -> Result<CataloguePage, CatalogueError> {
    let first_page_html = fetcher.fetch(&get_page_url(base_url, 0)).await?;
    let first_page = parser::catalogue::parse(&first_page_html)?;

    match first_page.total_results {
        None => Ok(first_page),
        Some(total)
            if total
                == (first_page.entries.len() + first_page.anomalies.len()) =>
        {
            Ok(first_page)
        }
        Some(total) => {
            let n_pages = calculate_number_of_pages(
                total,
                first_page.entries.len() + first_page.anomalies.len(),
            )?;
            let mut pages: Vec<CataloguePage> = stream::iter(1..n_pages)
                .map(|page| async move {
                    let url = get_page_url(base_url, page);
                    let html = fetcher.fetch(&url).await?;
                    let catalogue_page = parser::catalogue::parse(&html)?;
                    validate_page_total(catalogue_page, url, total)
                })
                .buffer_unordered(4) // subjects rarely have more than a couple of pages
                .try_collect()
                .await?;
            pages.push(first_page);
            combine_pages(pages, total)
        }
    }
}

fn get_page_url(url: &str, page: usize) -> String {
    if url.contains("?") {
        format!("{}&page={}", url, page)
    } else {
        format!("{}?page={}", url, page)
    }
}

fn get_matiere_url(url: &str, id: &str) -> String {
    if url.contains("?") {
        format!("{url}&matieres%5B{id}%5D={id}")
    } else {
        format!("{url}?matieres%5B{id}%5D={id}")
    }
}

fn validate_page_total(
    catalogue_page: CataloguePage,
    url: String,
    total: usize,
) -> Result<CataloguePage, CatalogueError> {
    match catalogue_page.total_results {
        Some(page_total) => {
            if page_total != total {
                Err(CatalogueError::PageTotalDrift {
                    url,
                    got: Some(page_total),
                    expected: total,
                })
            } else {
                Ok(catalogue_page)
            }
        }
        // « Aucun résultat » past the computed upper bound: the page-count
        // arithmetic over-estimates when page 0 under-states the site's
        // page size (199 real pages vs 204 computed on the live run); the
        // count reconciliation after merging still guarantees completeness
        None if catalogue_page.entries.is_empty()
            && catalogue_page.anomalies.is_empty() =>
        {
            Ok(catalogue_page)
        }
        None => Err(CatalogueError::PageTotalDrift {
            url,
            got: None,
            expected: total,
        }),
    }
}

fn calculate_number_of_pages(
    total: usize,
    n_entries: usize,
) -> Result<usize, CatalogueError> {
    const max_pages_cap: usize = 1000;
    if n_entries == 0 {
        Err(CatalogueError::EmptyFirstPage { total })
    } else {
        let n_pages = total.div_ceil(n_entries);
        if n_pages > max_pages_cap {
            Err(CatalogueError::TooManyPages {
                calculated: n_pages,
                cap: max_pages_cap,
            })
        } else {
            Ok(n_pages)
        }
    }
}

fn combine_pages(
    pages: Vec<CataloguePage>,
    total: usize,
) -> Result<CataloguePage, CatalogueError> {
    let mut entries = Vec::new();
    let mut anomalies = Vec::new();
    for page in pages {
        entries.extend(page.entries);
        anomalies.extend(page.anomalies);
    }

    let n_entries = entries.len() + anomalies.len();
    if n_entries != total {
        Err(CatalogueError::TotalMismatch {
            got: n_entries,
            expected: total,
        })
    } else {
        Ok(CataloguePage {
            entries,
            anomalies,
            total_results: Some(total),
        })
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ulaval_scheduler_core::CatalogueEntry;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[test]
    fn page_url_extends_an_existing_query_string() {
        assert_eq!(
            get_page_url("https://ulaval.ca/etudes/cours?matieres=113", 3),
            "https://ulaval.ca/etudes/cours?matieres=113&page=3"
        );
    }

    #[test]
    fn page_url_starts_the_query_string_when_absent() {
        assert_eq!(
            get_page_url("https://ulaval.ca/etudes/cours", 0),
            "https://ulaval.ca/etudes/cours?page=0"
        );
    }

    #[test]
    fn number_of_pages_rounds_up_the_partial_last_page() {
        let n_pages = calculate_number_of_pages(52, 50)
            .unwrap_or_else(|e| panic!("52 results over pages of 50: {e}"));

        assert_eq!(n_pages, 2);
    }

    #[test]
    fn number_of_pages_is_exact_when_pages_divide_evenly() {
        let n_pages = calculate_number_of_pages(100, 50)
            .unwrap_or_else(|e| panic!("100 results over pages of 50: {e}"));

        assert_eq!(n_pages, 2);
    }

    #[test]
    fn empty_first_page_with_a_positive_total_is_a_contradiction() {
        let result = calculate_number_of_pages(5, 0);

        assert!(
            matches!(result, Err(CatalogueError::EmptyFirstPage { total: 5 })),
            "expected EmptyFirstPage, got {result:?}"
        );
    }

    #[test]
    fn page_count_above_the_cap_is_an_error() {
        let result = calculate_number_of_pages(10_000, 1);

        assert!(
            matches!(
                result,
                Err(CatalogueError::TooManyPages {
                    calculated: 10_000,
                    cap: 1000,
                })
            ),
            "expected TooManyPages, got {result:?}"
        );
    }

    #[test]
    fn combining_pages_merges_entries_and_anomalies() {
        let pages =
            vec![page(&["GEX-1000", "GEX-2000"], 0), page(&["GEX-3000"], 1)];

        let combined = combine_pages(pages, 4)
            .unwrap_or_else(|e| panic!("counts add up to the total: {e}"));

        assert_eq!(combined.entries.len(), 3);
        assert_eq!(combined.anomalies.len(), 1);
        assert_eq!(combined.total_results, Some(4));
    }

    #[test]
    fn combined_count_missing_the_total_is_a_mismatch() {
        let pages = vec![page(&["GEX-1000"], 0)];

        let result = combine_pages(pages, 2);

        assert!(
            matches!(
                result,
                Err(CatalogueError::TotalMismatch {
                    got: 1,
                    expected: 2
                })
            ),
            "expected TotalMismatch, got {result:?}"
        );
    }

    #[test]
    fn matiere_url_uses_the_bracketed_drupal_form() {
        // the flat `?matieres=113` is silently ignored by the site — this
        // exact encoded string is what makes the filter work
        assert_eq!(
            get_matiere_url("https://ulaval.ca/etudes/cours", "113"),
            "https://ulaval.ca/etudes/cours?matieres%5B113%5D=113"
        );
    }

    #[test]
    fn matiere_url_extends_an_existing_query_string() {
        assert_eq!(
            get_matiere_url("https://ulaval.ca/etudes/cours?search=", "9"),
            "https://ulaval.ca/etudes/cours?search=&matieres%5B9%5D=9"
        );
    }

    // -- the per-URL engine against wiremock --------------------------------

    #[tokio::test]
    async fn scraping_a_no_results_page_yields_an_empty_catalogue() {
        let server = MockServer::start().await;
        mount_page(&server, 0, no_results_html()).await;

        let page = scrape_catalogue(&server)
            .await
            .unwrap_or_else(|e| panic!("scrape the empty catalogue: {e}"));

        assert!(page.entries.is_empty());
        assert!(page.anomalies.is_empty());
        assert_eq!(page.total_results, None);
    }

    #[tokio::test]
    async fn a_single_page_catalogue_needs_no_fan_out() {
        let server = MockServer::start().await;
        // the .expect(1) inside mount_page doubles as proof that no page
        // beyond 0 is requested: any extra request would 404 and fail
        mount_page(&server, 0, page_html(2, &["GEX-1000", "GEX-2000"])).await;

        let page = scrape_catalogue(&server)
            .await
            .unwrap_or_else(|e| panic!("scrape a single page: {e}"));

        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.total_results, Some(2));
    }

    #[tokio::test]
    async fn a_multi_page_catalogue_is_merged_and_reconciled() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        mount_page(&server, 1, page_html(3, &["GEX-3000"])).await;

        let page = scrape_catalogue(&server)
            .await
            .unwrap_or_else(|e| panic!("scrape two pages: {e}"));

        // buffer_unordered yields pages in completion order, so sort before
        // comparing; ordering is the orchestrator's (merge/sort/dedup) job
        let mut codes: Vec<&str> = page
            .entries
            .iter()
            .map(|entry| entry.code.as_str())
            .collect();
        codes.sort_unstable();
        assert_eq!(codes, ["GEX-1000", "GEX-2000", "GEX-3000"]);
        assert_eq!(page.total_results, Some(3));
    }

    #[tokio::test]
    async fn a_failing_first_page_is_a_fetch_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("a 404 first page must fail");

        assert!(
            matches!(error, CatalogueError::Fetch(_)),
            "expected Fetch error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn an_unrecognized_first_page_is_a_parse_error() {
        let server = MockServer::start().await;
        mount_page(&server, 0, r#"<div id="resultats"></div>"#.to_string())
            .await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("markup drift on the first page must fail");

        assert!(
            matches!(error, CatalogueError::Parse(_)),
            "expected Parse error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn a_failing_later_page_stops_the_run() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        Mock::given(method("GET"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("a 404 later page must fail");

        assert!(
            matches!(error, CatalogueError::Fetch(_)),
            "expected Fetch error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn an_unrecognized_later_page_stops_the_run() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        mount_page(&server, 1, r#"<div id="resultats"></div>"#.to_string())
            .await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("markup drift on a later page must fail");

        assert!(
            matches!(error, CatalogueError::Parse(_)),
            "expected Parse error, got {error:?}"
        );
    }

    #[tokio::test]
    async fn a_drifting_total_on_a_later_page_stops_the_run() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        mount_page(&server, 1, page_html(4, &["GEX-3000"])).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("a drifting total must fail");

        assert!(
            matches!(
                &error,
                CatalogueError::PageTotalDrift {
                    url,
                    got: Some(4),
                    expected: 3,
                } if url.contains("page=1")
            ),
            "expected PageTotalDrift naming page 1, got {error:?}"
        );
    }

    #[tokio::test]
    async fn an_overestimated_page_count_tolerates_a_no_results_tail() {
        // page 0 under-states the site's true page size (2 vs 3), so the
        // computed page count over-shoots and the extra page resolves to
        // « Aucun résultat » — the shape the live catalogue exhibits; ADR
        // 2026-07-tolerance-des-pages-aucun-resultat-du-fan-out
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(5, &["GEX-1000", "GEX-2000"])).await;
        mount_page(
            &server,
            1,
            page_html(5, &["GEX-3000", "GEX-4000", "GEX-5000"]),
        )
        .await;
        mount_page(&server, 2, no_results_html()).await;

        let page = scrape_catalogue(&server)
            .await
            .unwrap_or_else(|e| panic!("scrape past the real last page: {e}"));

        assert_eq!(page.entries.len(), 5);
        assert!(page.anomalies.is_empty());
        assert_eq!(page.total_results, Some(5));
    }

    #[tokio::test]
    async fn a_no_results_tail_hiding_missing_entries_is_a_total_mismatch() {
        // tolerance must not become silent truncation: an empty tail page
        // that leaves the merged count short still fails at reconciliation
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        mount_page(&server, 1, no_results_html()).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("2 merged entries against a total of 3 must fail");

        assert!(
            matches!(
                error,
                CatalogueError::TotalMismatch {
                    got: 2,
                    expected: 3
                }
            ),
            "expected TotalMismatch, got {error:?}"
        );
    }

    #[tokio::test]
    async fn a_no_results_page_carrying_entries_is_drift() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        let no_results_with_entry = format!(
            "{}{}",
            no_results_html(),
            concat!(
                r#"<a class="cours-element--lien" href="/etudes/cours/gex-3000">"#,
                r#"<span class="cours-element--sigle">GEX-3000</span>"#,
                r#"<span class="cours-element--titre">Cours GEX-3000</span></a>"#,
            )
        );
        mount_page(&server, 1, no_results_with_entry).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("entries on a « Aucun résultat » page must fail");

        assert!(
            matches!(
                &error,
                CatalogueError::PageTotalDrift {
                    url,
                    got: None,
                    expected: 3,
                } if url.contains("page=1")
            ),
            "expected PageTotalDrift naming page 1, got {error:?}"
        );
    }

    #[tokio::test]
    async fn a_no_results_page_carrying_an_anomaly_is_drift() {
        // « Aucun résultat » markup carrying a malformed entry: 0 entries
        // but 1 anomaly — must not read as a tolerated empty tail
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(3, &["GEX-1000", "GEX-2000"])).await;
        let no_results_with_anomaly = format!(
            "{}{}",
            no_results_html(),
            concat!(
                r#"<a class="cours-element--lien" href="/etudes/cours/x">"#,
                r#"<span class="cours-element--titre">Sans sigle</span></a>"#,
            )
        );
        mount_page(&server, 1, no_results_with_anomaly).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("an anomaly on a « Aucun résultat » page must fail");

        assert!(
            matches!(
                &error,
                CatalogueError::PageTotalDrift {
                    url,
                    got: None,
                    expected: 3,
                } if url.contains("page=1")
            ),
            "expected PageTotalDrift naming page 1, got {error:?}"
        );
    }

    #[tokio::test]
    async fn entries_missing_across_pages_are_a_total_mismatch() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(4, &["GEX-1000", "GEX-2000"])).await;
        mount_page(&server, 1, page_html(4, &["GEX-3000"])).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("3 merged entries against a total of 4 must fail");

        assert!(
            matches!(
                error,
                CatalogueError::TotalMismatch {
                    got: 3,
                    expected: 4
                }
            ),
            "expected TotalMismatch, got {error:?}"
        );
    }

    #[tokio::test]
    async fn an_empty_first_page_with_a_positive_total_is_a_contradiction() {
        let server = MockServer::start().await;
        mount_page(&server, 0, page_html(5, &[])).await;

        let error = scrape_catalogue(&server)
            .await
            .expect_err("an empty first page announcing 5 results must fail");

        assert!(
            matches!(error, CatalogueError::EmptyFirstPage { total: 5 }),
            "expected EmptyFirstPage, got {error:?}"
        );
    }

    async fn scrape_catalogue(
        server: &MockServer,
    ) -> Result<CataloguePage, CatalogueError> {
        // zero intervals: throttle timing is unit-tested on a virtual
        // clock; these tests only assert orchestration and must stay fast
        let fetcher = Fetcher::new(Duration::ZERO, Duration::ZERO)
            .unwrap_or_else(|e| panic!("build fetcher: {e}"));
        scrape_partition(&fetcher, &server.uri()).await
    }

    async fn mount_page(server: &MockServer, page: usize, html: String) {
        Mock::given(method("GET"))
            .and(query_param("page", page.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_string(html))
            .expect(1)
            .mount(server)
            .await;
    }

    fn page_html(total: usize, codes: &[&str]) -> String {
        let mut html = format!(
            r#"<div class="total-resultats"><p>{total} résultats</p></div>"#
        );
        for code in codes {
            html.push_str(&format!(
                concat!(
                    r#"<a class="cours-element--lien" href="/etudes/cours/{code}">"#,
                    r#"<span class="cours-element--sigle">{code}</span>"#,
                    r#"<span class="cours-element--titre">Cours {code}</span></a>"#,
                ),
                code = code
            ));
        }
        html
    }

    fn no_results_html() -> String {
        r#"<div class="resultats--offre-etudes"><p>Aucun résultat</p></div>"#
            .to_string()
    }

    fn page(codes: &[&str], n_anomalies: usize) -> CataloguePage {
        CataloguePage {
            entries: codes
                .iter()
                .map(|code| CatalogueEntry {
                    code: code.to_string(),
                    title: format!("Cours {code}"),
                    url: format!("https://ulaval.ca/etudes/cours/{code}"),
                })
                .collect(),
            anomalies: (0..n_anomalies)
                .map(|i| ParseError::MissingElement {
                    selector: format!("selector-{i}"),
                })
                .collect(),
            total_results: Some(codes.len() + n_anomalies),
        }
    }
}
