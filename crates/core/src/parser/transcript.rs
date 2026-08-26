use std::sync::LazyLock;

use crate::parser::ParseError;
use crate::{
    CourseCycle, Season, Semester, Transcript, TranscriptCourse,
    TranscriptSection, TranscriptSession,
};
use scraper::{ElementRef, Html, Selector};

const BANNER_CSS: &str = "th.ddtitle";
static BANNER: LazyLock<Selector> = LazyLock::new(|| sel(BANNER_CSS));
static SESSION_LABEL: LazyLock<Selector> =
    LazyLock::new(|| sel("span.fieldOrangetextbold"));
static DDDEFAULT: LazyLock<Selector> = LazyLock::new(|| sel("td.dddefault"));
static TR: LazyLock<Selector> = LazyLock::new(|| sel("tr"));
static TH: LazyLock<Selector> = LazyLock::new(|| sel("th"));

const LAVAL_BANNER: &str = "CRÉDITS DE L'UNIVERSITÉ LAVAL";
const RECOGNIZED_BANNER: &str = "RECONNAISSANCE DES ACQUIS";
const IN_PROGRESS_BANNER: &str = "CRÉDITS EN COURS";
const PROGRAM_BANNER: &str = "PROGRAMME(S) FRÉQUENTÉ(S)";

#[derive(Debug)]
pub struct TranscriptPage {
    pub transcript: Transcript,
    pub anomalies: Vec<ParseError>,
    // the earliest « Fréquentation » start among the program(s) marked
    // « En cours » on « PROGRAMME(S) FRÉQUENTÉ(S) » — `None` when that
    // section is absent or names no program still in progress. Scopes
    // `earliest_start` (`transcript.rs`) away from an older, unrelated
    // program's sessions; not part of the relevé grammar proper, so its
    // absence is never an anomaly (ADR `2026-08-import-de-releve-capsule`).
    pub program_floor: Option<Semester>,
}

// The page is walked one `<tr>` at a time, in document order, driving a
// small state machine: which of the three grammar sections is active, and
// which session (if any) course rows currently belong to. A row outside the
// grammar — a column header, a totals line, a separator, an empty row — is
// recognized by shape and skipped without comment; anything the grammar
// cannot place is surfaced instead of silently dropped (ADR
// `2026-08-import-de-releve-capsule`).
pub fn parse(html: &str) -> Result<TranscriptPage, ParseError> {
    let doc = Html::parse_document(html);

    let mut anomalies = Vec::new();
    let mut sessions: Vec<TranscriptSession> = Vec::new();
    let mut section: Option<TranscriptSection> = None;
    let mut session_index: Option<usize> = None;
    let mut any_banner_seen = false;
    let mut recognized_seen = false;

    for row in doc.select(&TR) {
        if let Some(banner) = row.select(&BANNER).next() {
            any_banner_seen = true;
            section = banner_kind(banner);
            recognized_seen |= section.is_some();
            session_index = None;
            continue;
        }

        if let Some(label) = row.select(&SESSION_LABEL).next() {
            if let Some(active_section) = section {
                handle_session_header(
                    row,
                    label,
                    active_section,
                    &mut sessions,
                    &mut session_index,
                    &mut anomalies,
                );
            }
            continue;
        }

        handle_row(
            row,
            section,
            session_index,
            any_banner_seen,
            &mut sessions,
            &mut anomalies,
        );
    }

    if !recognized_seen {
        // two different diagnoses share this one guard: no `th.ddtitle` at
        // all (the element itself is missing, `BANNER_CSS` says so
        // truthfully) versus some `th.ddtitle` present but none of them one
        // of the three recognized banners (the element is there — what is
        // missing is a name the grammar knows, and the message must say so,
        // not point back at a selector that already matched something; TRU)
        let selector = if any_banner_seen {
            format!(
                "{BANNER_CSS} portant l'une des trois bannières \
                 reconnues : « {LAVAL_BANNER} », « {RECOGNIZED_BANNER} » \
                 ou « {IN_PROGRESS_BANNER} »"
            )
        } else {
            BANNER_CSS.to_string()
        };
        return Err(ParseError::MissingElement { selector });
    }

    Ok(TranscriptPage {
        transcript: Transcript { sessions },
        anomalies,
        program_floor: parse_program_floor(&doc),
    })
}

