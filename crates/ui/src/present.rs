use crate::data::{fnv1a_64, DataError};

// ERR-1: every user-facing error states five things, in French — what
// happened, what the app did about it, what is affected, what to do now,
// and a copyable id. `detail` is the technical text one click away (ERR-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiError {
    pub what: String,
    pub reaction: String,
    pub affected: String,
    pub action: String,
    pub id: String,
    pub detail: String,
}

pub fn present_data_error(error: &DataError) -> UiError {
    let what = match error {
        DataError::Fetch { file, .. } => {
            format!(
                "Le fichier de données « {file} » n'a pas pu être téléchargé."
            )
        }
        DataError::Parse { file, .. } => {
            format!("Le fichier de données « {file} » est illisible.")
        }
    };
    UiError {
        what,
        reaction: "L'application ne démarre pas tant que le catalogue \
                   n'est pas chargé; rien n'a été perdu."
            .to_string(),
        affected: "Tout l'affichage — aucun cours ni programme n'est \
                   disponible."
            .to_string(),
        action: "Vérifiez votre connexion puis rechargez la page; si \
                 l'erreur persiste, signalez-la avec l'identifiant \
                 ci-dessous."
            .to_string(),
        id: error_id(&error.to_string()),
        detail: error.to_string(),
    }
}

// deterministic (fnv of the message): the same failure always carries the
// same id, so two reports of it can be recognized as one
// A correction the catalogue could not honour, said in French. Every case
// names the course and what the student can do about it — a correction that
// quietly did nothing would be worse than none at all.
pub fn present_override_note(
    note: &ulaval_scheduler_core::OverrideNote,
) -> String {
    use ulaval_scheduler_core::OverrideNote;
    match note {
        OverrideNote::Unparsed { code, error } => format!(
            "Préalables de {code} : la correction n'a pas pu être lue \
             ({error}); ceux du répertoire s'appliquent toujours."
        ),
        OverrideNote::UnknownCode { code } => format!(
            "Préalables de {code} : ce cours n'est pas au catalogue, la \
             correction ne s'applique à rien."
        ),
        OverrideNote::OfficialChanged { code, was, now } => format!(
            "Préalables de {code} : le répertoire a changé depuis votre \
             correction (« {was} » est devenu « {now} »). Votre version \
             reste appliquée."
        ),
    }
}

// What a correction being typed would mean, echoed before it is committed
// (INP-6). `valid` alone would be colour-shaped feedback, so `echo` always
// carries the same verdict in words (INP-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrereqDraft {
    pub valid: bool,
    pub echo: String,
}

// The parser's guards name themselves in English, like the rest of the
// code; on screen they are read by a student, in French. The fallback is
// the label itself rather than a vague « expression invalide »: a guard
// added later must show up as something, never be swallowed.
fn prereq_fault(label: &str) -> String {
    match label {
        "two operands in a row" => {
            "deux termes se suivent sans ET ni OU entre eux"
        }
        "( where an operator was expected" => {
            "une parenthèse ouvre là où ET ou OU était attendu"
        }
        ") without a left operand" => {
            "une parenthèse se ferme sans terme devant elle"
        }
        "unmatched )" => "une parenthèse fermante n'a pas d'ouvrante",
        "ET without a left operand" => "ET n'a pas de terme à sa gauche",
        "OU without a left operand" => "OU n'a pas de terme à sa gauche",
        "expression ends on an operator" => {
            "l'expression se termine sur un opérateur"
        }
        "unclosed (" => "une parenthèse reste ouverte",
        other => other,
    }
    .to_string()
}

// the same ceiling the solver's own flattening uses — a pathological
// expression must not walk forever
const MAX_DRAFT_NODES: usize = 10_000;

pub fn present_prereq_draft(text: &str) -> PrereqDraft {
    use ulaval_scheduler_core::parse_prereq_tree;

    let text = text.trim();
    if text.is_empty() {
        return PrereqDraft {
            valid: true,
            echo: "compris : ce cours n'a aucun préalable.".to_string(),
        };
    }
    let tree = match parse_prereq_tree(text) {
        Ok(tree) => tree,
        Err(error) => {
            return PrereqDraft {
                valid: false,
                echo: format!(
                    "non lu : {} - la correction n'est pas appliquée.",
                    prereq_fault(&error.error)
                ),
            };
        }
    };
    // the operands no catalogue can check (an examination, a range of
    // course numbers) are the surprising half: the solver presumes them
    // rather than verifying them, and the student must know which
    let presumed = presumed_operands(&tree);
    let echo = if presumed.is_empty() {
        "compris.".to_string()
    } else {
        format!(
            "compris - {} sera présumé acquis, le solveur ne peut pas le \
             vérifier.",
            presumed.join(", ")
        )
    };
    PrereqDraft { valid: true, echo }
}

