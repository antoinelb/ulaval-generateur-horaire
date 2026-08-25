// Importing one program from its ulaval.ca page: URL validation, the
// corsproxy.io target, and the import's three named phases (LAT-4/5, plan
// item 3/7). Pure and testable — the fetch itself is IO and lives in
// `browser.rs` (AP-7); French sentences live in `present.rs` (AP-5).
//
// This module also owns the shape the rest of the import produces —
// `LocalProgram` and the `build_local_program` step chaining parse +
// `semester_after` + `preparatory_rule` (plan items 1, 2, 4) — because
// `persist.rs`, `data.rs`, `panel.rs` and `browser.rs` (plan items 3, 4, 6,
// 7) all need the one type and step to build against; splitting it into a
// separate module would only add an import with no isolation benefit, since
// none of those callers can build without it.

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("invalid program url : {detail}")]
    InvalidUrl { detail: String },
    #[error("corsproxy.io request failed : {detail}")]
    Proxy { detail: String },
    #[error("program page not found (HTTP {status})")]
    NotFound { status: u16 },
    #[error("unexpected content type : {content_type}")]
    NotHtml { content_type: String },
    #[error("parsing program page : {detail}")]
    Parse { detail: String },
    #[error("computing scolarité préparatoire : {detail}")]
    Preparatory { detail: String },
    #[error("import cancelled")]
    Cancelled,
    #[error("browser could not prepare the request : {detail}")]
    BrowserApi { detail: String },
    #[error("catalogue not loaded yet")]
    CatalogueUnavailable,
}

// INP-7: only the commit validates. Accepts a real ulaval.ca program page —
// the form the scraper's own `cli.rs` defaults to,
// `https://www.ulaval.ca/etudes/programmes/{slug}` — normalizes the scheme
// and host, and drops query and fragment.
pub fn validate_program_url(raw: &str) -> Result<String, ImportError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ImportError::InvalidUrl {
            detail: "empty url".to_string(),
        });
    }
    let after_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(|| ImportError::InvalidUrl {
            detail: "url must start with https:// or http://".to_string(),
        })?;
    // a fragment can itself contain `?`, so it must go first
    let without_fragment = after_scheme.split('#').next().unwrap_or("");
    let without_query = without_fragment.split('?').next().unwrap_or("");
    let (host, path) = match without_query.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (without_query, String::new()),
    };
    if !host.eq_ignore_ascii_case("www.ulaval.ca")
        && !host.eq_ignore_ascii_case("ulaval.ca")
    {
        return Err(ImportError::InvalidUrl {
            detail: format!("unexpected host {host:?}"),
        });
    }
    const PREFIX: &str = "/etudes/programmes/";
    let Some(after_prefix) = path.strip_prefix(PREFIX) else {
        return Err(ImportError::InvalidUrl {
            detail: format!("path does not start with {PREFIX}"),
        });
    };
    let slug = after_prefix.split('/').next().unwrap_or("");
    if slug.is_empty() {
        return Err(ImportError::InvalidUrl {
            detail: "empty program slug".to_string(),
        });
    }
    Ok(format!("https://www.ulaval.ca{path}"))
}

// --- the stored shape (plan item 5) -----------------------------------------

// One program imported by URL, persisted under `gh.v1.programmes-locaux`
// (`persist.rs`). Anomalies ride as already-worded `Vec<String>` rather than
// `ParseError` — `ParseError` implements neither `Serialize` nor
// `Deserialize` and is not a thread type — the same convention as
// `Snapshot.warnings`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalProgram {
    pub program: ulaval_scheduler_core::Program,
    // provenance shown on the card (TRU)
    pub source_url: String,
    // an ISO instant the browser provides; pure code never invents time
    // (same discipline as `LogEntry.at`)
    pub imported_at: String,
    pub proxy: String,
    // parser anomalies, already worded — surfaced on the card, never tacit
    pub anomalies: Vec<String>,
}

pub const PROXY_HOST: &str = "corsproxy.io";

// Written by hand, not `js_sys::encode_uri_component`, so it is testable
// natively (plan item 3).
pub fn proxy_url(target: &str) -> String {
    let mut encoded = String::with_capacity(target.len());
    for byte in target.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => encoded.push(byte as char),
            // `write!` to a `String` cannot fail; nothing to propagate
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    format!("https://{PROXY_HOST}/?url={encoded}")
}