// « PROGRAMME(S) FRÉQUENTÉ(S) » lists every program the relevé's owner has
// ever pursued at Laval — a finished certificat or an earlier bac included —
// with no per-course tag elsewhere on the page to say which program a given
// « CRÉDITS DE L'UNIVERSITÉ LAVAL » row belongs to. Only the program(s)
// still « En cours » (never a finished « Diplôme obtenu » one) bound the
// horizon: the earliest of their own « Fréquentation » start dates. Reads
// its own state machine over the same `<tr>` stream `parse` walks —
// `Programme:` resets the pending start, `Fréquentation:` reads it,
// `En cours` commits it to the floor — since none of this is part of the
// three-section course grammar above.
fn parse_program_floor(doc: &Html) -> Option<Semester> {
    let mut in_section = false;
    let mut pending_start: Option<Semester> = None;
    let mut floor: Option<Semester> = None;

    for row in doc.select(&TR) {
        if let Some(banner) = row.select(&BANNER).next() {
            in_section = collapse_text(banner).starts_with(PROGRAM_BANNER);
            continue;
        }
        if !in_section {
            continue;
        }
        let Some(label) = row.select(&TH).next() else {
            continue;
        };
        match collapse_text(label).as_str() {
            "Programme:" => pending_start = None,
            "Fréquentation:" => {
                pending_start =
                    row.select(&DDDEFAULT).next().and_then(|cell| {
                        parse_frequentation_start(&collapse_text(cell))
                    });
            }
            "En cours" => {
                if let Some(start) = pending_start {
                    floor = Some(match floor {
                        Some(current)
                            if semester_rank(current)
                                <= semester_rank(start) =>
                        {
                            current
                        }
                        _ => start,
                    });
                }
            }
            _ => {}
        }
    }
    floor
}

// hiver, puis été, puis automne within one civil year — `Season`'s derived
// `Ord` is declaration order, not calendar time (mirrors
// `transcript::semester_key`, kept local: that one is private to its module
// and this comparison never needs the full `Transcript` domain type).
fn semester_rank(semester: Semester) -> (u16, u8) {
    let rank = match semester.season {
        Season::Winter => 0,
        Season::Summer => 1,
        Season::Fall => 2,
    };
    (semester.year, rank)
}

// « Automne 2024 à ce jour » (in progress), « Été 2025 » (a microprogramme's
// one-session stint) or a finished program's « Automne 2018 à Hiver 2021 » —
// only the leading season/year is read, trailing words (an end date, « à ce
// jour ») are not needed here and never make this fail. Not part of the
// relevé grammar: an unparseable or absent value is `None`, never reported.
fn parse_frequentation_start(raw: &str) -> Option<Semester> {
    let mut words = raw.split_whitespace();
    let season = match words.next()? {
        "Automne" => Season::Fall,
        "Hiver" => Season::Winter,
        "Été" => Season::Summer,
        _ => return None,
    };
    let year = words.next()?.parse::<u16>().ok()?;
    Some(Semester { season, year })
}

// « CRÉDITS DE L'UNIVERSITÉ LAVAL », « RECONNAISSANCE DES ACQUIS » and
// « CRÉDITS EN COURS » each carry trailing markup (a jump-to-top link, a
// spacer of `&nbsp;`) after the banner text proper, so the section is read
// off a prefix match rather than equality. Any other banner («
// INFORMATIONS ÉTUDIANTES », « PROGRAMME(S) FRÉQUENTÉ(S) », « BILAN DU
// RELEVÉ (PREMIER CYCLE) ») is out of the grammar by design: `None` leaves
// every row under it skipped, never reported.
fn banner_kind(banner: ElementRef) -> Option<TranscriptSection> {
    let text = collapse_text(banner);

    if text.starts_with(LAVAL_BANNER) {
        Some(TranscriptSection::Laval)
    } else if text.starts_with(RECOGNIZED_BANNER) {
        Some(TranscriptSection::Recognized)
    } else if text.starts_with(IN_PROGRESS_BANNER) {
        Some(TranscriptSection::InProgress)
    } else {
        None
    }
}