// a bounded walk, never a recursion: an arbitrarily deep expression is a
// student's typing, not a trusted input
fn presumed_operands(tree: &ulaval_scheduler_core::PrereqTree) -> Vec<String> {
    use ulaval_scheduler_core::PrereqTree;
    let mut presumed = Vec::new();
    let mut stack = vec![tree];
    for _ in 0..MAX_DRAFT_NODES {
        let Some(node) = stack.pop() else {
            break;
        };
        match node {
            PrereqTree::Raw { raw } => presumed.push(format!("« {raw} »")),
            PrereqTree::All { all } => stack.extend(all.iter()),
            PrereqTree::Any { any } => stack.extend(any.iter()),
            PrereqTree::Course(_) | PrereqTree::ProgramCredits { .. } => {}
        }
    }
    presumed
}

pub fn error_id(detail: &str) -> String {
    let hash = fnv1a_64(0xcbf2_9ce4_8422_2325, detail.as_bytes());
    format!("GH-{:08X}", (hash >> 32) as u32 ^ hash as u32)
}

// --- the weekly grid geometry ---------------------------------------------

use ulaval_scheduler_core::{Day, Section, Time};

use crate::data::Snapshot;
use crate::solve::WeeklySchedule;
use crate::state::{self, Plan};

// The whole teachable day, always shown — the axis never breathes with the
// data (notes 2026-08-13), so a block keeps its place when courses change.
// Data outside the frame still stretches it rather than being cut (TST-1).
pub const AXIS_START: u16 = 8 * 60 + 30;
pub const AXIS_END: u16 = 22 * 60 + 30;
pub const COLOR_SLOTS: usize = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct GridModel {
    // axis labels, one per hour — « 8:30 » … « 17:30 »
    pub hours: Vec<String>,
    pub start: u16,
    pub end: u16,
    pub days: Vec<DayColumn>,
    pub conflict: bool,
    // selected courses whose sections carry no weekly slot (à distance) —
    // listed under the grid, never interpolated (TRU-4)
    pub unplaced: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayColumn {
    pub label: &'static str,
    pub conflict: bool,
    pub blocks: Vec<Block>,
}

// One rendered block, all coordinates in percent of the day column; a
// ghost is an alternative option shown when its course is selected —
// clicking it pins `nrcs` (sémantique swap, ADR
// `2026-07-contrat-horaire-hebdomadaire-vers-ui`).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub code: String,
    pub title: String,
    pub detail: String,
    pub top: f32,
    pub height: f32,
    pub left: f32,
    pub width: f32,
    pub color: usize,
    pub ghost: bool,
    pub valid: bool,
    // this very block overlaps another selected block: the hatch goes
    // here, not on every slot of an unlucky course (a lone Saturday slot
    // stays clean even when its course clashes elsewhere)
    pub clash: bool,
    // the option's identity — the sorted NRC set a click pins
    pub nrcs: Vec<String>,
}

pub fn grid_model(
    schedule: &WeeklySchedule,
    snapshot: &Snapshot,
    ghosts_for: Option<&str>,
) -> GridModel {
    let mut raw: Vec<(usize, RawBlock)> = Vec::new();
    let mut unplaced = Vec::new();
    // a hybrid option repeats the same slot in twin sections: one block
    // per (course, day, time), never a duplicate (TST-1)
    let mut seen: std::collections::BTreeSet<(String, usize, u16, u16)> =
        std::collections::BTreeSet::new();
    for (i, course) in schedule.report.courses.iter().enumerate() {
        let color = i % COLOR_SLOTS;
        let title = course_title(snapshot, &course.code);
        let nrcs = option_nrcs(&course.selected);
        let mut placed = false;
        for section in &course.selected {
            for slot in &section.slots {
                placed = true;
                if !seen.insert((
                    course.code.clone(),
                    day_index(slot.day),
                    minutes(slot.start),
                    minutes(slot.end),
                )) {
                    continue;
                }
                raw.push((
                    day_index(slot.day),
                    RawBlock {
                        start: minutes(slot.start),
                        end: minutes(slot.end),
                        block: Block {
                            code: course.code.clone(),
                            title: title.to_string(),
                            detail: section_detail(&course.code, section),
                            top: 0.0,
                            height: 0.0,
                            left: 0.0,
                            width: 100.0,
                            color,
                            ghost: false,
                            valid: course.valid,
                            clash: false,
                            nrcs: nrcs.clone(),
                        },
                    },
                ));
            }
        }
        if !placed && !course.selected.is_empty() {
            unplaced.push(course.code.clone());
        }
        if ghosts_for == Some(course.code.as_str()) {
            for alternative in &course.alternatives {
                let nrcs = option_nrcs(&alternative.sections);
                for section in &alternative.sections {
                    for slot in &section.slots {
                        raw.push((
                            day_index(slot.day),
                            RawBlock {
                                start: minutes(slot.start),
                                end: minutes(slot.end),
                                block: Block {
                                    code: course.code.clone(),
                                    title: title.to_string(),
                                    detail: section_detail(
                                        &course.code,
                                        section,
                                    ),
                                    top: 0.0,
                                    height: 0.0,
                                    left: 0.0,
                                    width: 100.0,
                                    color,
                                    ghost: true,
                                    valid: alternative.valid,
                                    clash: false,
                                    nrcs: nrcs.clone(),
                                },
                            },
                        ));
                    }
                }
            }
        }
    }
    let (start, end) = axis_span(&raw);
    let days = build_days(raw, start, end);
    GridModel {
        hours: hour_labels(start, end),
        start,
        end,
        conflict: !schedule.report.valid,
        days,
        unplaced,
    }
}

