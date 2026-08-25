use std::time::Duration;

use ulaval_scheduler_scraper::catalogue::{self, CatalogueError};
use ulaval_scheduler_scraper::fetch::Fetcher;
use ulaval_scheduler_scraper::parser::catalogue::CataloguePage;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn partitioning_a_no_results_catalogue_yields_it_empty() {
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
async fn an_unrecognized_first_page_is_a_parse_error() {
    let server = MockServer::start().await;
    mount_page(&server, 0, r#"<div id="resultats"></div>"#.to_string()).await;

    let error = scrape_catalogue(&server)
        .await
        .expect_err("markup drift on the first page must fail");

    assert!(
        matches!(error, CatalogueError::Parse(_)),
        "expected Parse error, got {error:?}"
    );
}

#[tokio::test]
async fn a_partitioned_catalogue_merges_all_matieres() {
    let server = MockServer::start().await;
    // 1 entry on the unfiltered first page against a total of 3 forces the
    // partitioned path; the union of the facets is the catalogue
    mount_page(
        &server,
        0,
        page_html(3, &["GEX-1000"]) + &facet_html(&["7", "113"]),
    )
    .await;
    mount_matiere_page(
        &server,
        "7",
        0,
        page_html(2, &["ACT-1000", "ACT-2000"]),
    )
    .await;
    mount_matiere_page(&server, "113", 0, page_html(1, &["GEX-1000"])).await;

    let page = scrape_catalogue(&server)
        .await
        .unwrap_or_else(|e| panic!("scrape two matières: {e}"));

    // partitions land in completion order and duplicates survive: sorting
    // and dedup are the artifact's job (`Catalogue::from_entries` in cli)
    let mut codes: Vec<&str> = page
        .entries
        .iter()
        .map(|entry| entry.code.as_str())
        .collect();
    codes.sort_unstable();
    assert_eq!(codes, ["ACT-1000", "ACT-2000", "GEX-1000"]);
    assert!(page.anomalies.is_empty());
    assert_eq!(page.total_results, Some(3));
}

#[tokio::test]
async fn a_multi_page_partition_is_reconciled_quietly() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        page_html(3, &["GEX-1000"]) + &facet_html(&["113"]),
    )
    .await;
    mount_matiere_page(
        &server,
        "113",
        0,
        page_html(3, &["GEX-1000", "GEX-2000"]),
    )
    .await;
    mount_matiere_page(&server, "113", 1, page_html(3, &["GEX-3000"])).await;

    let page = scrape_catalogue(&server)
        .await
        .unwrap_or_else(|e| panic!("scrape a paginated matière: {e}"));

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
async fn a_failing_matiere_names_it_and_stops_the_run() {
    let server = MockServer::start().await;
    mount_page(
        &server,
        0,
        page_html(3, &["ACT-1000"]) + &facet_html(&["7"]),
    )
    .await;
    Mock::given(method("GET"))
        .and(query_param("matieres[7]", "7"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .with_priority(1)
        .mount(&server)
        .await;

    let error = scrape_catalogue(&server)
        .await
        .expect_err("a 404 matière must fail");

    assert!(
        matches!(
            &error,
            CatalogueError::Partition { matiere, source }
                if matiere == "M7 - Matière 7"
                    && matches!(**source, CatalogueError::Fetch(_))
        ),
        "expected Partition wrapping a Fetch error, got {error:?}"
    );
}

#[tokio::test]
async fn a_first_page_without_the_facet_widget_is_a_parse_error() {
    let server = MockServer::start().await;
    // total 3 forces partitioning, but there is no widget to partition by
    mount_page(&server, 0, page_html(3, &["GEX-1000"])).await;

    let error = scrape_catalogue(&server)
        .await
        .expect_err("a missing facet widget must fail");

    assert!(
        matches!(error, CatalogueError::Parse(_)),
        "expected Parse error, got {error:?}"
    );
}

async fn scrape_catalogue(
    server: &MockServer,
) -> Result<CataloguePage, CatalogueError> {
    let fetcher = Fetcher::new(Duration::ZERO, Duration::ZERO)
        .unwrap_or_else(|e| panic!("build fetcher: {e}"));
    catalogue::scrape(&fetcher, &server.uri()).await
}

pub(crate) async fn mount_page(
    server: &MockServer,
    page: usize,
    html: String,
) {
    Mock::given(method("GET"))
        .and(query_param("page", page.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .expect(1)
        .mount(server)
        .await;
}

pub(crate) fn page_html(total: usize, codes: &[&str]) -> String {
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

// wiremock matches on decoded query keys, so `matieres[7]` here only
// matches the bracketed encoded form the code must emit — the flat
// `matieres=7` the site silently ignores would leave these mocks unmatched
pub(crate) async fn mount_matiere_page(
    server: &MockServer,
    id: &str,
    page: usize,
    html: String,
) {
    Mock::given(method("GET"))
        .and(query_param(format!("matieres[{id}]"), id))
        .and(query_param("page", page.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .expect(1)
        // beat the plain page-N mocks, which also match filtered requests
        .with_priority(1)
        .mount(server)
        .await;
}

pub(crate) fn facet_html(ids: &[&str]) -> String {
    let mut html = String::new();
    for id in ids {
        html.push_str(&format!(
            concat!(
                r#"<input type="checkbox" id="edit-matieres-{id}--2" "#,
                r#"name="matieres[{id}]" value="{id}" "#,
                r#"class="form-checkbox hidden-checkbox">"#,
                r#"<label for="edit-matieres-{id}--2" class="option">"#,
                r#"<svg></svg>M{id} - Matière {id}</label>"#,
            ),
            id = id
        ));
    }
    html
}

fn no_results_html() -> String {
    r#"<div class="resultats--offre-etudes"><p>Aucun résultat</p></div>"#
        .to_string()
}