// A session-header row's institution, when it has one, sits in a
// `td.dddefault` sibling of the same `<tr>` — only ever populated under
// « RECONNAISSANCE DES ACQUIS », since Université Laval never names itself.
fn handle_session_header(
    row: ElementRef,
    label: ElementRef,
    section: TranscriptSection,
    sessions: &mut Vec<TranscriptSession>,
    session_index: &mut Option<usize>,
    anomalies: &mut Vec<ParseError>,
) {
    let raw = collapse_text(label);

    match parse_semester_label(&raw) {
        Ok(semester) => {
            let institution = row
                .select(&DDDEFAULT)
                .next()
                .map(collapse_text)
                .filter(|text| !text.is_empty());
            sessions.push(TranscriptSession {
                section,
                semester,
                institution,
                courses: Vec::new(),
            });
            *session_index = Some(sessions.len() - 1);
        }
        // an unparseable label is reported on its own; the rows that follow
        // it have no session to attach to, so each is reported in turn
        // rather than pinned to the wrong one
        Err(error) => {
            anomalies.push(error);
            *session_index = None;
        }
    }
}

// « Automne 2024 », or « Hiver 2013: » under RECONNAISSANCE — the trailing
// colon is trimmed before the season word is read.
fn parse_semester_label(raw: &str) -> Result<Semester, ParseError> {
    let malformed = || ParseError::MalformedEntry {
        selector: "span.fieldOrangetextbold".to_string(),
        raw: raw.to_string(),
    };

    let mut words = raw.trim().trim_end_matches(':').split_whitespace();
    let season = match words.next() {
        Some("Automne") => Season::Fall,
        Some("Hiver") => Season::Winter,
        Some("Été") => Season::Summer,
        _ => return Err(malformed()),
    };
    let year = words
        .next()
        .and_then(|word| word.parse::<u16>().ok())
        .ok_or_else(malformed)?;
    if words.next().is_some() {
        return Err(malformed());
    }
    // `Semester`'s documented domain is two-digit (`Display` is `year %
    // 100`, `FromStr` is always `2000 + yy` — `core/src/program.rs`): a year
    // outside 2000-2099 builds a value neither of those can round-trip
    // (« Hiver 1998 » displays "H98", which `FromStr` reads back as 2098),
    // so it is malformed here rather than silently anchoring a plan on the
    // wrong century.
    if !(2000..=2099).contains(&year) {
        return Err(malformed());
    }

    Ok(Semester { season, year })
}

// A course row is recognized by its first cell alone: a `td.dddefault`
// holding a course code. Every other row shape a relevé page carries —
// column headers, totals, separators, an empty `<tr>` — is a `<th>` first,
// a `td.ddseparator`/`td.dddead` first, or has no first cell at all, and is
// skipped here without comment.
fn handle_row(
    row: ElementRef,
    section: Option<TranscriptSection>,
    session_index: Option<usize>,
    any_banner_seen: bool,
    sessions: &mut [TranscriptSession],
    anomalies: &mut Vec<ParseError>,
) {
    let Some(cell) = first_cell(row) else {
        return;
    };
    if !DDDEFAULT.matches(&cell) {
        return;
    }
    let code = collapse_text(cell);
    if !is_transcript_course_code(&code) {
        return;
    }

    if section.is_none() {
        // an unrecognized banner already explains why nothing here is kept;
        // only a course row read before the relevé's grammar ever started
        // (no banner seen at all yet) is a surprise worth reporting
        if !any_banner_seen {
            anomalies.push(ParseError::MalformedEntry {
                selector: "course row".to_string(),
                raw: format!("{code}: under no recognized section"),
            });
        }
        return;
    }

    let Some(index) = session_index else {
        anomalies.push(ParseError::MalformedEntry {
            selector: "course row".to_string(),
            raw: format!("{code}: before any session header"),
        });
        return;
    };

    match parse_course_row(row, &code) {
        Ok(course) => sessions[index].courses.push(course),
        Err(error) => anomalies.push(error),
    }
}