struct RawBlock {
    start: u16,
    end: u16,
    block: Block,
}

fn course_title<'a>(snapshot: &'a Snapshot, code: &'a str) -> &'a str {
    snapshot
        .by_code
        .get(code)
        .map(|&index| snapshot.courses[index].title.as_str())
        .unwrap_or(code)
}

// « GCI-1007 - A », « GEX-4008 - Z1 - à distance » — the section letter
// when the page gave one, the mode when it is not the in-person default
// (the page carries no Cours/Labo/TD type — nothing is invented)
fn section_detail(code: &str, section: &Section) -> String {
    let mut parts = vec![code.to_string()];
    if let Some(letter) = &section.section {
        parts.push(letter.clone());
    }
    match section.mode {
        ulaval_scheduler_core::Mode::InPerson => {}
        ulaval_scheduler_core::Mode::Remote => {
            parts.push("à distance".to_string())
        }
        ulaval_scheduler_core::Mode::Hybrid => {
            parts.push("hybride".to_string())
        }
    }
    parts.join(" - ")
}

fn option_nrcs(sections: &[Section]) -> Vec<String> {
    let mut nrcs: Vec<String> =
        sections.iter().map(|section| section.nrc.clone()).collect();
    nrcs.sort();
    nrcs
}

fn minutes(time: Time) -> u16 {
    u16::from(time.hour) * 60 + u16::from(time.minute)
}

// the design frame, stretched (rounded to the half-hour) by any block
// outside it — data wins over the frame, the frame never cuts
fn axis_span(raw: &[(usize, RawBlock)]) -> (u16, u16) {
    let lowest = raw
        .iter()
        .map(|(_, block)| block.start)
        .min()
        .unwrap_or(AXIS_START);
    let highest = raw
        .iter()
        .map(|(_, block)| block.end)
        .max()
        .unwrap_or(AXIS_END);
    let start = AXIS_START.min(lowest - lowest % 30);
    let end = AXIS_END.max(highest + (30 - highest % 30) % 30);
    (start, end)
}

fn hour_labels(start: u16, end: u16) -> Vec<String> {
    (0..=(end - start) / 60)
        .map(|hour| {
            let minute = start + hour * 60;
            format!("{}:{:02}", minute / 60, minute % 60)
        })
        .collect()
}

const DAY_LABELS: [&str; 7] = [
    "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
];

fn day_index(day: Day) -> usize {
    match day {
        Day::Monday => 0,
        Day::Tuesday => 1,
        Day::Wednesday => 2,
        Day::Thursday => 3,
        Day::Friday => 4,
        Day::Saturday => 5,
        Day::Sunday => 6,
    }
}

// Monday→Friday always (spatial constancy, LAY-1); a weekend column only
// when a slot actually lands there
fn build_days(
    raw: Vec<(usize, RawBlock)>,
    start: u16,
    end: u16,
) -> Vec<DayColumn> {
    let span = f32::from(end - start);
    let weekend_used = raw.iter().map(|&(day, _)| day).max().unwrap_or(0);
    let day_count = 5usize.max(weekend_used + 1);
    let mut days: Vec<DayColumn> = DAY_LABELS[..day_count]
        .iter()
        .map(|&label| DayColumn {
            label,
            conflict: false,
            blocks: Vec::new(),
        })
        .collect();
    let mut per_day: Vec<Vec<RawBlock>> =
        (0..day_count).map(|_| Vec::new()).collect();
    for (day, block) in raw {
        per_day[day].push(block);
    }
    for (day, mut blocks) in per_day.into_iter().enumerate() {
        assign_lanes(&mut blocks);
        // the hatch marks actual overlap between selected blocks — the
        // report's per-course verdict stays on the status line
        let clashes: Vec<bool> = blocks
            .iter()
            .map(|one| {
                !one.block.ghost
                    && blocks.iter().any(|other| {
                        !other.block.ghost
                            && other.block.code != one.block.code
                            && one.start < other.end
                            && other.start < one.end
                    })
            })
            .collect();
        for (raw_block, clash) in blocks.into_iter().zip(clashes) {
            let mut block = raw_block.block;
            block.clash = clash;
            block.top = f32::from(raw_block.start - start) / span * 100.0;
            block.height =
                f32::from(raw_block.end - raw_block.start) / span * 100.0;
            days[day].conflict |= clash;
            days[day].blocks.push(block);
        }
    }
    days
}

