use crate::parser::ParseError;
use ulaval_scheduler_core::CatalogueEntry;

#[derive(Debug)]
pub struct CataloguePage {
    pub entries: Vec<CatalogueEntry>,
    pub anomalies: Vec<ParseError>,
    // None on the « Aucun résultat » variant, which displays no count
    pub total_results: Option<usize>,
}

pub fn parse(html: &str) -> Result<CataloguePage, ParseError> {
    let doc = scraper::Html::parse_document(html);

    let total_results = get_total_results(&doc)?;
    let (entries, anomalies) = get_catalogues(&doc);

    Ok(CataloguePage {
        entries,
        anomalies,
        total_results,
    })
}

fn get_total_results(
    doc: &scraper::Html,
) -> Result<Option<usize>, ParseError> {
    let selector_str = "div.total-resultats p";
    let no_results_selector_str = "div.resultats--offre-etudes p";
    let selector = scraper::Selector::parse(selector_str)
        .expect("Static selector is valid");
    let no_results_selector =
        scraper::Selector::parse(no_results_selector_str)
            .expect("Static selector is valid");

    let text = doc
        .select(&selector)
        .next()
        .map(|element| element.text().collect::<String>());

    match text {
        Some(text) => {
            let total = text
                .split_whitespace()
                .next()
                .and_then(|element| element.parse::<usize>().ok())
                .ok_or_else(|| ParseError::MalformedEntry {
                    selector: selector_str.to_string(),
                    raw: text,
                })?;
            Ok(Some(total))
        }
        None => {
            let is_no_results = doc
                .select(&no_results_selector)
                .next()
                .map(|element| element.text().collect::<String>())
                .is_some_and(|text| text.trim() == "Aucun résultat");
            if is_no_results {
                Ok(None)
            } else {
                Err(ParseError::MissingElement {
                    selector: format!(
                        "{selector_str} (nor {no_results_selector_str})"
                    ),
                })
            }
        }
    }
}

fn get_catalogues(
    doc: &scraper::Html,
) -> (Vec<CatalogueEntry>, Vec<ParseError>) {
    let selector_str = "a.cours-element--lien";
    let selector = scraper::Selector::parse(selector_str)
        .expect("Static selector is valid");

    let mut entries: Vec<CatalogueEntry> = Vec::new();
    let mut anomalies: Vec<ParseError> = Vec::new();

    for element in doc.select(&selector) {
        match parse_catalogue(&element, selector_str) {
            Ok(entry) => entries.push(entry),
            Err(anomaly) => anomalies.push(anomaly),
        }
    }

    (entries, anomalies)
}

fn parse_catalogue(
    element: &scraper::ElementRef,
    selector_str: &str,
) -> Result<CatalogueEntry, ParseError> {
    let code_selector_str = "span.cours-element--sigle";
    let title_selector_str = "span.cours-element--titre";
    let code_selector = scraper::Selector::parse(code_selector_str)
        .expect("Static selector is valid");
    let title_selector = scraper::Selector::parse(title_selector_str)
        .expect("Static selector is valid");

    let code = element
        .select(&code_selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: code_selector_str.to_string(),
            raw: element.html(),
        })?;
    let title = element
        .select(&title_selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: title_selector_str.to_string(),
            raw: element.html(),
        })?;
    let url = element
        .value()
        .attr("href")
        .map(|url| format!("https://www.ulaval.ca{}", url))
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: selector_str.to_string(),
            raw: element.html(),
        })?;

    Ok(CatalogueEntry { code, title, url })
}

// excluded from coverage: assertion failure branches only run when a test
// fails, so test code can never reach 100% of itself
#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_with_neither_marker_is_drift() {
        let html = r#"<div id="resultats"></div>"#;

        let result = parse(html);

        assert!(
            matches!(result, Err(ParseError::MissingElement { .. })),
            "expected MissingElement, got {result:?}"
        );
    }

    #[test]
    fn results_container_without_no_results_text_is_drift() {
        // The container class exists on every page; if total-resultats
        // drifts, its presence alone must not read as "no results".
        let html = concat!(
            r#"<div class="resultats--offre-etudes">"#,
            "<p>Quelque chose d'autre</p></div>",
        );

        let result = parse(html);

        assert!(
            matches!(result, Err(ParseError::MissingElement { .. })),
            "expected MissingElement, got {result:?}"
        );
    }

    #[test]
    fn entry_without_code_is_an_anomaly_not_a_failure() {
        let html = concat!(
            r#"<div class="total-resultats"><p>2&nbsp;résultats</p></div>"#,
            r#"<a class="cours-element--lien" href="/etudes/cours/xxx-1000">"#,
            r#"<span class="cours-element--titre">Sans sigle</span></a>"#,
            r#"<a class="cours-element--lien" href="/etudes/cours/gex-1000">"#,
            r#"<span class="cours-element--sigle">GEX-1000</span>"#,
            r#"<span class="cours-element--titre">Valide</span></a>"#,
        );

        let page = parse(html).expect("page shape is recognized");

        assert_eq!(page.entries.len(), 1, "valid entry still parsed");
        assert_eq!(page.entries[0].code, "GEX-1000");
        assert_eq!(page.anomalies.len(), 1);
        let anomaly = &page.anomalies[0];
        assert!(
            matches!(
                anomaly,
                ParseError::MalformedEntry { raw, .. } if raw.contains("xxx-1000")
            ),
            "anomaly carries the offending entry HTML, got {anomaly:?}"
        );
    }

    #[test]
    fn entry_without_title_is_an_anomaly_not_a_failure() {
        let html = concat!(
            r#"<div class="total-resultats"><p>1&nbsp;résultat</p></div>"#,
            r#"<a class="cours-element--lien" href="/etudes/cours/gex-1000">"#,
            r#"<span class="cours-element--sigle">GEX-1000</span></a>"#,
        );

        let page = parse(html).expect("page shape is recognized");

        assert!(page.entries.is_empty());
        assert_eq!(page.anomalies.len(), 1);
        let anomaly = &page.anomalies[0];
        assert!(
            matches!(
                anomaly,
                ParseError::MalformedEntry { selector, .. }
                    if selector.contains("titre")
            ),
            "anomaly names the title selector, got {anomaly:?}"
        );
    }

    #[test]
    fn entry_without_href_is_an_anomaly_not_a_failure() {
        let html = concat!(
            r#"<div class="total-resultats"><p>1&nbsp;résultat</p></div>"#,
            r#"<a class="cours-element--lien">"#,
            r#"<span class="cours-element--sigle">GEX-1000</span>"#,
            r#"<span class="cours-element--titre">Sans lien</span></a>"#,
        );

        let page = parse(html).expect("page shape is recognized");

        assert!(page.entries.is_empty());
        assert_eq!(page.anomalies.len(), 1);
    }

    #[test]
    fn malformed_total_is_an_error_carrying_the_raw_text() {
        let html = r#"<div class="total-resultats"><p>beaucoup</p></div>"#;

        let result = parse(html);

        assert!(
            matches!(
                &result,
                Err(ParseError::MalformedEntry { raw, .. }) if raw.contains("beaucoup")
            ),
            "expected MalformedEntry with raw text, got {result:?}"
        );
    }
}
