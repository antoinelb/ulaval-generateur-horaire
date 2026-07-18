use std::fs;
use std::time::Duration;

use ulaval_scheduler_core::{Catalogue, CatalogueEntry};
use ulaval_scheduler_scraper::catalogue::{self, CatalogueError};
use ulaval_scheduler_scraper::fetch::Fetcher;
use ulaval_scheduler_scraper::parser;
use ulaval_scheduler_scraper::parser::catalogue::CataloguePage;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/catalogue",
);

// (page, displayed total, entry count) — gex_2 is past-the-end with the facet
// active (total still displayed); all_last is the « Aucun résultat » variant,
// which displays no total (ADR 2026-07-page-aucun-resultat-et-total-optionnel).
const PAGES: &[(&str, Option<usize>, usize)] = &[
    ("gex_0", Some(52), 50),
    ("gex_1", Some(52), 2),
    ("gex_2", Some(52), 0),
    ("all_last", None, 0),
];

// The frozen pages are all well-formed, so without this test the error
// closures exist in this binary only as never-run instantiations, which
// llvm-cov reports as phantom missed regions.
#[test]
fn error_paths_stay_errors_through_the_public_api() {
    let drift = parser::catalogue::parse(r#"<div id="resultats"></div>"#);
    assert!(drift.is_err(), "markup drift must be an error");

    let bad_total = parser::catalogue::parse(
        r#"<div class="total-resultats"><p>beaucoup</p></div>"#,
    );
    assert!(bad_total.is_err(), "non-numeric total must be an error");

    let malformed_entries = concat!(
        r#"<div class="total-resultats"><p>3&nbsp;résultats</p></div>"#,
        r#"<a class="cours-element--lien" href="/etudes/cours/a">"#,
        r#"<span class="cours-element--titre">Sans sigle</span></a>"#,
        r#"<a class="cours-element--lien" href="/etudes/cours/b">"#,
        r#"<span class="cours-element--sigle">SANS-TITRE</span></a>"#,
        r#"<a class="cours-element--lien">"#,
        r#"<span class="cours-element--sigle">SANS-LIEN</span>"#,
        r#"<span class="cours-element--titre">Sans lien</span></a>"#,
    );
    let page = parser::catalogue::parse(malformed_entries)
        .unwrap_or_else(|e| panic!("recognized page shape: {e}"));
    assert!(page.entries.is_empty());
    assert_eq!(page.anomalies.len(), 3, "one anomaly per malformed entry");
}

#[test]
fn parses_merges_and_matches_expected_catalogue() {
    let mut entries: Vec<CatalogueEntry> = Vec::new();

    for (page, expected_total, expected_count) in PAGES {
        let html_path = format!("{FIXTURE_DIR}/{page}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let catalogue_page = parser::catalogue::parse(&html)
            .unwrap_or_else(|e| panic!("parse {page}: {e}"));

        assert!(
            catalogue_page.anomalies.is_empty(),
            "anomalies on {page}: {:?}",
            catalogue_page.anomalies
        );
        assert_eq!(
            catalogue_page.total_results, *expected_total,
            "wrong total_results on {page}"
        );
        assert_eq!(
            catalogue_page.entries.len(),
            *expected_count,
            "wrong entry count on {page}"
        );
        entries.extend(catalogue_page.entries);
    }

    let got = serde_json::to_value(Catalogue::from_entries(entries))
        .unwrap_or_else(|e| panic!("serialize parsed catalogue: {e}"));

    let json_path = format!("{FIXTURE_DIR}/gex.json");
    let raw = fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("read {json_path}: {e}"));
    let expected: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {json_path}: {e}"));

    assert_eq!(got, expected, "parsed catalogue differs from gex.json");
}

#[test]
fn the_real_facet_widget_parses_with_gex_present() {
    let html_path = format!("{FIXTURE_DIR}/gex_0.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let (matieres, anomalies) = parser::catalogue::parse_matieres(&html)
        .unwrap_or_else(|e| panic!("parse the real facet widget: {e}"));

    assert!(anomalies.is_empty(), "anomalies: {anomalies:?}");
    assert!(
        matieres.len() > 100,
        "expected the full matière directory, got {}",
        matieres.len()
    );
    assert!(
        matieres
            .iter()
            .any(|m| m.id == "113" && m.label == "GEX - Génie des eaux"),
        "expected 113 = GEX - Génie des eaux in the directory"
    );
}

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