// Side-by-side lanes for overlapping blocks (the design's conflict view):
// blocks sorted by start form clusters of transitive overlap; within a
// cluster each block takes the first lane free at its start, and every
// member is widened to the cluster's lane count.
fn assign_lanes(blocks: &mut [RawBlock]) {
    blocks.sort_by_key(|block| (block.start, block.end));
    let mut cluster_from = 0;
    let mut cluster_end = 0u16;
    let mut lanes: Vec<u16> = Vec::new();
    let mut assigned: Vec<usize> = vec![0; blocks.len()];
    for i in 0..blocks.len() {
        if i > 0 && blocks[i].start >= cluster_end {
            close_cluster(blocks, &assigned, cluster_from..i, lanes.len());
            cluster_from = i;
            lanes.clear();
        }
        let start = blocks[i].start;
        let lane = lanes
            .iter()
            .position(|&end| end <= start)
            .unwrap_or(lanes.len());
        if lane == lanes.len() {
            lanes.push(0);
        }
        lanes[lane] = blocks[i].end;
        assigned[i] = lane;
        cluster_end = cluster_end.max(blocks[i].end);
    }
    let len = blocks.len();
    close_cluster(blocks, &assigned, cluster_from..len, lanes.len());
}

fn close_cluster(
    blocks: &mut [RawBlock],
    assigned: &[usize],
    range: std::ops::Range<usize>,
    lane_count: usize,
) {
    let width = 100.0 / lane_count.max(1) as f32;
    for i in range {
        blocks[i].block.left = assigned[i] as f32 * width;
        blocks[i].block.width = width;
    }
}

// --- the schedule's status line -------------------------------------------

// never colour alone: the glyph and the wording carry the state (INP-3)
pub fn schedule_status(schedule: &WeeklySchedule, forced: bool) -> String {
    if schedule.report.courses.is_empty() {
        "aucun cours avec horaire dans cette session".to_string()
    } else if schedule.report.valid {
        if forced {
            // the student pinned at least one section by hand — the word
            // « automatique » would lie (rapport étudiante 2026-08-13)
            "sections forcées - sans conflit ✓".to_string()
        } else {
            "combinaison automatique - sans conflit ✓".to_string()
        }
    } else {
        "⚠ conflit d'horaire — plages en cause hachurées".to_string()
    }
}

// « 3 cr » ou « 6–12 cr » — showing the interval whole is the UI's choice
// for a stage the student weights himself (plan § Source)
pub fn credits_label(credits: &ulaval_scheduler_core::Credits) -> String {
    match credits {
        ulaval_scheduler_core::Credits::Fixed(count) => format!("{count} cr"),
        ulaval_scheduler_core::Credits::Range { min, max } => {
            format!("{min}–{max} cr")
        }
    }
}

// --- the session ribbon ---------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RibbonCard {
    // 1-based index in the horizon
    pub index: usize,
    // « A1-A26 » — or « É27 » for an été strip
    pub label: String,
    pub summer: bool,
    pub credits: u32,
    // above the student's own cap — marked, never silent
    pub over_cap: bool,
    pub has_range: bool,
    pub codes: Vec<String>,
    // the free annotation (« à l'étranger »)
    pub special: Option<String>,
    pub current: bool,
    // the session sits before the real-world clock's semester — its
    // courses are the student's acquired past (purely visual, ADR
    // `2026-08-retrait-de-la-notion-de-cours-reussi`)
    pub passed: bool,
    // its weekly schedule clashes: the card must say so even when the
    // session is not displayed (rapport étudiante 2026-08-13)
    pub conflict: bool,
}

