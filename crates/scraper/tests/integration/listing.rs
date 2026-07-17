use ulaval_scheduler_core::{Catalogue, CatalogueEntry};
use ulaval_scheduler_scraper::parser::listing;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/listing",
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
    let drift = listing::parse(r#"<div id="resultats"></div>"#);
    assert!(drift.is_err(), "markup drift must be an error");

    let bad_total = listing::parse(
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
    let page = listing::parse(malformed_entries)
        .unwrap_or_else(|e| panic!("recognized page shape: {e}"));
    assert!(page.entries.is_empty());
    assert_eq!(page.anomalies.len(), 3, "one anomaly per malformed entry");
}

#[test]
fn parses_merges_and_matches_expected_catalogue() {
    let mut entries: Vec<CatalogueEntry> = Vec::new();

    for (page, expected_total, expected_count) in PAGES {
        let html_path = format!("{FIXTURE_DIR}/{page}.html");
        let html = std::fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let listing_page = listing::parse(&html)
            .unwrap_or_else(|e| panic!("parse {page}: {e}"));

        assert!(
            listing_page.anomalies.is_empty(),
            "anomalies on {page}: {:?}",
            listing_page.anomalies
        );
        assert_eq!(
            listing_page.total_results, *expected_total,
            "wrong total_results on {page}"
        );
        assert_eq!(
            listing_page.entries.len(),
            *expected_count,
            "wrong entry count on {page}"
        );
        entries.extend(listing_page.entries);
    }

    entries.sort_by(|a, b| a.code.cmp(&b.code));
    entries.dedup_by(|a, b| a.code == b.code);

    let got = serde_json::to_value(Catalogue { courses: entries })
        .unwrap_or_else(|e| panic!("serialize parsed catalogue: {e}"));

    let json_path = format!("{FIXTURE_DIR}/gex.json");
    let raw = std::fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("read {json_path}: {e}"));
    let expected: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse {json_path}: {e}"));

    assert_eq!(got, expected, "parsed catalogue differs from gex.json");
}