// The proxy's HTTP status and the target's content type, turned into the
// one typed outcome the rest of the import can branch on.
pub fn classify_response(
    status: u16,
    content_type: Option<&str>,
) -> Result<(), ImportError> {
    if status == 404 || status == 410 {
        return Err(ImportError::NotFound { status });
    }
    if !(200..=299).contains(&status) {
        return Err(ImportError::Proxy {
            detail: format!("HTTP {status}"),
        });
    }
    let mime = content_type
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_lowercase())
        .unwrap_or_default();
    if mime != "text/html" && mime != "application/xhtml+xml" {
        return Err(ImportError::NotHtml {
            content_type: content_type.unwrap_or_default().to_string(),
        });
    }
    Ok(())
}

// --- building a `LocalProgram` from the fetched page (plan items 1, 2, 4) --

// Items 1, 2 and 4 chained into the one pure step from fetched HTML to a
// program ready to store: parse, dated by the caller's clock with the same
// vintage rule the scrape uses (`core::semester_after`), then append the
// « Scolarité préparatoire » rule exactly as `cli.rs::add_preparatory_rules`
// does — this is the only place that applies it, so it never rides twice.
// Anomalies are kept as worded strings, surfaced on the card rather than
// dropped; a parsing or préparatoire failure comes back typed so the caller
// shows it as an import error without touching anything else in the app
// (BLD-1).
pub fn build_local_program(
    html: &str,
    source_url: &str,
    imported_at: String,
    now_secs: u64,
    courses: &[ulaval_scheduler_core::Course],
) -> Result<LocalProgram, ImportError> {
    let semester = ulaval_scheduler_core::semester_after(now_secs);
    let page = ulaval_scheduler_core::parser::program::parse(html, semester)
        .map_err(|error| ImportError::Parse {
            detail: error.to_string(),
        })?;
    let mut program = page.program;
    let anomalies =
        page.anomalies.iter().map(ToString::to_string).collect();

    match ulaval_scheduler_core::preparatory_rule(&program.mandatory, courses)
    {
        Ok(Some(rule)) => program.rules.push(rule),
        // nothing reachable: the rule is omitted, not emitted empty
        Ok(None) => {}
        Err(error) => {
            return Err(ImportError::Preparatory {
                detail: error.to_string(),
            })
        }
    }

    Ok(LocalProgram {
        program,
        source_url: source_url.to_string(),
        imported_at,
        proxy: PROXY_HOST.to_string(),
        anomalies,
    })
}