pub fn ribbon_model(
    snapshot: &Snapshot,
    plan: &Plan,
    current: usize,
    today: ulaval_scheduler_core::Semester,
) -> Vec<RibbonCard> {
    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let semesters = state::session_semesters(plan.start, &seasons);
    semesters
        .iter()
        .enumerate()
        .map(|(i, &semester)| {
            let index = i + 1;
            let codes = state::session_codes(plan, index);
            let credits = crate::solve::session_credits(snapshot, plan, index);
            RibbonCard {
                index,
                label: state::session_label(&semesters, i),
                summer: semester.season
                    == ulaval_scheduler_core::Season::Summer,
                credits: credits.total,
                over_cap: credits.total > plan.credit_cap,
                has_range: credits.has_range,
                passed: state::semester_precedes(semester, today),
                conflict: !codes.is_empty()
                    && !crate::solve::weekly_schedule(snapshot, plan, index)
                        .report
                        .valid,
                codes,
                special: plan.special.get(&index).cloned(),
                current: index == current,
            }
        })
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod override_note_tests {
    use ulaval_scheduler_core::OverrideNote;

    use super::{present_override_note, present_prereq_draft};

    #[test]
    fn an_empty_correction_reads_as_no_prerequisites_at_all() {
        let draft = present_prereq_draft("   ");
        assert!(draft.valid);
        assert!(draft.echo.contains("aucun préalable"), "{}", draft.echo);
    }

    #[test]
    fn a_readable_expression_says_so_without_repeating_it() {
        let draft = present_prereq_draft("GCI-1000 ET (MAT-1900 OU MAT-1902)");
        assert!(draft.valid);
        assert_eq!(draft.echo, "compris.");
    }

    #[test]
    fn an_operand_the_grammar_cannot_check_is_named_before_the_commit() {
        let draft = present_prereq_draft("Examen de langue OU GCI-1000");
        assert!(draft.valid, "{}", draft.echo);
        assert!(
            draft.echo.contains("« Examen de langue »"),
            "the student must see what the solver will only presume: {}",
            draft.echo
        );
        assert!(draft.echo.contains("présumé acquis"), "{}", draft.echo);
    }

    #[test]
    fn a_broken_expression_is_refused_in_words_not_only_in_colour() {
        let draft = present_prereq_draft("GCI-1000 ET");
        assert!(!draft.valid);
        assert_eq!(
            draft.echo,
            "non lu : l'expression se termine sur un opérateur - la \
             correction n'est pas appliquée.",
            "the fault is named in French, like everything on screen"
        );
        assert!(
            draft.echo.contains("n'est pas appliquée"),
            "the echo says the consequence, not just the fault: {}",
            draft.echo
        );
    }

    #[test]
    fn every_guard_of_the_grammar_names_itself_in_french() {
        // the eight the parser can raise, plus the fallback that keeps a
        // future one visible instead of swallowed
        for (raw, expected) in [
            ("( GLG-1900 ) GLG-1000", "deux termes se suivent"),
            ("GLG-1000 (GLG-1900)", "une parenthèse ouvre là où"),
            ("()", "se ferme sans terme devant"),
            ("GLG-1000 )", "n'a pas d'ouvrante"),
            ("GLG-1000 ET", "l'expression se termine sur un opérateur"),
            ("ET GLG-1000", "ET n'a pas de terme à sa gauche"),
            ("OU GLG-1000", "OU n'a pas de terme à sa gauche"),
            ("( GLG-1000", "une parenthèse reste ouverte"),
        ] {
            let draft = present_prereq_draft(raw);
            assert!(!draft.valid, "{raw:?} must be refused");
            assert!(
                draft.echo.contains(expected),
                "{raw:?}: expected {expected:?}, got {:?}",
                draft.echo
            );
        }
        assert_eq!(super::prereq_fault("brand new guard"), "brand new guard");
    }

    #[test]
    fn a_credits_threshold_needs_no_presumption() {
        let draft = present_prereq_draft("GEX, Crédits exigés : 60");
        assert!(draft.valid);
        assert_eq!(draft.echo, "compris.");
    }

    #[test]
    fn every_refused_correction_names_the_course_and_what_happened() {
        let cases = [
            (
                OverrideNote::Unparsed {
                    code: "GCI-2000".to_string(),
                    error: "expression ends on an operator".to_string(),
                },
                "n'a pas pu être lue",
            ),
            (
                OverrideNote::UnknownCode {
                    code: "GCI-2000".to_string(),
                },
                "n'est pas au catalogue",
            ),
            (
                OverrideNote::OfficialChanged {
                    code: "GCI-2000".to_string(),
                    was: "GCI-1000".to_string(),
                    now: "GCI-1005".to_string(),
                },
                "a changé depuis votre correction",
            ),
        ];
        for (note, expected) in cases {
            let message = present_override_note(&note);
            assert!(message.contains("GCI-2000"), "{message}");
            assert!(message.contains(expected), "{message}");
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn both_data_errors_present_their_five_parts_in_french() {
        let fetch = present_data_error(&DataError::Fetch {
            file: "cours.json".to_string(),
            detail: "HTTP 404".to_string(),
        });
        assert!(fetch.what.contains("cours.json"));
        assert!(fetch.what.contains("téléchargé"));
        let parse = present_data_error(&DataError::Parse {
            file: "meta.json".to_string(),
            detail: "expected value".to_string(),
        });
        assert!(parse.what.contains("illisible"));
        for error in [&fetch, &parse] {
            assert!(!error.reaction.is_empty());
            assert!(!error.affected.is_empty());
            assert!(!error.action.is_empty());
            assert!(error.id.starts_with("GH-"));
            assert!(!error.detail.is_empty());
        }
    }

    #[test]
    fn the_id_is_deterministic_and_separates_distinct_failures() {
        assert_eq!(error_id("same"), error_id("same"));
        assert_ne!(error_id("same"), error_id("other"));
        assert_eq!(error_id("x").len(), "GH-".len() + 8);
    }

    // --- grid geometry ---

    use ulaval_scheduler_core::{Alternative, CourseReport, ScheduleReport};

    use crate::data::{parse_data, RawData};

    fn snapshot() -> Snapshot {
        parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"Hydrologie","credits":3,
                   "cycle":1,"prerequisites":null,"equivalents":[],
                   "seasons":{}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn section(json: &str) -> Section {
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"))
    }

    fn monday(_code: &str, nrc: &str, start: &str, end: &str) -> Section {
        section(&format!(
            r#"{{"nrc":"{nrc}","section":"A","mode":"in-person","slots":[
                {{"day":"monday","start":"{start}","end":"{end}"}}]}}"#
        ))
    }

    fn course(
        code: &str,
        valid: bool,
        selected: Vec<Section>,
    ) -> CourseReport {
        CourseReport {
            code: code.to_string(),
            valid,
            selected,
            alternatives: Vec::new(),
        }
    }

    fn wrap(courses: Vec<CourseReport>, valid: bool) -> WeeklySchedule {
        WeeklySchedule {
            report: ScheduleReport { valid, courses },
            excluded: Vec::new(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn blocks_land_at_their_time_with_title_detail_and_colour() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "08:30", "11:20")],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.start, AXIS_START);
        assert_eq!(grid.end, AXIS_END);
        assert_eq!(grid.hours.first().map(String::as_str), Some("8:30"));
        assert_eq!(grid.hours.last().map(String::as_str), Some("22:30"));
        let block = &grid.days[0].blocks[0];
        assert_eq!(block.title, "Hydrologie", "title read off the snapshot");
        assert_eq!(block.detail, "GEX-1000 - A");
        assert_eq!(block.color, 0);
        assert!((block.top - 0.0).abs() < f32::EPSILON);
        assert!((block.height - 170.0 / 840.0 * 100.0).abs() < 0.01);
        assert!((block.width - 100.0).abs() < f32::EPSILON);
        assert!(!grid.conflict);
        assert_eq!(grid.days.len(), 5, "Lundi→Vendredi, no weekend");
    }

    #[test]
    fn ghosts_appear_only_for_the_selected_course_with_their_nrcs() {
        let mut with_ghost = course(
            "GEX-1000",
            true,
            vec![monday("GEX-1000", "111", "08:30", "09:20")],
        );
        with_ghost.alternatives = vec![Alternative {
            sections: vec![
                section(
                    r#"{"nrc":"333","section":"B","mode":"in-person","slots":[
                        {"day":"tuesday","start":"12:30","end":"15:20"}]}"#,
                ),
                monday("GEX-1000", "222", "14:30", "15:20"),
            ],
            valid: false,
        }];
        let schedule = wrap(vec![with_ghost], true);

        let silent = grid_model(&schedule, &snapshot(), None);
        assert_eq!(silent.days[1].blocks.len(), 0, "no ghost unrequested");

        let grid = grid_model(&schedule, &snapshot(), Some("GEX-1000"));
        let ghost = &grid.days[1].blocks[0];
        assert!(ghost.ghost);
        assert!(!ghost.valid, "swap semantics carried through");
        assert_eq!(ghost.nrcs, ["222", "333"], "sorted option identity");
    }

    #[test]
    fn overlapping_blocks_share_their_column_in_lanes() {
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    false,
                    vec![monday("GEX-1000", "111", "08:30", "11:20")],
                ),
                course(
                    "GEX-2000",
                    false,
                    vec![monday("GEX-2000", "222", "09:30", "12:20")],
                ),
                course(
                    "GEX-3000",
                    true,
                    vec![monday("GEX-3000", "333", "14:30", "15:20")],
                ),
            ],
            false,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let blocks = &grid.days[0].blocks;
        assert!((blocks[0].width - 50.0).abs() < f32::EPSILON);
        assert!((blocks[0].left - 0.0).abs() < f32::EPSILON);
        assert!((blocks[1].width - 50.0).abs() < f32::EPSILON);
        assert!((blocks[1].left - 50.0).abs() < f32::EPSILON);
        assert!(
            (blocks[2].width - 100.0).abs() < f32::EPSILON,
            "the later block starts its own cluster, full width"
        );
        assert!(grid.days[0].conflict, "the day carries the warning");
        assert!(grid.conflict);
        assert_eq!(
            schedule_status(&schedule, false),
            "⚠ conflit d'horaire — plages en cause hachurées"
        );
    }

    #[test]
    fn twin_hybrid_sections_draw_one_block_not_two() {
        // the same slot in two sections of one option (hybrid pattern)
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![
                    monday("GEX-1000", "111", "08:30", "09:20"),
                    monday("GEX-1000", "222", "08:30", "09:20"),
                ],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days[0].blocks.len(), 1, "deduplicated");
        assert!(!grid.days[0].blocks[0].clash, "no self-conflict");
    }

    #[test]
    fn the_hatch_marks_only_the_overlapping_blocks() {
        // GEX-1000 clashes GEX-2000 on monday but its thursday slot is
        // alone: the thursday block stays clean, the day unmarked
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    false,
                    vec![section(
                        r#"{"nrc":"111","section":"A","mode":"in-person",
                            "slots":[
                              {"day":"monday","start":"08:30","end":"11:20"},
                              {"day":"thursday","start":"08:30","end":"09:20"}
                            ]}"#,
                    )],
                ),
                course(
                    "GEX-2000",
                    false,
                    vec![monday("GEX-2000", "222", "09:30", "12:20")],
                ),
            ],
            false,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert!(grid.days[0].blocks.iter().all(|block| block.clash));
        assert!(grid.days[0].conflict);
        assert!(!grid.days[3].blocks[0].clash, "the lone slot is clean");
        assert!(!grid.days[3].conflict, "Jeudi unmarked");
    }

    #[test]
    fn a_freed_lane_is_reused_by_a_later_block() {
        // A 8:30–9:20 (lane 0), B 8:30–10:20 (lane 1), C 9:30–10:20 —
        // still overlapping B, so same cluster, but lane 0 is free again
        let schedule = wrap(
            vec![
                course(
                    "GEX-1000",
                    true,
                    vec![monday("GEX-1000", "111", "08:30", "09:20")],
                ),
                course(
                    "GEX-2000",
                    true,
                    vec![monday("GEX-2000", "222", "08:30", "10:20")],
                ),
                course(
                    "GEX-3000",
                    true,
                    vec![monday("GEX-3000", "333", "09:30", "10:20")],
                ),
            ],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        let blocks = &grid.days[0].blocks;
        assert!((blocks[2].left - 0.0).abs() < f32::EPSILON, "lane reused");
        assert!((blocks[2].width - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_weekend_slot_adds_its_column_and_stretches_the_axis() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":null,"mode":"hybrid","slots":[
                        {"day":"saturday","start":"18:00","end":"22:45"}]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 6, "Samedi included");
        assert_eq!(grid.days[5].label, "Samedi");
        assert_eq!(grid.end, 23 * 60, "rounded up to the half-hour");
        assert_eq!(
            grid.days[5].blocks[0].detail, "GEX-1000 - hybride",
            "no section letter, the mode says what it is"
        );
    }

    #[test]
    fn every_weekday_lands_in_its_own_column() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":"R","mode":"remote","slots":[
                        {"day":"tuesday","start":"08:30","end":"09:20"},
                        {"day":"wednesday","start":"08:30","end":"09:20"},
                        {"day":"thursday","start":"08:30","end":"09:20"},
                        {"day":"friday","start":"08:30","end":"09:20"},
                        {"day":"sunday","start":"08:30","end":"09:20"}]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 7, "a Sunday slot opens the whole week");
        for day in [1, 2, 3, 4, 6] {
            assert_eq!(grid.days[day].blocks.len(), 1, "day {day}");
        }
        assert_eq!(
            grid.days[1].blocks[0].detail, "GEX-1000 - R - à distance",
            "a remote slot says so on the block"
        );
    }

    #[test]
    fn a_remote_course_without_slots_is_listed_never_interpolated() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![section(
                    r#"{"nrc":"111","section":"Z1","mode":"remote",
                        "slots":[]}"#,
                )],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.unplaced, ["GEX-1000"]);
        assert!(grid.days.iter().all(|day| day.blocks.is_empty()));
    }

    #[test]
    fn an_early_slot_stretches_the_axis_downward() {
        let schedule = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "07:15", "08:20")],
            )],
            true,
        );
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.start, 7 * 60, "floored to the half-hour");
    }

    #[test]
    fn an_empty_schedule_keeps_the_frame_and_says_so() {
        let schedule = wrap(Vec::new(), true);
        let grid = grid_model(&schedule, &snapshot(), None);
        assert_eq!(grid.days.len(), 5);
        assert_eq!(grid.start, AXIS_START);
        assert_eq!(
            schedule_status(&schedule, false),
            "aucun cours avec horaire dans cette session"
        );
        let one = wrap(
            vec![course(
                "GEX-1000",
                true,
                vec![monday("GEX-1000", "111", "08:30", "09:20")],
            )],
            true,
        );
        assert_eq!(
            schedule_status(&one, false),
            "combinaison automatique - sans conflit ✓"
        );
        assert_eq!(
            schedule_status(&one, true),
            "sections forcées - sans conflit ✓",
            "a hand-pinned section must not claim « automatique »"
        );
    }

    #[test]
    fn credit_labels_show_the_whole_interval() {
        assert_eq!(
            credits_label(&ulaval_scheduler_core::Credits::Fixed(3)),
            "3 cr"
        );
        assert_eq!(
            credits_label(&ulaval_scheduler_core::Credits::Range {
                min: 6,
                max: 12
            }),
            "6–12 cr"
        );
    }

    // --- ribbon ---

    #[test]
    fn the_ribbon_walks_the_horizon_with_credits_states_and_annotations() {
        let snapshot = parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],"seasons":{}},
                  {"code":"GEX-2000","title":"T","credits":4,"cycle":1,
                   "prerequisites":null,"equivalents":[],"seasons":{}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let plan = Plan {
            study_sessions: 4,
            displayed_placement: std::collections::BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GEX-2000".to_string(), 2),
            ]),
            special: std::collections::BTreeMap::from([(
                3,
                "à l'étranger".to_string(),
            )]),
            ..Plan::default()
        };
        // the real-world clock sits in H27: A26 is over, H27 is not
        let today = "H27"
            .parse::<ulaval_scheduler_core::Semester>()
            .unwrap_or_else(|e| panic!("{e}"));
        let ribbon = ribbon_model(&snapshot, &plan, 2, today);
        // A26 H27 É27 A27 H28 É28 : 4 study sessions + the étés
        assert_eq!(ribbon.len(), 6);
        assert_eq!(ribbon[0].label, "A1-A26");
        assert!(ribbon[0].passed, "A26 precedes today's H27");
        assert!(!ribbon[0].conflict, "no drawable clash here");
        assert_eq!(ribbon[0].credits, 3);
        assert_eq!(ribbon[1].codes, ["GEX-2000"]);
        assert!(ribbon[1].current);
        assert!(!ribbon[1].passed, "the running semester is not past");
        assert!(ribbon[2].summer);
        assert_eq!(ribbon[2].special.as_deref(), Some("à l'étranger"));
        assert!(!ribbon[3].passed, "the future is not past either");
    }

    #[test]
    fn a_clashing_session_marks_its_ribbon_card() {
        // two fall courses with one overlapping monday option each: the
        // card must warn even when the session is not displayed
        let snapshot = parse_data(
            &RawData {
                courses: r#"{"courses":[
                  {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],
                   "seasons":{"fall":{"last_offered":2026,"options":[
                     [{"nrc":"111","section":"A","mode":"in-person","slots":[
                        {"day":"monday","start":"08:30","end":"11:20"}]}]
                   ]}}},
                  {"code":"GEX-2000","title":"T","credits":3,"cycle":1,
                   "prerequisites":null,"equivalents":[],
                   "seasons":{"fall":{"last_offered":2026,"options":[
                     [{"nrc":"222","section":"A","mode":"in-person","slots":[
                        {"day":"monday","start":"09:30","end":"12:20"}]}]
                   ]}}}
                ]}"#
                .to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = Plan {
            study_sessions: 2,
            ..Plan::default()
        };
        plan.manual
            .insert(1, vec!["GEX-1000".to_string(), "GEX-2000".to_string()]);
        let today = "H26"
            .parse::<ulaval_scheduler_core::Semester>()
            .unwrap_or_else(|e| panic!("{e}"));
        let ribbon = ribbon_model(&snapshot, &plan, 1, today);
        assert!(ribbon[0].conflict, "the clash marks the card");
        assert!(!ribbon[1].conflict);
    }
}
