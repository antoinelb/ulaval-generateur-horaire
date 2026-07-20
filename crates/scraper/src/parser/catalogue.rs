use std::collections::HashMap;
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use ulaval_scheduler_core::CatalogueEntry;

use crate::parser::ParseError;

const MATIERE_INPUT_CSS: &str = r#"input.form-checkbox[name^="matieres["]"#;
static MATIERE_INPUT: LazyLock<Selector> =
    LazyLock::new(|| sel(MATIERE_INPUT_CSS));
static MATIERE_LABEL: LazyLock<Selector> =
    LazyLock::new(|| sel("label.option"));

const TOTAL_CSS: &str = "div.total-resultats p";
static TOTAL: LazyLock<Selector> = LazyLock::new(|| sel(TOTAL_CSS));
const NO_RESULTS_CSS: &str = "div.resultats--offre-etudes p";
static NO_RESULTS: LazyLock<Selector> = LazyLock::new(|| sel(NO_RESULTS_CSS));

const ENTRY_CSS: &str = "a.cours-element--lien";
static ENTRY: LazyLock<Selector> = LazyLock::new(|| sel(ENTRY_CSS));

const ENTRY_CODE_CSS: &str = "span.cours-element--sigle";
static ENTRY_CODE: LazyLock<Selector> = LazyLock::new(|| sel(ENTRY_CODE_CSS));
const ENTRY_TITLE_CSS: &str = "span.cours-element--titre";
static ENTRY_TITLE: LazyLock<Selector> =
    LazyLock::new(|| sel(ENTRY_TITLE_CSS));

#[derive(Debug, Clone)]
pub struct Matiere {
    pub id: String,
    pub label: String,
}

#[derive(Debug)]
pub struct CataloguePage {
    pub entries: Vec<CatalogueEntry>,
    pub anomalies: Vec<ParseError>,
    pub total_results: Option<usize>,
}

pub fn parse(html: &str) -> Result<CataloguePage, ParseError> {
    let doc = Html::parse_document(html);

    let total_results = get_total_results(&doc)?;
    let (entries, anomalies) = get_catalogues(&doc);

    Ok(CataloguePage {
        entries,
        anomalies,
        total_results,
    })
}

pub fn parse_matieres(
    html: &str,
) -> Result<(Vec<Matiere>, Vec<ParseError>), ParseError> {
    let doc = Html::parse_document(html);
    let labels: HashMap<String, String> = doc
        .select(&MATIERE_LABEL)
        .filter_map(|label| {
            let for_attr = label.value().attr("for")?;
            let text = label.text().collect::<String>().trim().to_string();
            Some((for_attr.to_string(), text))
        })
        .collect();

    let mut matieres = Vec::new();
    let mut anomalies = Vec::new();
    for input in doc.select(&MATIERE_INPUT) {
        match parse_matiere(&input, &labels, MATIERE_INPUT_CSS) {
            Ok(matiere) => matieres.push(matiere),
            Err(anomaly) => anomalies.push(anomaly),
        }
    }

    if matieres.is_empty() && anomalies.is_empty() {
        // no widget at all is markup drift, not an empty facet: refuse to
        // partition the catalogue into nothing
        Err(ParseError::MissingElement {
            selector: MATIERE_INPUT_CSS.to_string(),
        })
    } else {
        Ok((matieres, anomalies))
    }
}

fn get_total_results(doc: &Html) -> Result<Option<usize>, ParseError> {
    let text = doc
        .select(&TOTAL)
        .next()
        .map(|element| element.text().collect::<String>());

    match text {
        Some(text) => {
            let total = text
                .split_whitespace()
                .next()
                .and_then(|element| element.parse::<usize>().ok())
                .ok_or_else(|| ParseError::MalformedEntry {
                    selector: TOTAL_CSS.to_string(),
                    raw: text,
                })?;
            Ok(Some(total))
        }
        None => {
            let is_no_results = doc
                .select(&NO_RESULTS)
                .next()
                .map(|element| element.text().collect::<String>())
                .is_some_and(|text| text.trim() == "Aucun résultat");
            if is_no_results {
                Ok(None)
            } else {
                Err(ParseError::MissingElement {
                    selector: format!("{TOTAL_CSS} (nor {NO_RESULTS_CSS})"),
                })
            }
        }
    }
}