// --- the import's three named phases (D3) ----------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImportPhase {
    Download,
    Parse,
    Save,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhaseState {
    Done,
    Running,
    Pending,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PhaseRow {
    pub label: String,
    pub state: PhaseState,
}

const PHASES: [(ImportPhase, &str); 3] = [
    (ImportPhase::Download, "Téléchargement de la page"),
    (ImportPhase::Parse, "Analyse du programme"),
    (ImportPhase::Save, "Enregistrement"),
];

// `None` (nothing started yet) is the fourth state the caller can be in —
// every row reads `Pending`.
pub fn phase_rows(current: Option<ImportPhase>) -> Vec<PhaseRow> {
    let Some(current) = current else {
        return PHASES
            .iter()
            .map(|&(_, label)| PhaseRow {
                label: label.to_string(),
                state: PhaseState::Pending,
            })
            .collect();
    };
    let current_index = PHASES
        .iter()
        .position(|&(phase, _)| phase == current)
        .unwrap_or(0);
    PHASES
        .iter()
        .enumerate()
        .map(|(index, &(_, label))| {
            let state = match index.cmp(&current_index) {
                std::cmp::Ordering::Less => PhaseState::Done,
                std::cmp::Ordering::Equal => PhaseState::Running,
                std::cmp::Ordering::Greater => PhaseState::Pending,
            };
            PhaseRow {
                label: label.to_string(),
                state,
            }
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use ulaval_scheduler_core::{
        Course, CourseCycle, Credits, PrereqTree, Prerequisites,
    };

    // --- validate_program_url ---

    #[test]
    fn an_empty_url_is_refused() {
        assert!(matches!(
            validate_program_url("   "),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn a_missing_scheme_is_refused() {
        assert!(matches!(
            validate_program_url("www.ulaval.ca/etudes/programmes/gex"),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn a_wrong_host_is_refused() {
        assert!(matches!(
            validate_program_url("https://example.com/etudes/programmes/gex"),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn a_wrong_path_is_refused() {
        assert!(matches!(
            validate_program_url("https://www.ulaval.ca/autre-chemin/gex"),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn a_host_with_no_path_at_all_is_refused() {
        assert!(matches!(
            validate_program_url("https://www.ulaval.ca"),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn an_empty_final_segment_is_refused() {
        assert!(matches!(
            validate_program_url("https://www.ulaval.ca/etudes/programmes/"),
            Err(ImportError::InvalidUrl { .. })
        ));
    }

    #[test]
    fn http_is_rewritten_to_https() {
        assert_eq!(
            validate_program_url("http://www.ulaval.ca/etudes/programmes/gex")
                .unwrap_or_else(|e| panic!("{e}")),
            "https://www.ulaval.ca/etudes/programmes/gex"
        );
    }

    #[test]
    fn a_bare_ulaval_host_is_normalized_to_www() {
        assert_eq!(
            validate_program_url("https://ulaval.ca/etudes/programmes/gex")
                .unwrap_or_else(|e| panic!("{e}")),
            "https://www.ulaval.ca/etudes/programmes/gex"
        );
    }

    #[test]
    fn query_and_fragment_are_dropped() {
        assert_eq!(
            validate_program_url(
                "https://www.ulaval.ca/etudes/programmes/gex?lang=fr#section"
            )
            .unwrap_or_else(|e| panic!("{e}")),
            "https://www.ulaval.ca/etudes/programmes/gex"
        );
    }

    #[test]
    fn a_valid_url_is_returned_trimmed_and_unchanged() {
        assert_eq!(
            validate_program_url(
                "  https://www.ulaval.ca/etudes/programmes/gex  "
            )
            .unwrap_or_else(|e| panic!("{e}")),
            "https://www.ulaval.ca/etudes/programmes/gex"
        );
    }

    // --- proxy_url ---

    #[test]
    fn the_proxy_url_wraps_the_target_under_its_query_parameter() {
        let wrapped = proxy_url("https://www.ulaval.ca/etudes/programmes/gex");
        assert!(wrapped.starts_with("https://corsproxy.io/?url="));
    }

    #[test]
    fn reserved_characters_are_percent_encoded() {
        let wrapped = proxy_url("https://a.b/c?d=e");
        assert_eq!(
            wrapped,
            "https://corsproxy.io/?url=https%3A%2F%2Fa.b%2Fc%3Fd%3De"
        );
    }

    #[test]
    fn accented_characters_are_percent_encoded_byte_by_byte() {
        let wrapped = proxy_url("é");
        // 'é' is the two UTF-8 bytes 0xC3 0xA9
        assert_eq!(wrapped, "https://corsproxy.io/?url=%C3%A9");
    }

    #[test]
    fn unreserved_characters_pass_through() {
        let wrapped = proxy_url("abc-XYZ_123.~");
        assert_eq!(wrapped, "https://corsproxy.io/?url=abc-XYZ_123.~");
    }

    // --- classify_response ---

    #[test]
    fn a_404_or_410_is_not_found() {
        for status in [404, 410] {
            assert_eq!(
                classify_response(status, Some("text/html")),
                Err(ImportError::NotFound { status })
            );
        }
    }

    #[test]
    fn any_other_status_outside_2xx_is_a_proxy_failure() {
        assert_eq!(
            classify_response(500, Some("text/html")),
            Err(ImportError::Proxy {
                detail: "HTTP 500".to_string()
            })
        );
    }

    #[test]
    fn a_missing_content_type_is_not_html() {
        assert_eq!(
            classify_response(200, None),
            Err(ImportError::NotHtml {
                content_type: String::new()
            })
        );
    }

    #[test]
    fn a_non_html_content_type_is_refused() {
        assert_eq!(
            classify_response(200, Some("application/json")),
            Err(ImportError::NotHtml {
                content_type: "application/json".to_string()
            })
        );
    }

    #[test]
    fn html_with_a_charset_parameter_is_accepted_case_insensitively() {
        assert_eq!(
            classify_response(200, Some("Text/HTML; charset=utf-8")),
            Ok(())
        );
    }

    #[test]
    fn xhtml_is_accepted_too() {
        assert_eq!(
            classify_response(200, Some("application/xhtml+xml")),
            Ok(())
        );
    }

    #[test]
    fn every_2xx_status_with_html_is_accepted() {
        assert_eq!(classify_response(200, Some("text/html")), Ok(()));
        assert_eq!(classify_response(299, Some("text/html")), Ok(()));
    }

    // --- phase_rows ---

    #[test]
    fn no_phase_started_reads_as_three_pending_rows() {
        let rows = phase_rows(None);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.state == PhaseState::Pending));
    }

    #[test]
    fn the_download_phase_is_running_with_nothing_done_yet() {
        let rows = phase_rows(Some(ImportPhase::Download));
        assert_eq!(rows[0].state, PhaseState::Running);
        assert_eq!(rows[1].state, PhaseState::Pending);
        assert_eq!(rows[2].state, PhaseState::Pending);
    }

    #[test]
    fn the_parse_phase_marks_download_done_and_save_pending() {
        let rows = phase_rows(Some(ImportPhase::Parse));
        assert_eq!(rows[0].state, PhaseState::Done);
        assert_eq!(rows[1].state, PhaseState::Running);
        assert_eq!(rows[2].state, PhaseState::Pending);
    }

    #[test]
    fn the_save_phase_marks_the_first_two_done() {
        let rows = phase_rows(Some(ImportPhase::Save));
        assert_eq!(rows[0].state, PhaseState::Done);
        assert_eq!(rows[1].state, PhaseState::Done);
        assert_eq!(rows[2].state, PhaseState::Running);
        assert_eq!(rows[2].label, "Enregistrement");
    }

    // --- build_local_program ---

    // The smallest page the program parser reads without an anomaly:
    // title, official-code accordion button, canonical link, total
    // credits, admission sessions, and one block holding one accordion
    // (`GEX-1000` as its only mandatory course) — the same literal
    // `crates/scraper/src/program.rs`'s own tests build, copied here so
    // this crate need not depend on scraper's private test helpers.
    fn program_html(slug: &str, code: &str, with_admission: bool) -> String {
        let matiere = code.rsplit('-').next().unwrap_or(code);
        let admission = if with_admission {
            concat!(
                r#"<div class="admission--liste-sessions">"#,
                "<h2>Sessions d'admission</h2><ul>",
                r#"<li class="bloc-session">"#,
                r#"<strong class="bloc-session--titre">Automne</strong>"#,
                "</li></ul></div>",
            )
        } else {
            ""
        };
        format!(
            concat!(
                "<html><body>",
                "<h1>Baccalauréat en {slug}</h1>",
                r#"<button class="header-wrapper accordeon-oe-programme" "#,
                r#"id="{code}-{matiere}-avenir"></button>"#,
                r#"<link rel="canonical" "#,
                r#"href="https://www.ulaval.ca/etudes/programmes/{slug}">"#,
                r#"<div class="bloc-promo">"#,
                r#"<span class="promo-entete--titre">120</span>"#,
                r#"<span class="promo-entete--contenu">Crédits</span>"#,
                "</div>",
                "{admission}",
                r#"<section id="section-structure">"#,
                r#"<div class="fe-bloc-section">"#,
                r#"<div class="collapsible-sections">"#,
                r#"<div class="controls-title fe-bloc-titre">"#,
                r#"<h4 class="fe-bloc-titre--texte">Programme</h4>"#,
                r#"<span class="fe-bloc-titre--credits">"#,
                "120 crédits exigés</span></div>",
                r#"<div class="toggle-section">"#,
                r#"<p class="toggle-section--header">"#,
                r#"<span class="item">Cours obligatoires</span></p>"#,
                r#"<div class="toggle-section--content">"#,
                r#"<div class="fe-bloc-regle--paragraphe">"#,
                r#"<ul class="fe--liste-cours"><li>"#,
                r#"<span class="cours-carte--sigle">GEX-1000</span>"#,
                "</li></ul>",
                "</div>",
                "</div></div></div></div></section>",
                "</body></html>",
            ),
            slug = slug,
            code = code,
            matiere = matiere,
            admission = admission,
        )
    }

    fn course(code: &str, prerequisites: Option<Prerequisites>) -> Course {
        Course {
            code: code.to_string(),
            title: "x".to_string(),
            credits: Credits::Fixed(3),
            cycle: CourseCycle::First,
            prerequisites,
            equivalents: Vec::new(),
            seasons: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn a_valid_page_is_dated_by_the_caller_clock() {
        let html = program_html("genie-des-eaux", "B-GEX", true);
        // an arbitrary but fixed instant; the point is comparing against
        // `semester_after` applied to that same instant, not a hardcoded
        // semester
        let now_secs = 1_772_000_000;
        let expected_semester =
            ulaval_scheduler_core::semester_after(now_secs);

        let local = build_local_program(
            &html,
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            now_secs,
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(local.program.code, "B-GEX");
        assert_eq!(local.program.semester, expected_semester);
        assert_eq!(local.program.mandatory, vec!["GEX-1000".to_string()]);
        assert!(local.anomalies.is_empty());
    }

    #[test]
    fn a_reachable_preuniversity_prerequisite_gets_the_preparatory_rule() {
        let html = program_html("genie-des-eaux", "B-GEX", true);
        let courses = vec![course(
            "GEX-1000",
            Some(Prerequisites::Parsed {
                raw: "MAT-0339".to_string(),
                tree: PrereqTree::Course("MAT-0339".to_string()),
            }),
        )];

        let local = build_local_program(
            &html,
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            1_772_000_000,
            &courses,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        // exactly one, not merely at least one: a stray duplicate push
        // would still let `find` succeed below
        let matching: Vec<_> = local
            .program
            .rules
            .iter()
            .filter(|rule| rule.title == "Scolarité préparatoire")
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "the preparatory rule must be applied exactly once, got {:?}",
            local.program.rules
        );
        let rule = matching[0];
        match &rule.courses {
            ulaval_scheduler_core::RuleCourses::List { courses } => {
                assert_eq!(courses, &["MAT-0339".to_string()]);
            }
            other => panic!("expected a course list, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_catalogue_yields_no_preparatory_rule() {
        let html = program_html("genie-des-eaux", "B-GEX", true);

        let local = build_local_program(
            &html,
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            1_772_000_000,
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert!(!local
            .program
            .rules
            .iter()
            .any(|rule| rule.title == "Scolarité préparatoire"));
    }

    #[test]
    fn unreadable_html_is_a_typed_parse_error() {
        let error = build_local_program(
            "",
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            1_772_000_000,
            &[],
        )
        .expect_err("empty html carries none of the required elements");
        assert!(matches!(error, ImportError::Parse { .. }));
    }

    #[test]
    fn a_parser_anomaly_is_surfaced_not_swallowed() {
        // no admission block at all: the parser keeps the program but
        // records an anomaly (`parse_possible_semester_start`)
        let html = program_html("genie-des-eaux", "B-GEX", false);

        let local = build_local_program(
            &html,
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            1_772_000_000,
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert!(
            !local.anomalies.is_empty(),
            "a missing admission block must surface as an anomaly"
        );
    }

    #[test]
    fn provenance_is_copied_verbatim() {
        let html = program_html("genie-des-eaux", "B-GEX", true);
        let source_url =
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux";
        let imported_at = "2026-08-24T12:34:56Z".to_string();

        let local = build_local_program(
            &html,
            source_url,
            imported_at.clone(),
            1_772_000_000,
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(local.source_url, source_url);
        assert_eq!(local.imported_at, imported_at);
        assert_eq!(local.proxy, PROXY_HOST);
    }

    #[test]
    fn a_graph_over_budget_is_a_typed_preparatory_error() {
        // mirrors `core::preparatory::tests::a_graph_over_budget_is_a_typed_error`:
        // one mandatory course whose tree names more than the 10 000-node
        // budget forces `preparatory_rule` itself to error, which this
        // function must turn into `ImportError::Preparatory` rather than
        // panicking or silently dropping the rule
        let html = program_html("genie-des-eaux", "B-GEX", true);
        let leaves = (0..10_001)
            .map(|i| PrereqTree::Course(format!("GEX-{i:05}")))
            .collect();
        let courses = vec![course(
            "GEX-1000",
            Some(Prerequisites::Parsed {
                raw: "x".to_string(),
                tree: PrereqTree::All { all: leaves },
            }),
        )];

        let error = build_local_program(
            &html,
            "https://www.ulaval.ca/etudes/programmes/genie-des-eaux",
            "2026-08-24T00:00:00Z".to_string(),
            1_772_000_000,
            &courses,
        )
        .expect_err("a graph over the node budget must error");
        assert!(matches!(error, ImportError::Preparatory { .. }));
    }
}