// The remaining `td.dddefault` cells of a course row, in order: cycle,
// title, then either (grade, credits) — every section but « CRÉDITS EN
// COURS » — or (credits) alone, since a session still running has no note
// yet.
fn parse_course_row(
    row: ElementRef,
    code: &str,
) -> Result<TranscriptCourse, ParseError> {
    let cells: Vec<ElementRef> = row.select(&DDDEFAULT).collect();

    let (cycle, title, grade, credits) = match cells.as_slice() {
        [_, cycle, title, grade, credits] => {
            (*cycle, *title, Some(*grade), *credits)
        }
        [_, cycle, title, credits] => (*cycle, *title, None, *credits),
        _ => {
            return Err(ParseError::MalformedEntry {
                selector: "course row".to_string(),
                raw: format!(
                    "{code}: {} value cells",
                    cells.len().saturating_sub(1)
                ),
            });
        }
    };

    Ok(TranscriptCourse {
        code: code.to_string(),
        cycle: parse_transcript_cycle(&collapse_text(cycle), code)?,
        title: collapse_text(title),
        grade: grade.map(collapse_text),
        credits: parse_transcript_credits(&collapse_text(credits), code)?,
    })
}

fn parse_transcript_cycle(
    raw: &str,
    code: &str,
) -> Result<CourseCycle, ParseError> {
    match raw {
        "0" => Ok(CourseCycle::Preuniversity),
        "1" => Ok(CourseCycle::First),
        "2" => Ok(CourseCycle::Second),
        other => Err(ParseError::MalformedEntry {
            selector: "cycle".to_string(),
            raw: format!("{code}: {other}"),
        }),
    }
}

// wrapped in `<p class="rightaligntext">` and padded on the real page — every
// whitespace run is dropped before the integer is parsed
fn parse_transcript_credits(raw: &str, code: &str) -> Result<i64, ParseError> {
    let collapsed: String = raw.split_whitespace().collect();
    collapsed
        .parse::<i64>()
        .map_err(|_| ParseError::MalformedEntry {
            selector: "credits".to_string(),
            raw: format!("{code}: {raw}"),
        })
}

// `XX(X{0,2})-####`, i.e. `[A-Z]{2,4}-[0-9]{4}`, matched on bytes — `core`
// carries no `regex` crate.
fn is_transcript_course_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    let subject_len =
        bytes.iter().take_while(|b| b.is_ascii_uppercase()).count();

    (2..=4).contains(&subject_len)
        && bytes.get(subject_len) == Some(&b'-')
        && bytes.len() == subject_len + 5
        && bytes[subject_len + 1..].iter().all(u8::is_ascii_digit)
}