fn get_catalogues(doc: &Html) -> (Vec<CatalogueEntry>, Vec<ParseError>) {
    let mut entries: Vec<CatalogueEntry> = Vec::new();
    let mut anomalies: Vec<ParseError> = Vec::new();

    for element in doc.select(&ENTRY) {
        match parse_catalogue(&element, ENTRY_CSS) {
            Ok(entry) => entries.push(entry),
            Err(anomaly) => anomalies.push(anomaly),
        }
    }

    (entries, anomalies)
}

fn parse_catalogue(
    element: &ElementRef,
    selector_str: &str,
) -> Result<CatalogueEntry, ParseError> {
    let code = element
        .select(&ENTRY_CODE)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: ENTRY_CODE_CSS.to_string(),
            raw: element.html(),
        })?;
    let title = element
        .select(&ENTRY_TITLE)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: ENTRY_TITLE_CSS.to_string(),
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

fn parse_matiere(
    input: &ElementRef,
    labels: &HashMap<String, String>,
    selector_str: &str,
) -> Result<Matiere, ParseError> {
    let id = input.value().attr("value").ok_or_else(|| {
        ParseError::MalformedEntry {
            selector: format!("{selector_str}[value]"),
            raw: input.html(),
        }
    })?;
    let dom_id = input.value().attr("id").ok_or_else(|| {
        ParseError::MalformedEntry {
            selector: format!("{selector_str}[id]"),
            raw: input.html(),
        }
    })?;
    let label = labels
        .get(dom_id)
        .filter(|label| !label.is_empty())
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: format!(r#"label.option[for="{dom_id}"]"#),
            raw: input.html(),
        })?;

    Ok(Matiere {
        id: id.to_string(),
        label: label.to_string(),
    })
}

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).expect("Static selector is valid")
}

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

    #[test]
    fn matieres_are_parsed_with_ids_and_svg_free_labels() {
        let html = concat!(
            r#"<input type="checkbox" id="edit-matieres-113--2" "#,
            r#"name="matieres[113]" value="113" "#,
            r#"class="form-checkbox hidden-checkbox">"#,
            r#"<label for="edit-matieres-113--2" class="option">"#,
            r#"<svg viewBox="0 0 16 16"><path d="m0 0"/></svg>"#,
            "\n  GEX - Génie des eaux </label>",
            r#"<input type="checkbox" id="edit-matieres-9--2" "#,
            r#"name="matieres[9]" value="9" "#,
            r#"class="form-checkbox hidden-checkbox">"#,
            r#"<label for="edit-matieres-9--2" class="option">"#,
            r#"<svg></svg>GUI - Gest. urbaine et immobilière</label>"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert!(anomalies.is_empty(), "no anomalies expected: {anomalies:?}");
        assert_eq!(matieres.len(), 2);
        assert_eq!(matieres[0].id, "113");
        assert_eq!(matieres[0].label, "GEX - Génie des eaux");
        assert_eq!(matieres[1].id, "9");
        assert_eq!(matieres[1].label, "GUI - Gest. urbaine et immobilière");
    }

    #[test]
    fn matiere_checkbox_without_label_is_an_anomaly_not_a_failure() {
        let html = concat!(
            r#"<input type="checkbox" id="edit-matieres-7--2" "#,
            r#"name="matieres[7]" value="7" class="form-checkbox">"#,
            r#"<input type="checkbox" id="edit-matieres-113--2" "#,
            r#"name="matieres[113]" value="113" class="form-checkbox">"#,
            r#"<label for="edit-matieres-113--2" class="option">"#,
            r#"<svg></svg>GEX - Génie des eaux</label>"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert_eq!(matieres.len(), 1, "valid matière still parsed");
        assert_eq!(matieres[0].id, "113");
        assert_eq!(anomalies.len(), 1);
        let anomaly = &anomalies[0];
        assert!(
            matches!(
                anomaly,
                ParseError::MalformedEntry { selector, .. }
                    if selector.contains("edit-matieres-7--2")
            ),
            "anomaly names the orphaned checkbox, got {anomaly:?}"
        );
    }

    #[test]
    fn matiere_checkbox_without_value_is_an_anomaly_not_a_failure() {
        let html = concat!(
            r#"<input type="checkbox" id="edit-matieres-7--2" "#,
            r#"name="matieres[7]" class="form-checkbox">"#,
            r#"<label for="edit-matieres-7--2" class="option">"#,
            r#"<svg></svg>ACT - Actuariat</label>"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert!(matieres.is_empty());
        assert_eq!(anomalies.len(), 1);
        assert!(
            matches!(
                &anomalies[0],
                ParseError::MalformedEntry { selector, .. }
                    if selector.contains("[value]")
            ),
            "anomaly names the missing attribute, got {:?}",
            anomalies[0]
        );
    }

    #[test]
    fn matiere_checkbox_without_dom_id_is_an_anomaly_not_a_failure() {
        // no id means no label can reference it
        let html = concat!(
            r#"<input type="checkbox" name="matieres[7]" value="7" "#,
            r#"class="form-checkbox">"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert!(matieres.is_empty());
        assert_eq!(anomalies.len(), 1);
        assert!(
            matches!(
                &anomalies[0],
                ParseError::MalformedEntry { selector, .. }
                    if selector.contains("[id]")
            ),
            "anomaly names the missing attribute, got {:?}",
            anomalies[0]
        );
    }

    #[test]
    fn a_label_without_for_pairs_with_nothing_and_is_skipped() {
        let html = concat!(
            r#"<label class="option"><svg></svg>Sans cible</label>"#,
            r#"<input type="checkbox" id="edit-matieres-7--2" "#,
            r#"name="matieres[7]" value="7" class="form-checkbox">"#,
            r#"<label for="edit-matieres-7--2" class="option">"#,
            r#"<svg></svg>ACT - Actuariat</label>"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert_eq!(matieres.len(), 1);
        assert!(anomalies.is_empty(), "no anomalies expected: {anomalies:?}");
    }

    #[test]
    fn page_without_the_facet_widget_is_drift_not_an_empty_facet() {
        let html = r#"<div class="total-resultats"><p>1 résultat</p></div>"#;

        let result = parse_matieres(html);

        assert!(
            matches!(result, Err(ParseError::MissingElement { .. })),
            "expected MissingElement, got {result:?}"
        );
    }

    #[test]
    fn duplicated_input_attributes_from_the_live_site_still_parse() {
        // the live HTML repeats the whole attribute block inside each
        // checkbox tag; html5ever keeps the first occurrence of each
        let html = concat!(
            r#"<input data-drupal-selector="edit-matieres-113" "#,
            r#"type="checkbox" id="edit-matieres-113--2" "#,
            r#"name="matieres[113]" value="113" "#,
            r#"class="form-checkbox hidden-checkbox" "#,
            r#"data-drupal-selector="edit-matieres-113" "#,
            r#"type="checkbox" id="edit-matieres-113--2" "#,
            r#"name="matieres[113]" value="113" "#,
            r#"class="form-checkbox hidden-checkbox" "#,
            r#"aria-controls="resultats">"#,
            r#"<label for="edit-matieres-113--2" class="option">"#,
            r#"<svg></svg>GEX - Génie des eaux</label>"#,
        );

        let (matieres, anomalies) =
            parse_matieres(html).expect("widget is present");

        assert!(anomalies.is_empty(), "no anomalies expected: {anomalies:?}");
        assert_eq!(matieres.len(), 1);
        assert_eq!(matieres[0].id, "113");
        assert_eq!(matieres[0].label, "GEX - Génie des eaux");
    }
}