// text can be split across nodes/lines and padded with tabs;
// collapse it to a single space-joined string, matching parser/program.rs
fn collapse_text(element: ElementRef) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_cell(row: ElementRef) -> Option<ElementRef> {
    row.children().filter_map(ElementRef::wrap).next()
}

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).expect("Static selector is valid")
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_frequentation_value_is_simply_absent() {
        // whitespace-only: nothing to read, `None`, never an anomaly
        assert_eq!(parse_frequentation_start("  "), None);
    }

    // --- HTML builders ---

    fn document(body: &str) -> String {
        format!("<html><body><table>{body}</table></body></html>")
    }

    fn banner(text: &str) -> String {
        format!(r#"<tr><th class="ddtitle">{text}</th></tr>"#)
    }

    // a « PROGRAMME(S) FRÉQUENTÉ(S) » row: `Programme:`, `Fréquentation:`,
    // `En cours` or `Diplôme obtenu`, each a `th.ddlabel` optionally
    // followed by a `td.dddefault` value
    fn program_row(label: &str, value: Option<&str>) -> String {
        let cell = value
            .map(|v| format!(r#"<td class="dddefault">{v}</td>"#))
            .unwrap_or_default();
        format!(r#"<tr><th class="ddlabel">{label}</th>{cell}</tr>"#)
    }

    fn session_header(label: &str, institution: Option<&str>) -> String {
        let institution_cell = institution
            .map(|text| format!(r#"<td class="dddefault">{text}</td>"#))
            .unwrap_or_default();
        format!(
            r#"<tr><th class="ddlabel"><span class="fieldOrangetextbold">{label}</span></th>{institution_cell}</tr>"#
        )
    }

    fn course_row(
        code: &str,
        cycle: &str,
        title: &str,
        grade: Option<&str>,
        credits: &str,
    ) -> String {
        let grade_cell = grade
            .map(|g| format!(r#"<td class="dddefault">{g}</td>"#))
            .unwrap_or_default();
        format!(
            r#"<tr><td class="dddefault">{code}</td><td class="dddefault">{cycle}</td><td class="dddefault">{title}</td>{grade_cell}<td class="dddefault">{credits}</td></tr>"#
        )
    }

    fn header_row(text: &str) -> String {
        format!(r#"<tr><th class="ddheader">{text}</th></tr>"#)
    }

    fn separator_row() -> &'static str {
        r#"<tr><td class="ddseparator">&nbsp;</td></tr>"#
    }

    fn malformed_entry(error: &ParseError) -> (&str, &str) {
        match error {
            ParseError::MalformedEntry { selector, raw } => {
                (selector.as_str(), raw.as_str())
            }
            other => panic!("expected MalformedEntry, got {other:?}"),
        }
    }

    fn ok_page(html: &str) -> TranscriptPage {
        parse(html).unwrap_or_else(|e| panic!("expected a page, got {e}"))
    }

    // --- Sections ---

    #[test]
    fn a_laval_block_with_grades_is_read_session_by_session() {
        // covers all three cycle values in one pass: préuniversitaire,
        // premier and deuxième cycle
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("CHM-0150", "0", "Chimie", Some("A"), "3"),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
                course_row("GEX-7000", "2", "Séminaire", Some("A+"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.transcript.sessions.len(), 1);
        let session = &page.transcript.sessions[0];
        assert_eq!(session.section, TranscriptSection::Laval);
        assert_eq!(
            session.semester,
            Semester {
                season: Season::Fall,
                year: 2024
            }
        );
        assert_eq!(session.institution, None);
        assert_eq!(
            session
                .courses
                .iter()
                .map(|c| (c.code.as_str(), c.cycle, c.grade.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("CHM-0150", CourseCycle::Preuniversity, Some("A")),
                ("BIO-1904", CourseCycle::First, Some("B")),
                ("GEX-7000", CourseCycle::Second, Some("A+")),
            ]
        );
    }

    #[test]
    fn an_in_progress_block_has_no_note_column() {
        let html = document(
            &[
                banner(IN_PROGRESS_BANNER),
                session_header("Été 2026", None),
                course_row("GEX-2590", "1", "Stage", None, "9"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        let session = &page.transcript.sessions[0];
        assert_eq!(session.section, TranscriptSection::InProgress);
        assert_eq!(
            session.semester,
            Semester {
                season: Season::Summer,
                year: 2026
            }
        );
        assert_eq!(session.courses[0].grade, None);
        assert_eq!(session.courses[0].credits, 9);
    }

    #[test]
    fn a_recognized_block_carries_its_institution_and_a_colon_label() {
        let html = document(
            &[
                banner(RECOGNIZED_BANNER),
                session_header("Hiver 2013:", Some("Université de Montréal")),
                course_row("MAT-1910", "1", "Maths", Some("V"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        let session = &page.transcript.sessions[0];
        assert_eq!(session.section, TranscriptSection::Recognized);
        assert_eq!(
            session.semester,
            Semester {
                season: Season::Winter,
                year: 2013
            }
        );
        assert_eq!(
            session.institution,
            Some("Université de Montréal".to_string())
        );
    }

    // --- Rows skipped without anomaly ---

    #[test]
    fn a_total_session_row_is_skipped() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
                header_row("Total session (Premier cycle)"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.transcript.sessions[0].courses.len(), 1);
    }

    #[test]
    fn a_separator_row_is_skipped() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                separator_row().to_string(),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert!(page.transcript.sessions[0].courses.is_empty());
    }

    #[test]
    fn an_empty_row_is_skipped() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                "<tr></tr>".to_string(),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert!(page.transcript.sessions[0].courses.is_empty());
    }

    #[test]
    fn a_dddefault_led_row_that_is_not_a_course_code_is_skipped() {
        // shaped like the RECONNAISSANCE « Crédits réussis / Crédits pour
        // moyenne / Moyenne » summary row: a `td.dddefault` first cell whose
        // text is not a sigle
        let html = document(&[
            banner(LAVAL_BANNER),
            session_header("Automne 2024", None),
            r#"<tr><td class="dddefault">Crédits réussis</td><td class="dddefault">3</td></tr>"#
                .to_string(),
        ]
        .concat());

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert!(page.transcript.sessions[0].courses.is_empty());
    }

    #[test]
    fn subject_prefixes_from_two_to_four_letters_are_recognized() {
        // the grammar is `XX(X{0,2})-####`, i.e. 2 to 4 letters: a 4-letter
        // sigle silently dropped by a narrower `2..=3` would collide with
        // the plan's own « toute ligne … est rapportée, jamais ignorée »
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("MA-1000", "1", "Deux lettres", Some("A"), "3"),
                course_row("GENI-1000", "1", "Quatre lettres", Some("B"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(
            page.transcript.sessions[0]
                .courses
                .iter()
                .map(|c| c.code.as_str())
                .collect::<Vec<_>>(),
            vec!["MA-1000", "GENI-1000"],
        );
    }

    #[test]
    fn a_five_letter_subject_prefix_is_still_not_a_course_code() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("GENIE-1000", "1", "Cinq lettres", Some("A"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert!(page.transcript.sessions[0].courses.is_empty());
    }

    #[test]
    fn a_banner_split_across_a_line_break_is_still_recognized() {
        // a plausible real Capsule export variant: the banner text is split
        // by a line break rather than kept on one line, so the raw text
        // node reads "CRÉDITS DE\nL'UNIVERSITÉ LAVAL" — collapse_text must
        // normalize this the same way it does for course-row cells, or the
        // banner goes unrecognized and its rows are silently skipped
        let html = document(
            &[
                banner("CRÉDITS DE\nL'UNIVERSITÉ LAVAL"),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.transcript.sessions.len(), 1);
        assert_eq!(
            page.transcript.sessions[0].section,
            TranscriptSection::Laval
        );
        assert_eq!(page.transcript.sessions[0].courses.len(), 1);
    }

    #[test]
    fn an_unknown_banners_rows_are_skipped() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
                banner("INFORMATIONS ÉTUDIANTES"),
                course_row("GEX-9999", "1", "Égaré", Some("A"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.transcript.sessions.len(), 1);
        assert_eq!(page.transcript.sessions[0].courses.len(), 1);
    }

    #[test]
    fn a_session_label_with_no_active_section_is_ignored() {
        // a `fieldOrangetextbold` row read before the relevé's grammar has
        // started (no recognized banner yet) has nothing to attach to and
        // is silently ignored, like the course rows around it
        let html = document(
            &[
                session_header("Automne 2024", None),
                banner(LAVAL_BANNER),
                session_header("Hiver 2025", None),
                course_row("GCI-1004", "1", "Fluides", Some("A"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.transcript.sessions.len(), 1);
        assert_eq!(
            page.transcript.sessions[0].semester,
            Semester {
                season: Season::Winter,
                year: 2025
            }
        );
    }

    // --- Anomalies ---

    #[test]
    fn a_course_row_of_the_wrong_shape_is_reported() {
        let html = document(&[
            banner(LAVAL_BANNER),
            session_header("Automne 2024", None),
            r#"<tr><td class="dddefault">BIO-1904</td><td class="dddefault">1</td></tr>"#
                .to_string(),
        ]
        .concat());

        let page = ok_page(&html);
        assert!(page.transcript.sessions[0].courses.is_empty());
        assert_eq!(page.anomalies.len(), 1, "got {:?}", page.anomalies);
        assert_eq!(malformed_entry(&page.anomalies[0]).0, "course row");
    }

    #[test]
    fn a_bad_cycle_value_is_reported() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "9", "Botanique", Some("B"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.transcript.sessions[0].courses.is_empty());
        assert_eq!(page.anomalies.len(), 1, "got {:?}", page.anomalies);
        assert_eq!(malformed_entry(&page.anomalies[0]).0, "cycle");
    }

    #[test]
    fn a_bad_credits_value_is_reported() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "trois"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.transcript.sessions[0].courses.is_empty());
        assert_eq!(page.anomalies.len(), 1, "got {:?}", page.anomalies);
        assert_eq!(malformed_entry(&page.anomalies[0]).0, "credits");
    }

    #[test]
    fn a_bad_session_label_is_reported() {
        for label in [
            "Printemps 2026",
            "Automne",
            "Automne vingt",
            "Automne 2024 bis",
            // outside `Semester`'s two-digit domain (2000-2099): neither
            // round-trips through `Display`/`FromStr`
            "Hiver 1998",
            "Automne 2105",
        ] {
            let html = document(
                &[banner(LAVAL_BANNER), session_header(label, None)].concat(),
            );

            let page = ok_page(&html);
            assert_eq!(
                page.anomalies.len(),
                1,
                "for {label:?}, got {:?}",
                page.anomalies
            );
            assert_eq!(
                malformed_entry(&page.anomalies[0]),
                ("span.fieldOrangetextbold", label)
            );
            assert!(page.transcript.sessions.is_empty());
        }
    }

    #[test]
    fn a_course_row_before_any_session_header_is_reported() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.transcript.sessions.is_empty());
        assert_eq!(page.anomalies.len(), 1, "got {:?}", page.anomalies);
        let (selector, raw) = malformed_entry(&page.anomalies[0]);
        assert_eq!(selector, "course row");
        assert!(raw.contains("before any session header"), "{raw}");
    }

    #[test]
    fn a_course_row_under_no_recognized_section_is_reported() {
        // a sigle-shaped row read before the relevé's grammar has started at
        // all: the founding case the plan calls out on its own, distinct
        // from an unrecognized banner's rows (which are skipped in silence)
        let html = document(
            &[
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("GCI-1004", "1", "Fluides", Some("A"), "3"),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert_eq!(page.transcript.sessions[0].courses.len(), 1);
        assert_eq!(page.anomalies.len(), 1, "got {:?}", page.anomalies);
        let (selector, raw) = malformed_entry(&page.anomalies[0]);
        assert_eq!(selector, "course row");
        assert!(raw.contains("under no recognized section"), "{raw}");
    }

    #[test]
    fn a_page_without_any_recognized_banner_is_missing_the_transcript() {
        let html = document("<tr><td>rien</td></tr>");
        match parse(&html) {
            Err(ParseError::MissingElement { selector }) => {
                assert_eq!(selector, "th.ddtitle");
            }
            other => panic!("expected MissingElement, got {other:?}"),
        }
    }

    #[test]
    fn a_page_with_only_unrecognized_banners_names_what_is_missing() {
        // `th.ddtitle` elements genuinely exist here (unlike the test
        // above) — the diagnostic must not claim the element itself is
        // missing (TRU), it must name the recognized banner text that
        // never showed up
        let html = document(&banner("INFORMATIONS ÉTUDIANTES"));
        match parse(&html) {
            Err(ParseError::MissingElement { selector }) => {
                assert_ne!(
                    selector, "th.ddtitle",
                    "the element was found; a recognized banner was not"
                );
                assert!(selector.contains(LAVAL_BANNER));
                assert!(selector.contains(RECOGNIZED_BANNER));
                assert!(selector.contains(IN_PROGRESS_BANNER));
            }
            other => panic!("expected MissingElement, got {other:?}"),
        }
    }

    // --- program_floor -----------------------------------------------------

    #[test]
    fn the_fixture_program_floor_is_the_bacs_own_frequentation_start() {
        let html = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test_cases/transcripts/exemple.html"
        ));
        let page = ok_page(html);
        assert_eq!(
            page.program_floor,
            Some(Semester {
                season: Season::Fall,
                year: 2024
            }),
            "Baccalauréat en génie des eaux, Fréquentation Automne 2024"
        );
    }

    #[test]
    fn program_floor_is_the_earliest_en_cours_frequentation_regardless_of_order(
    ) {
        // a finished certificat before the floor never counts (no « En
        // cours »); a microprogramme listed *after* the bac but with an
        // earlier « Fréquentation » still lowers the floor — the algorithm
        // reads every program, not just the first
        let html = document(
            &[
                banner(PROGRAM_BANNER),
                program_row("Programme:", Some("Certificat en xyz")),
                program_row(
                    "Fréquentation:",
                    Some("Automne 2018 à Hiver 2020"),
                ),
                program_row("Diplôme obtenu", Some("Certificat")),
                program_row(
                    "Programme:",
                    Some("Baccalauréat en génie des eaux"),
                ),
                program_row("Fréquentation:", Some("Hiver 2022 à ce jour")),
                program_row("En cours", None),
                program_row("Programme:", Some("Microprogramme de stage")),
                program_row("Fréquentation:", Some("Automne 2020 à ce jour")),
                program_row("En cours", None),
                banner(RECOGNIZED_BANNER),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(
            page.program_floor,
            Some(Semester {
                season: Season::Fall,
                year: 2020
            }),
            "the microprogramme's A20 beats the bac's own H22, \
             the certificat's A18 never counts (finished, not en cours)"
        );
    }

    #[test]
    fn an_unparseable_frequentation_is_ignored_without_an_anomaly() {
        // not part of the relevé grammar proper (ADR
        // `2026-08-import-de-releve-capsule`): a season it does not
        // recognize, a value missing its year, and a non-numeric year all
        // simply fail to set the floor
        let html = document(
            &[
                banner(PROGRAM_BANNER),
                program_row("Programme:", Some("Programme mystère")),
                program_row("Fréquentation:", Some("Printemps 2024")),
                program_row("En cours", None),
                program_row("Programme:", Some("Autre programme")),
                program_row("Fréquentation:", Some("Automne")),
                program_row("En cours", None),
                program_row("Programme:", Some("Encore un programme")),
                program_row("Fréquentation:", Some("Automne vingt")),
                program_row("En cours", None),
                banner(RECOGNIZED_BANNER),
            ]
            .concat(),
        );

        let page = ok_page(&html);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
        assert_eq!(page.program_floor, None);
    }

    #[test]
    fn no_programme_section_leaves_the_floor_unset() {
        let html = document(
            &[
                banner(LAVAL_BANNER),
                session_header("Automne 2024", None),
                course_row("BIO-1904", "1", "Botanique", Some("B"), "3"),
            ]
            .concat(),
        );
        let page = ok_page(&html);
        assert_eq!(page.program_floor, None);
    }
}
