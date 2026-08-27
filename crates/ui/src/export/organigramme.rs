// The pure model of the printed « organigramme » document — entête avec
// bloc méta, bandeau de stats, colonnes de sessions, étés entre les
// colonnes, jetons de préalables, pied règles / exigences / légende —
// reproducing the token grammar of `gex_organigramme.pdf`'s « Note 1 »
// legend and the approved mockup design (ADR
// `2026-08-refonte-du-design-imprime-de-lorganigramme`): a letter token
// right of the box that serves as a prerequisite (jeton de sortie), the
// same letter left of every box that requires it (jeton d'entrée), shaded
// when the requirement is concomitant, and a numeric token for a
// program-credits threshold. No Dioxus, no web-sys, no clock:
// `components/print/organigramme.rs` renders this model.

use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    coverage_report, horizon_sessions, Constraint, Course, CoverageReport,
    LanguageQualification, LanguageRequirement, PrereqTree, Prerequisites,
    Program, Rule, RuleCourses, RuleReport, RuleStatus, Scope, Season,
    Semester, PREPARATORY_RULE_TITLE, STAGES_RULE_TITLE,
};

use crate::data::{Snapshot, OUT_OF_PROGRAM_RULE_TITLE};
use crate::export::provenance::{export_provenance, ExportProvenance};
use crate::present::credits_label;
use crate::state::{self, Plan};

// far above any real prerequisite tree; mirrors `core::organigramme`'s own
// cap — bounds the flatten loop, no recursion
const MAX_TREE_NODES: usize = 10_000;

// bijective base-26 has at most ~14 digits for a usize on a 64-bit target
// (log26(2^64) ≈ 13.6); 20 leaves ample headroom while staying an explicit,
// verifiable bound rather than an open-ended loop
const MAX_LETTER_DIGITS: usize = 20;

#[derive(Debug, Clone, PartialEq)]
pub struct OrganigrammeDocument {
    // « Organigramme des cours » — the small uppercase line over the title
    pub kicker: String,
    pub program_title: String,
    // « B-GEX, version A24 » / « … — cheminement type »
    pub subtitle: String,
    pub meta: MetaBlock,
    pub stats: Vec<Stat>,
    // one column per automne/hiver session — étés live in `summers`
    pub columns: Vec<Column>,
    // real summer sessions with courses, straddling the columns around
    // them; in ordinal mode the unplaced stage courses of the « Stages »
    // rule join them at the official document's fixed positions (the été
    // after each hiver), an été with nothing at all is not drawn
    pub summers: Vec<SummerGroup>,
    // the official document's small rules table: one row per rule of the
    // effective program (program scope plus the chosen concentration and
    // profile). The document's own per-cheminement columns (Commun / Plus
    // de conception / Moins de conception) are NOT computable from what
    // core exposes — no data source ties a course to one of those three
    // cheminements — so this table stays a flat list; nobody should re-add
    // the per-cheminement split without a new data source to compute it
    // from.
    pub rules_table: Vec<RulesRow>,
    // the « Exigences » sideboxes: reconnaissance des acquis, exigence
    // linguistique
    pub requirements: Vec<SideBox>,
    pub legend: Vec<LegendEntry>,
    pub disclaimer: String,
    pub notes: Vec<String>,
}

// The top-right provenance block (EXP-1): generation instant with zone,
// data vintage, versions, repository, and the share link back into the app.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaBlock {
    // « Généré le 2026-08-26 à 15:22 (UTC−04:00, America/Toronto) »
    pub generated: String,
    // « Données du répertoire : 2026-08-01 »
    pub data: String,
    // « app v0.1.0 — code b3f2a1c — données 4be09d21 »
    pub build: String,
    pub repo_label: String,
    pub repo_url: String,
    pub share_url: String,
    pub share_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    // « A2024 » (personal mode) / « A1 » (ordinal mode)
    pub label: String,
    // « 15 cr » — the session's placed total; `None` in ordinal mode, where
    // the cheminement type shows no per-column count
    pub credits: Option<String>,
    pub boxes: Vec<CourseBox>,
    // the grey « Cours option » box for this column, when at least one
    // unsatisfied rule placed a slot here — at most one per column
    // (`place_option_boxes` never pushes a second)
    pub options: Vec<OptionBox>,
}

// A summer group drawn between the columns around it — a real summer
// session, or an ordinal-mode stage slot. `first_column`/`last_column` are
// the 0-based indices of the straddled columns; the view turns them into a
// grid-column pair.
#[derive(Debug, Clone, PartialEq)]
pub struct SummerGroup {
    // « É2025 » — `None` in ordinal mode: the cheminement type's étés
    // carry no head at all (retour d'Antoine, 2026-08-26)
    pub label: Option<String>,
    pub credits: Option<String>,
    pub first_column: usize,
    pub last_column: usize,
    pub boxes: Vec<CourseBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionBox {
    pub heading: String,
    pub rules: Vec<OptionRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionRule {
    pub title: String,
    pub constraint: String,
    pub choices: Vec<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RulesRow {
    pub rule: String,
    pub constraint: String,
    pub chosen: String,
    // `false` renders the row as still-to-choose (italic, muted) — only an
    // `Incomplete` verdict; a constraint-less `Reported` rule has nothing
    // enumerable left to complete
    pub resolved: bool,
    // the counted courses' credit total, « — » when nothing is counted yet
    pub credits: String,
}

// One « Exigences » sidebox: lines of prose, each optionally led by a bold
// term (a course code, « Exigence linguistique. »).
#[derive(Debug, Clone, PartialEq)]
pub struct SideBox {
    pub lines: Vec<SideBoxLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SideBoxLine {
    pub lead: Option<String>,
    pub text: String,
}

// The legend's five fixed entries — typed, so the dumb view knows which
// swatch to draw without parsing prose.
#[derive(Debug, Clone, PartialEq)]
pub enum LegendEntry {
    Letter { text: String },
    Shaded { text: String },
    Credits { text: String },
    Chip { chip: String, text: String },
    // the dashed border: a course that is only a possibility (an unchosen
    // stage, a hors-programme addition)
    Optional { text: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CourseBox {
    pub code: String,
    pub title: String,
    pub credits: String,
    // one oklch hue per matière, ranked among every matière drawn in this
    // document (ADR `2026-08-couleurs-organigramme-par-matiere`)
    pub hue: f32,
    // « R3 » when a rule counted this course, « HP » when the Hors
    // programme rule did — mandatory courses carry none
    pub tag: Option<String>,
    // drawn with a dashed border: a hors-programme course, or an ordinal
    // stage slot past the first (INP-3: the chip and the legend carry the
    // meaning, the border is redundant)
    pub optional: bool,
    pub entry: Vec<Token>,
    pub exit: Option<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // `shaded`: the legend's « jeton ombré » — see `assign_tokens` below for
    // the exact rule this model applies (never set on an exit token: only
    // an entry token can be concomitant with the box that carries it)
    Letter { letter: String, shaded: bool },
    Credits { credits: u32 },
}

// One prerequisite-tree operand, flattened out of `All`/`Any` alike — the
// document draws every operand a course lists, it does not evaluate the
// boolean the tree encodes (that is the solver's job, not the printed
// document's).
#[derive(Debug, Clone, PartialEq)]
enum Operand {
    Course(String),
    // the répertoire's `*` (ADR `2026-08-etoile-de-concomitance-au-parsing`):
    // same source box, but the entry token it earns is shaded
    Concomitant(String),
    Credits(u32),
    // kept as a variant so the flatten names every grammar shape, even
    // though the document draws nothing for it (and no longer notes it)
    Raw,
}

// One session of the horizon while the document is being assembled — split
// into `Column`s and `SummerGroup`s once the tokens are assigned, so the
// token pass sees every box in one flat document order.
struct Slot {
    semester: Semester,
    // 1-based over the whole horizon, étés included — what
    // `state::session_codes` and `solve::session_credits` speak; 0 for a
    // synthetic stage slot, which no session owns
    session: usize,
    summer: bool,
    // an ordinal-mode stage slot: the 0-based index of the column its
    // straddle starts on (the kth hiver), resolved when the slot is made
    stage_anchor: Option<usize>,
    boxes: Vec<CourseBox>,
}

pub fn organigramme_document(
    snapshot: &Snapshot,
    plan: &Plan,
    program: Option<&Program>,
    generated_at: &str,
    share_url: &str,
) -> OrganigrammeDocument {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let semesters = state::session_semesters(plan.start, &seasons);
    let ordinal = is_ordinal(plan, program);

    let mut notes: Vec<String> = program
        .map(|program| program.notes.clone())
        .unwrap_or_default();

    // every placed code once, sorted — the stats' « placed » set
    let mut placed_codes: Vec<String> = (1..=semesters.len())
        .flat_map(|session| state::session_codes(plan, session))
        .collect();
    placed_codes.sort_unstable();
    placed_codes.dedup();

    // built in the same pass as the boxes themselves, so `by_code` and
    // `courses` are each read exactly once per box
    let mut slots: Vec<Slot> = Vec::with_capacity(semesters.len());
    let mut operands: Vec<Vec<Vec<Operand>>> =
        Vec::with_capacity(semesters.len());
    // first document position of each code — the token pass's only source
    // of truth for « is this a box in the document »
    let mut box_position: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for (index, &semester) in semesters.iter().enumerate() {
        let session = index + 1;
        let codes = state::session_codes(plan, session);
        let summer = semester.season == Season::Summer;
        if summer && codes.is_empty() {
            // an empty été is not drawn at all — the horizon offers one
            // after every hiver, most stay unused
            continue;
        }
        let mut boxes = Vec::with_capacity(codes.len());
        let mut slot_operands = Vec::with_capacity(codes.len());
        for (row_index, code) in codes.iter().enumerate() {
            box_position
                .entry(code.clone())
                .or_insert((slots.len(), row_index));
            let course: Option<&Course> = snapshot
                .by_code
                .get(code)
                .map(|&index| &snapshot.courses[index]);
            let (title, credits) = match course {
                Some(course) => {
                    (course.title.clone(), credits_label(&course.credits))
                }
                None => {
                    notes.push(format!(
                        "{code} : absent du catalogue de cours — affiché \
                         avec son code comme titre, sans crédits."
                    ));
                    (code.clone(), String::new())
                }
            };
            slot_operands.push(operands_of(
                course.and_then(|course| course.prerequisites.as_ref()),
            ));
            boxes.push(CourseBox {
                code: code.clone(),
                title,
                credits,
                // assigned over every box once synthetic stages have joined
                // the organigramme
                hue: 0.0,
                tag: None,
                optional: false,
                entry: Vec::new(),
                exit: None,
            });
        }
        slots.push(Slot {
            semester,
            session,
            summer,
            stage_anchor: None,
            boxes,
        });
        operands.push(slot_operands);
    }

    // the cheminement type also draws the unplaced stage courses, at the
    // official document's fixed positions — as ordinary boxes, so the
    // token pass sees them like any other course
    if ordinal {
        if let Some(program) = program {
            append_stage_slots(
                program,
                snapshot,
                plan,
                &semesters,
                &mut slots,
                &mut operands,
                &mut box_position,
                &mut notes,
            );
        }
    }

    assign_subject_hues(&mut slots);
    assign_tokens(&mut slots, &operands, &box_position);

    let (mut columns, mut summers, column_sessions) =
        split_slots(slots, snapshot, plan, ordinal);

    let concentration = plan
        .program
        .as_ref()
        .and_then(|choice| choice.concentration.as_deref());
    let profile = plan
        .program
        .as_ref()
        .and_then(|choice| choice.profile.as_deref());

    let mut rules_table = Vec::new();
    let mut report_kept: Option<CoverageReport> = None;

    // every field below depends on a chosen program: no program, nothing
    // to place or report — the columns and their courses still stand alone
    if let Some(program) = program {
        // the exact selection `panel::panel_model` feeds `coverage_report`
        // for the on-screen coverage: reused, not re-derived, so the
        // export never drifts from what the student already sees
        let mut selection = crate::panel::selection(plan);
        if plan.preparatory_done {
            selection.extend(crate::solve::preparatory_codes(program));
        }
        match coverage_report(
            program,
            concentration,
            profile,
            &selection,
            &snapshot.courses,
        ) {
            Ok(report) => {
                place_option_boxes(
                    &mut columns,
                    &column_sessions,
                    semesters.len(),
                    snapshot,
                    plan,
                    &report,
                    program,
                    concentration,
                    profile,
                    &mut notes,
                );
                rules_table = build_rules_table(
                    &report,
                    snapshot,
                    program,
                    concentration,
                    profile,
                );
                apply_tags(&mut columns, &mut summers, &report);
                report_kept = Some(report);
            }
            Err(error) => {
                // the counting failed, never the whole document — the
                // columns and their courses still render (ERR-5)
                notes.push(format!(
                    "Impossible de calculer les règles restantes : {error}"
                ));
            }
        }
    }

    let stats = build_stats(
        snapshot,
        plan,
        program,
        report_kept.as_ref(),
        &placed_codes,
        semesters.len(),
        ordinal,
    );
    let requirements = build_requirements(snapshot, plan, program);
    let subtitle = build_subtitle(plan, ordinal);

    let program_title = program
        .map(|program| program.title.clone())
        .or_else(|| plan.program.as_ref().map(|choice| choice.code.clone()))
        .unwrap_or_else(|| "Programme non choisi".to_string());

    let provenance = export_provenance(
        generated_at,
        snapshot.provenance.scraped_at.as_deref(),
    );

    OrganigrammeDocument {
        kicker: "Organigramme des cours".to_string(),
        program_title,
        subtitle,
        meta: build_meta(&provenance, generated_at, share_url),
        stats,
        columns,
        summers,
        rules_table,
        requirements,
        legend: build_legend(),
        disclaimer: "Document généré automatiquement à partir du répertoire \
                     de cours de l'Université Laval; en cas d'écart, la \
                     version officielle (ulaval.ca, Capsule) et la direction \
                     de programme prévalent."
            .to_string(),
        notes,
    }
}

// A director may move and freeze mandatory courses to author another
// cheminement type. The export becomes personal only when an explicit
// course choice is outside the mandatory lists of the effective program.
fn is_ordinal(plan: &Plan, program: Option<&Program>) -> bool {
    let choice = plan.program.as_ref();
    let concentration =
        choice.and_then(|choice| choice.concentration.as_deref());
    let profile = choice.and_then(|choice| choice.profile.as_deref());
    plan.electives
        .iter()
        .chain(plan.pinned_sessions.keys())
        .chain(plan.manual.values().flatten())
        .chain(plan.credited.iter())
        .chain(plan.rule_grants.keys())
        .all(|code| is_type_course(program, concentration, profile, code))
}

fn is_type_course(
    program: Option<&Program>,
    concentration: Option<&str>,
    profile: Option<&str>,
    code: &str,
) -> bool {
    is_mandatory_course(program, concentration, profile, code)
        || is_required_stage_course(program, code)
}

fn is_mandatory_course(
    program: Option<&Program>,
    concentration: Option<&str>,
    profile: Option<&str>,
    code: &str,
) -> bool {
    let Some(program) = program else {
        return false;
    };
    let scoped = program
        .concentrations
        .iter()
        .filter(|block| Some(block.title.as_str()) == concentration)
        .map(|block| &block.mandatory)
        .chain(
            program
                .profiles
                .iter()
                .filter(|block| Some(block.title.as_str()) == profile)
                .map(|block| &block.mandatory),
        );
    std::iter::once(&program.mandatory)
        .chain(scoped)
        .any(|mandatory| mandatory.iter().any(|held| held == code))
}

fn is_required_stage_course(program: Option<&Program>, code: &str) -> bool {
    let Some(program) = program else {
        return false;
    };
    let Some(stage_rule) = program
        .rules
        .iter()
        .find(|rule| rule.title == STAGES_RULE_TITLE)
    else {
        return false;
    };
    if !matches!(
        stage_rule.constraint,
        Some(Constraint::Course { min, .. }) if min > 0
    ) {
        return false;
    }
    let RuleCourses::List { courses } = &stage_rule.courses else {
        return false;
    };
    courses.first().is_some_and(|required| required == code)
}

// The complete organigramme owns one wheel: collect every represented
// matière first, then assign its alphabetical rank to every box. The second
// pass includes synthetic stage boxes and makes the hue independent of the
// session where a course was moved.
fn assign_subject_hues(slots: &mut [Slot]) {
    let mut subjects: Vec<String> = slots
        .iter()
        .flat_map(|slot| &slot.boxes)
        .map(|course_box| {
            crate::panel::subject_of(&course_box.code).to_string()
        })
        .collect();
    subjects.sort_unstable();
    subjects.dedup();

    for slot in slots {
        for course_box in &mut slot.boxes {
            course_box.hue = subject_hue(&subjects, &course_box.code);
        }
    }
}

fn subject_hue(subjects: &[String], code: &str) -> f32 {
    let subject = crate::panel::subject_of(code);
    let rank = subjects
        .binary_search_by(|candidate| candidate.as_str().cmp(subject))
        .expect(
            "subject comes from the same organigramme the list was built from",
        );
    rank as f32 / subjects.len() as f32 * 360.0
}

// The token-assignment walk: every box in document order (slot, then row),
// every operand of its own tree in tree order. A source code that is a box
// in the document earns its letter here, the first time some dependent
// needs it — never when the source box itself is walked, which is what
// keeps a source with no dependent silently letter-less. Operands the
// document cannot draw — an off-document source, a raw text — draw nothing
// and say nothing: the app's own panels already surface them, and a note
// per export was noise (retour d'Antoine, 2026-08-26).
fn assign_tokens(
    slots: &mut [Slot],
    operands: &[Vec<Vec<Operand>>],
    box_position: &BTreeMap<String, (usize, usize)>,
) {
    let mut letters: BTreeMap<String, String> = BTreeMap::new();
    let mut next_letter = 0usize;

    for slot_index in 0..slots.len() {
        for (row_index, box_operands) in
            operands[slot_index].iter().enumerate()
        {
            for operand in box_operands {
                match operand {
                    Operand::Course(source) | Operand::Concomitant(source) => {
                        let Some(&(source_slot, source_row)) =
                            box_position.get(source)
                        else {
                            continue;
                        };
                        let letter = letters
                            .entry(source.clone())
                            .or_insert_with(|| {
                                let letter = token_letter(next_letter);
                                next_letter += 1;
                                letter
                            })
                            .clone();
                        if slots[source_slot].boxes[source_row].exit.is_none()
                        {
                            slots[source_slot].boxes[source_row].exit =
                                Some(Token::Letter {
                                    letter: letter.clone(),
                                    shaded: false,
                                });
                        }
                        // the legend's « cours concomitant » : the
                        // répertoire's `*` on the prerequisite itself —
                        // the source may be taken before or alongside,
                        // whatever session the plan placed it in
                        let shaded =
                            matches!(operand, Operand::Concomitant(_));
                        slots[slot_index].boxes[row_index]
                            .entry
                            .push(Token::Letter { letter, shaded });
                    }
                    Operand::Credits(credits) => {
                        slots[slot_index].boxes[row_index]
                            .entry
                            .push(Token::Credits { credits: *credits });
                    }
                    Operand::Raw => {}
                }
            }
        }
    }
}

// The slots, tokens now assigned, become the document's two grids: one
// `Column` per automne/hiver, one `SummerGroup` per remaining été,
// straddling the column before and the column after it. Also returns each
// column's 1-based session (for `place_option_boxes`'s room math) and the
// hiver columns' indices (for `build_spans`'s fixed stage positions).
fn split_slots(
    slots: Vec<Slot>,
    snapshot: &Snapshot,
    plan: &Plan,
    ordinal: bool,
) -> (Vec<Column>, Vec<SummerGroup>, Vec<usize>) {
    let mut columns: Vec<Column> = Vec::new();
    let mut summers: Vec<SummerGroup> = Vec::new();
    let mut column_sessions: Vec<usize> = Vec::new();
    let mut study_ordinal = 0usize;

    for slot in slots {
        if let Some(first_column) = slot.stage_anchor {
            // a synthetic stage slot: the anchor was resolved against the
            // full horizon when the slot was made, and every real column
            // is already split (stage slots come last), so it lands
            // directly — merged into a real été group already straddling
            // the same pair, if one exists
            push_summer_boxes(
                &mut summers,
                None,
                None,
                first_column,
                first_column + 1,
                slot.boxes,
            );
            continue;
        }
        let total =
            crate::solve::session_credits(snapshot, plan, slot.session).total;
        // the cheminement type shows no per-session count — a generic
        // document claims no personal load
        let credits = (!ordinal).then(|| format!("{total} cr"));
        if slot.summer {
            let label = if ordinal {
                // an ordinal document names no year — its étés carry no
                // head at all
                None
            } else {
                Some(format!("É{}", slot.semester.year))
            };
            let first_column = columns.len().saturating_sub(1);
            summers.push(SummerGroup {
                label,
                credits,
                first_column,
                // the following column does not exist yet; clamped below
                // once the column count is final
                last_column: first_column + 1,
                boxes: slot.boxes,
            });
        } else {
            study_ordinal += 1;
            let letter = if slot.semester.season == Season::Fall {
                'A'
            } else {
                'H'
            };
            let label = if ordinal {
                format!("{letter}{study_ordinal}")
            } else {
                format!("{letter}{}", slot.semester.year)
            };
            column_sessions.push(slot.session);
            columns.push(Column {
                label,
                credits,
                boxes: slot.boxes,
                options: Vec::new(),
            });
        }
    }

    // a trailing été (or one before any column) still straddles a real
    // pair: clamp to the grid's own bounds rather than pointing past them
    for summer in &mut summers {
        if columns.len() >= 2 {
            summer.last_column = summer.last_column.min(columns.len() - 1);
            summer.first_column = summer.last_column.saturating_sub(1);
        } else {
            summer.first_column = 0;
            summer.last_column = 0;
        }
    }

    (columns, summers, column_sessions)
}

// One group per straddle: boxes aimed at a pair already occupied join the
// existing group instead of stacking a second one on the same grid cells.
fn push_summer_boxes(
    summers: &mut Vec<SummerGroup>,
    label: Option<String>,
    credits: Option<String>,
    first_column: usize,
    last_column: usize,
    boxes: Vec<CourseBox>,
) {
    if let Some(group) = summers.iter_mut().find(|group| {
        group.first_column == first_column && group.last_column == last_column
    }) {
        group.boxes.extend(boxes);
        return;
    }
    summers.push(SummerGroup {
        label,
        credits,
        first_column,
        last_column,
        boxes,
    });
}

// --- « cours option » boxes, stats, tableau des règles --------------------

// Past this many courses in a rule's own list, the box stops enumerating
// and points at the app / program page instead — a print-scale readability
// cap, not a data limit (the app still lists everything).
const MAX_LISTED_OPTION_COURSES: usize = 10;

// The « Cours option » boxes: one per session that still has room under
// the credit cap, listing — rule by rule — the remaining candidates that
// could actually be taken there: enough room for the course's own credits,
// and its prerequisites satisfiable by what the plan places earlier
// (retour d'Antoine, 2026-08-26 — the reader sees, per session, what may
// go where). A keyword/reference/raw rule (« tout cours de 1er cycle »)
// has no list to filter and states its sentence once, in the last session
// with room. A rule offered nowhere is named in a note, never silently
// absent. `sessions` maps each column back to its 1-based session (étés
// left the grid, so a column's index no longer is its session); `horizon`
// is the whole horizon's session count, étés included.
#[allow(clippy::too_many_arguments)]
fn place_option_boxes(
    columns: &mut [Column],
    sessions: &[usize],
    horizon: usize,
    snapshot: &Snapshot,
    plan: &Plan,
    report: &CoverageReport,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    notes: &mut Vec<String>,
) {
    if columns.is_empty() {
        return;
    }

    // the placement ground truth the feasibility check reads: each placed
    // course's session, the credits accumulated before each session, and
    // each column's remaining room under the cap
    let mut placed_session: BTreeMap<String, usize> = BTreeMap::new();
    let mut credits_before: Vec<u32> = vec![0; horizon + 1];
    let mut accumulated = 0u32;
    for session in 1..=horizon {
        if let Some(cell) = credits_before.get_mut(session) {
            *cell = accumulated;
        }
        accumulated +=
            crate::solve::session_credits(snapshot, plan, session).total;
        for code in state::session_codes(plan, session) {
            placed_session.entry(code).or_insert(session);
        }
    }
    let room: Vec<i64> = sessions
        .iter()
        .map(|&session| {
            let used =
                crate::solve::session_credits(snapshot, plan, session).total;
            i64::from(plan.credit_cap) - i64::from(used)
        })
        .collect();
    let last_roomy = room.iter().rposition(|&space| space > 0);

    // the rules that still offer a choice — stages have their own
    // representation (the été cards), and the préparatoire and « Hors
    // programme » never appear (retours d'Antoine, 2026-08-26)
    let relevant: Vec<&RuleReport> = report
        .rules
        .iter()
        .filter(|rule_report| {
            rule_report.status != RuleStatus::Satisfied
                && !(rule_report.status == RuleStatus::Reported
                    && rule_report
                        .counted
                        .as_ref()
                        .is_some_and(|counted| !counted.is_empty()))
                && rule_report.title != PREPARATORY_RULE_TITLE
                && rule_report.title != OUT_OF_PROGRAM_RULE_TITLE
                && rule_report.title != STAGES_RULE_TITLE
        })
        .collect();

    let mut offered: BTreeSet<&str> = BTreeSet::new();
    for (index, column) in columns.iter_mut().enumerate() {
        let space = room.get(index).copied().unwrap_or(0);
        if space <= 0 {
            continue;
        }
        let session = sessions.get(index).copied().unwrap_or(0);
        let mut rules_here: Vec<OptionRule> = Vec::new();
        for rule_report in &relevant {
            let Some(rule) = find_rule(
                program,
                concentration,
                profile,
                rule_report.scope,
                &rule_report.title,
            ) else {
                // defensive: `coverage_report` reports exactly the rules
                // the same `program` carries, so this cannot happen — but
                // the document degrades by skipping the rule rather than
                // panicking
                continue;
            };
            let constraint = match rule.constraint.as_ref() {
                Some(constraint) => constraint_text(constraint),
                None => "(contrainte non chiffrée)".to_string(),
            };
            match &rule.courses {
                // a long list printed whole is unreadable at print scale:
                // past the cap the rule points at the app and the program
                // page instead (retour d'Antoine, 2026-08-26)
                RuleCourses::List { courses }
                    if courses.len() > MAX_LISTED_OPTION_COURSES =>
                {
                    if Some(index) != last_roomy {
                        continue;
                    }
                    offered.insert(&rule_report.title);
                    rules_here.push(OptionRule {
                        title: rule_report.title.clone(),
                        constraint,
                        choices: Vec::new(),
                        raw: Some(
                            "voir les cours disponibles dans l'application \
                             ou la page web du programme"
                                .to_string(),
                        ),
                    });
                }
                RuleCourses::List { .. } => {
                    let choices: Vec<String> = rule_report
                        .candidates
                        .iter()
                        .flatten()
                        .filter(|code| {
                            candidate_fits(
                                snapshot,
                                code,
                                session,
                                space,
                                &placed_session,
                                &credits_before,
                            )
                        })
                        .cloned()
                        .collect();
                    if choices.is_empty() {
                        continue;
                    }
                    offered.insert(&rule_report.title);
                    rules_here.push(OptionRule {
                        title: rule_report.title.clone(),
                        constraint,
                        choices,
                        raw: None,
                    });
                }
                RuleCourses::Reference { raw, .. }
                | RuleCourses::Keyword { raw, .. }
                | RuleCourses::Raw { raw } => {
                    // no list to filter by session — the sentence appears
                    // once, under the last session with room
                    if Some(index) != last_roomy {
                        continue;
                    }
                    offered.insert(&rule_report.title);
                    rules_here.push(OptionRule {
                        title: rule_report.title.clone(),
                        constraint,
                        choices: Vec::new(),
                        raw: Some(raw.clone()),
                    });
                }
            }
        }
        if !rules_here.is_empty() {
            column.options.push(OptionBox {
                heading: "Cours option".to_string(),
                rules: rules_here,
            });
        }
    }

    for rule_report in &relevant {
        if !offered.contains(rule_report.title.as_str()) {
            notes.push(format!(
                "Aucune session n'a de place pour un choix de « {} ».",
                rule_report.title
            ));
        }
    }
}

// Whether `session` could host `code`, judged on the drawn placement
// alone: the course's credits must fit the session's remaining room, and
// its prerequisite tree must be satisfiable by what the plan places
// earlier. Every unknown lets the course through — a code the catalogue
// lacks, a raw leaf, a prerequisite the plan never places (it could be
// taken along the way): the printed document only rules out what the
// placement provably forbids, never more.
fn candidate_fits(
    snapshot: &Snapshot,
    code: &str,
    session: usize,
    room: i64,
    placed_session: &BTreeMap<String, usize>,
    credits_before: &[u32],
) -> bool {
    let Some(&index) = snapshot.by_code.get(code) else {
        return true;
    };
    let course = &snapshot.courses[index];
    if i64::from(course.credits.planning()) > room {
        return false;
    }
    match course.prerequisites.as_ref() {
        None | Some(Prerequisites::Raw { .. }) => true,
        Some(Prerequisites::Parsed { tree, .. }) => {
            tree_allows(tree, session, placed_session, credits_before)
        }
    }
}

// The boolean the prerequisite tree encodes, evaluated against the drawn
// placement — unlike the token pass, which draws every operand, this one
// really computes `all`/`any`. Explicit stacks, bounded by
// `MAX_TREE_NODES` (no recursion): the tree is linearized pre-order, then
// folded child-verdicts-first by walking the line backward.
fn tree_allows(
    tree: &PrereqTree,
    session: usize,
    placed_session: &BTreeMap<String, usize>,
    credits_before: &[u32],
) -> bool {
    let mut visit: Vec<&PrereqTree> = vec![tree];
    let mut order: Vec<&PrereqTree> = Vec::new();
    for _ in 0..MAX_TREE_NODES {
        let Some(node) = visit.pop() else {
            break;
        };
        order.push(node);
        match node {
            PrereqTree::All { all } => visit.extend(all.iter()),
            PrereqTree::Any { any } => visit.extend(any.iter()),
            _ => {}
        }
    }
    let mut verdicts: Vec<bool> = Vec::with_capacity(order.len());
    for node in order.iter().rev() {
        let verdict = match node {
            PrereqTree::Course(code) => placed_session
                .get(code)
                .map(|&placed| placed < session)
                .unwrap_or(true),
            PrereqTree::Concomitant { concomitant } => placed_session
                .get(concomitant)
                .map(|&placed| placed <= session)
                .unwrap_or(true),
            PrereqTree::ProgramCredits { program_credits } => {
                credits_before.get(session).copied().unwrap_or(0)
                    >= program_credits.credits
            }
            PrereqTree::Raw { .. } => true,
            PrereqTree::All { all } => {
                fold_children(&mut verdicts, all.len(), true)
            }
            PrereqTree::Any { any } => {
                fold_children(&mut verdicts, any.len(), false)
            }
        };
        verdicts.push(verdict);
    }
    verdicts.pop().unwrap_or(true)
}

// Pops the last `count` child verdicts and combines them — AND when
// `conjunction`, OR otherwise. Zero children never blocks (a degenerate
// node hides nothing).
fn fold_children(
    verdicts: &mut Vec<bool>,
    count: usize,
    conjunction: bool,
) -> bool {
    if count == 0 {
        return true;
    }
    let start = verdicts.len().saturating_sub(count);
    let children = verdicts.drain(start..);
    if conjunction {
        children.into_iter().all(|verdict| verdict)
    } else {
        children.into_iter().any(|verdict| verdict)
    }
}

// The one place a `RuleReport`'s scope is turned back into the `Rule` it
// came from — needed because the report itself drops the `Constraint` and
// `RuleCourses` once it has evaluated them.
fn find_rule<'a>(
    program: &'a Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    scope: Scope,
    title: &str,
) -> Option<&'a Rule> {
    let rules: &[Rule] = match scope {
        Scope::Program => &program.rules,
        Scope::Concentration => &program.concentration(concentration?)?.rules,
        Scope::Profile => &program.profile(profile?)?.rules,
    };
    rules.iter().find(|rule| rule.title == title)
}

// « 1 cours », « 3 à 9 crédits » — the counted unit alone: the table and
// the option boxes both show the rule's title separately.
fn constraint_text(constraint: &Constraint) -> String {
    match *constraint {
        Constraint::Course { min, max } if min == max => {
            format!("{min} cours")
        }
        Constraint::Course { min, max } => format!("{min} à {max} cours"),
        Constraint::Credits { min, max } if min == max => {
            format!("{min} crédits")
        }
        Constraint::Credits { min, max } => {
            format!("{min} à {max} crédits")
        }
    }
}

// The document's small rules table: every rule of the effective program,
// satisfied or not (unlike the option boxes, which skip satisfied rules).
fn build_rules_table(
    report: &CoverageReport,
    snapshot: &Snapshot,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> Vec<RulesRow> {
    report
        .rules
        .iter()
        // the préparatoire is admission remediation and « Hors programme »
        // is bookkeeping, not real program rules — both kept off the
        // printed table, complete plan included (retours d'Antoine,
        // 2026-08-26)
        .filter(|rule_report| {
            rule_report.title != PREPARATORY_RULE_TITLE
                && rule_report.title != OUT_OF_PROGRAM_RULE_TITLE
        })
        .map(|rule_report| {
            let constraint = find_rule(
                program,
                concentration,
                profile,
                rule_report.scope,
                &rule_report.title,
            )
            .and_then(|rule| rule.constraint.as_ref());
            let counted: Vec<&str> = rule_report
                .counted
                .iter()
                .flatten()
                .map(String::as_str)
                .collect();
            let resolved = rule_report.status != RuleStatus::Incomplete;
            let credits = if counted.is_empty() {
                "—".to_string()
            } else {
                counted
                    .iter()
                    .map(|code| course_credits(snapshot, code))
                    .sum::<u32>()
                    .to_string()
            };
            RulesRow {
                rule: rule_report.title.clone(),
                constraint: constraint
                    .map(constraint_text)
                    .unwrap_or_else(|| "—".to_string()),
                chosen: chosen_text(rule_report, &counted, resolved),
                resolved,
                credits,
            }
        })
        .collect()
}

// The « Cours retenus » cell: the counted codes when the rule has any; an
// `Incomplete` rule points at where the remaining choices live — the grey
// boxes, or the stage candidates themselves, since stages have no grey box
// (their bands live between the columns instead).
fn chosen_text(
    rule_report: &RuleReport,
    counted: &[&str],
    resolved: bool,
) -> String {
    let listed = counted.join(", ");
    if resolved {
        if listed.is_empty() {
            return "—".to_string();
        }
        return listed;
    }
    let source = if rule_report.title == STAGES_RULE_TITLE {
        match rule_report
            .candidates
            .as_ref()
            .filter(|candidates| !candidates.is_empty())
        {
            Some(candidates) => candidates.join(", "),
            None => "voir cases grises".to_string(),
        }
    } else {
        "voir cases grises".to_string()
    };
    if listed.is_empty() {
        format!("à choisir — {source}")
    } else {
        format!("{listed} — à compléter, {source}")
    }
}

// Every course a numbered rule (« Règle N » → « R{N} ») or the Hors
// programme rule counted earns its chip; the Stages rule and any other
// title tag nothing — a stage box is a plain course, the design's own
// rule. First rule in report order wins a doubly-counted course.
fn apply_tags(
    columns: &mut [Column],
    summers: &mut [SummerGroup],
    report: &CoverageReport,
) {
    let mut tags: BTreeMap<&str, (String, bool)> = BTreeMap::new();
    for rule_report in &report.rules {
        let Some(tag) = rule_tag(&rule_report.title) else {
            continue;
        };
        let out_of_program = rule_report.title == OUT_OF_PROGRAM_RULE_TITLE;
        for code in rule_report.counted.iter().flatten() {
            tags.entry(code.as_str())
                .or_insert((tag.clone(), out_of_program));
        }
    }
    let boxes = columns
        .iter_mut()
        .flat_map(|column| column.boxes.iter_mut())
        .chain(
            summers
                .iter_mut()
                .flat_map(|summer| summer.boxes.iter_mut()),
        );
    for course_box in boxes {
        if let Some((tag, out_of_program)) = tags.get(course_box.code.as_str())
        {
            course_box.tag = Some(tag.clone());
            // a hors-programme course is an optional addition — dashed,
            // like an unchosen ordinal stage slot
            course_box.optional |= *out_of_program;
        }
    }
}

// « Règle 3 » → « R3 », « Hors programme » → « HP »; every other title —
// « Stages », « Scolarité préparatoire », a concentration's unnumbered
// rule — tags nothing.
fn rule_tag(title: &str) -> Option<String> {
    if title == OUT_OF_PROGRAM_RULE_TITLE {
        return Some("HP".to_string());
    }
    rule_number(title).map(|number| format!("R{number}"))
}

fn rule_number(title: &str) -> Option<u32> {
    let digits: String = title
        .strip_prefix("Règle ")?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

// The header strip: credits against the program's target, then the
// personal cells (stages, hors programme, reconnus) — a zero cell is
// omitted, an empty strip never shown as a row of zeros. Without a program
// and its coverage there is no target to count against, so the strip
// degrades to the placed total alone (ERR-5, same spirit as the rules).
fn build_stats(
    snapshot: &Snapshot,
    plan: &Plan,
    program: Option<&Program>,
    report: Option<&CoverageReport>,
    placed_codes: &[String],
    horizon: usize,
    ordinal: bool,
) -> Vec<Stat> {
    let placed_credits: u32 = (1..=horizon)
        .map(|session| {
            crate::solve::session_credits(snapshot, plan, session).total
        })
        .sum();
    let (Some(program), Some(report)) = (program, report) else {
        return vec![Stat {
            value: placed_credits.to_string(),
            label: "crédits placés".to_string(),
        }];
    };

    let placed: BTreeSet<&str> =
        placed_codes.iter().map(String::as_str).collect();
    let stage_placed: Vec<&str> = counted_codes(report, STAGES_RULE_TITLE)
        .into_iter()
        .filter(|code| placed.contains(code))
        .collect();
    let stage_credits: u32 = stage_placed
        .iter()
        .map(|code| course_credits(snapshot, code))
        .sum();
    let out_credits: u32 = counted_codes(report, OUT_OF_PROGRAM_RULE_TITLE)
        .into_iter()
        .filter(|code| placed.contains(code))
        .map(|code| course_credits(snapshot, code))
        .sum();
    let credited_credits: u32 = plan
        .credited
        .iter()
        .map(|code| course_credits(snapshot, code))
        .sum();

    // stages are « en sus » (their credits sit outside the program total)
    // and hors programme courses count toward nothing; the credited ones
    // count without occupying a session
    let program_total = (i64::from(placed_credits)
        - i64::from(stage_credits)
        - i64::from(out_credits)
        + i64::from(credited_credits))
    .max(0);
    let required = program.credits_required.max(0);
    let mut stats = vec![Stat {
        value: format!("{program_total} / {required}"),
        label: "crédits choisis".to_string(),
    }];

    if ordinal {
        let remaining = (required - program_total).max(0);
        if remaining > 0 {
            stats.push(Stat {
                value: format!("{remaining} cr"),
                label: rules_range_label(report),
            });
        }
        return stats;
    }

    if !stage_placed.is_empty() {
        stats.push(Stat {
            value: stage_placed.len().to_string(),
            label: "stages".to_string(),
        });
    }
    if out_credits > 0 {
        stats.push(Stat {
            value: format!("{out_credits} cr"),
            label: "hors programme".to_string(),
        });
    }
    if credited_credits > 0 {
        stats.push(Stat {
            value: format!("{credited_credits} cr"),
            label: "reconnus".to_string(),
        });
    }
    stats
}

// « à choisir, règles 1 à 6 » — the numbered rules the remaining credits
// belong to; a program with no « Règle N » titles at all keeps the bare
// « à choisir » rather than inventing a range.
fn rules_range_label(report: &CoverageReport) -> String {
    let highest = report
        .rules
        .iter()
        .filter_map(|rule| rule_number(&rule.title))
        .max();
    match highest {
        Some(highest) if highest > 1 => {
            format!("à choisir, règles 1 à {highest}")
        }
        Some(_) => "à choisir, règle 1".to_string(),
        None => "à choisir".to_string(),
    }
}

fn counted_codes<'a>(report: &'a CoverageReport, title: &str) -> Vec<&'a str> {
    report
        .rules
        .iter()
        .find(|rule| rule.title == title)
        .and_then(|rule| rule.counted.as_ref())
        .map(|codes| codes.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

// 0 for a code the catalogue does not know: the absence is surfaced where
// the code is displayed (a box note, the reconnaissance sidebox), so the
// sums stay sums instead of failing over one missing course.
fn course_credits(snapshot: &Snapshot, code: &str) -> u32 {
    snapshot
        .by_code
        .get(code)
        .map(|&index| snapshot.courses[index].credits.planning())
        .unwrap_or(0)
}

// --- cours de stage du cheminement type (mode ordinal) --------------------

// Each stage course of the « Stages » rule the plan does not place is
// drawn as its own course box at the official document's fixed position:
// the été after the kth hiver (retour d'Antoine, 2026-08-26 — the
// « Microprogramme de stage » bands are gone, the course itself is
// clearer). The first stage is the expected path; the later ones are only
// possibilities and are marked `optional` (dashed border). A stage whose
// straddle would fall past the drawn horizon is not drawn — the rules
// table still names every stage. Slots are appended after every real
// session so the token pass sees the boxes in document order and
// `split_slots` already knows every real column when it lands them.
#[allow(clippy::too_many_arguments)]
fn append_stage_slots(
    program: &Program,
    snapshot: &Snapshot,
    plan: &Plan,
    semesters: &[Semester],
    slots: &mut Vec<Slot>,
    operands: &mut Vec<Vec<Vec<Operand>>>,
    box_position: &mut BTreeMap<String, (usize, usize)>,
    notes: &mut Vec<String>,
) {
    let Some(stage_rule) = program
        .rules
        .iter()
        .find(|rule| rule.title == STAGES_RULE_TITLE)
    else {
        return;
    };
    let RuleCourses::List { courses } = &stage_rule.courses else {
        return;
    };

    // the anchors an été can occupy (the column before each été), read
    // straight off the horizon — `split_slots` derives the very same
    // numbering; only anchors whose straddle fits the drawn grid count.
    // Alongside them: where each placed stage sits, as the bound its
    // followers must respect.
    let stage_set: BTreeSet<&str> =
        courses.iter().map(String::as_str).collect();
    let mut anchors: Vec<usize> = Vec::new();
    let mut used: BTreeSet<usize> = BTreeSet::new();
    // per placed stage: the minimum anchor VALUE a later stage may take —
    // one past a stage sitting in an été, the é right after a stage
    // sitting in a regular session
    let mut placed_bound: BTreeMap<&str, usize> = BTreeMap::new();
    let mut column = 0usize;
    for (index, semester) in semesters.iter().enumerate() {
        let session = index + 1;
        if semester.season == Season::Summer {
            let anchor = column.saturating_sub(1);
            for code in state::session_codes(plan, session) {
                if let Some(&code_ref) = stage_set.get(code.as_str()) {
                    // one stage per été, structurally: this é is spent
                    used.insert(anchor);
                    placed_bound.entry(code_ref).or_insert(anchor + 1);
                }
            }
            continue;
        }
        for code in state::session_codes(plan, session) {
            if let Some(&code_ref) = stage_set.get(code.as_str()) {
                placed_bound.entry(code_ref).or_insert(column);
            }
        }
        if semester.season == Season::Winter {
            anchors.push(column);
        }
        column += 1;
    }
    let column_count = column;
    anchors.retain(|&anchor| anchor + 1 < column_count);

    // chronology holds by construction: a synthetic stage takes the first
    // free anchor AT OR PAST the bound left by every earlier stage of the
    // rule — stage II never lands in an été before a placed stage I
    // (retour d'Antoine, 2026-08-26)
    let mut minimum = 0usize;
    for (index, code) in courses.iter().enumerate() {
        let number = index + 1;
        if box_position.contains_key(code) {
            // the plan already places this stage — it is drawn there, and
            // its followers go after it
            if let Some(&bound) = placed_bound.get(code.as_str()) {
                minimum = minimum.max(bound);
            }
            continue;
        }
        let Some(&first_column) = anchors
            .iter()
            .find(|anchor| !used.contains(anchor) && **anchor >= minimum)
        else {
            // no free été left before the horizon's edge — the rules
            // table still names every stage
            continue;
        };
        used.insert(first_column);
        minimum = first_column + 1;
        let course: Option<&Course> = snapshot
            .by_code
            .get(code)
            .map(|&index| &snapshot.courses[index]);
        let (title, credits) = match course {
            Some(course) => {
                (course.title.clone(), credits_label(&course.credits))
            }
            None => {
                notes.push(format!(
                    "{code} : absent du catalogue de cours — affiché \
                     avec son code comme titre, sans crédits."
                ));
                (code.clone(), String::new())
            }
        };
        box_position.insert(code.clone(), (slots.len(), 0));
        operands.push(vec![operands_of(
            course.and_then(|course| course.prerequisites.as_ref()),
        )]);
        slots.push(Slot {
            semester: Semester {
                season: Season::Summer,
                year: 0,
            },
            session: 0,
            summer: true,
            stage_anchor: Some(first_column),
            boxes: vec![CourseBox {
                code: code.clone(),
                title,
                credits,
                // assigned with every other box after all stages are appended
                hue: 0.0,
                tag: None,
                optional: number > 1,
                entry: Vec::new(),
                exit: None,
            }],
        });
    }
}

// --- exigences, sous-titre, méta, légende ---------------------------------

// The « Exigences » sideboxes: the credited courses (reconnaissance des
// acquis — counted, never given a session) and the program's language
// requirement. Both absent means no box at all, never an empty frame.
fn build_requirements(
    snapshot: &Snapshot,
    plan: &Plan,
    program: Option<&Program>,
) -> Vec<SideBox> {
    let mut boxes = Vec::new();
    if !plan.credited.is_empty() {
        let mut lines: Vec<SideBoxLine> = plan
            .credited
            .iter()
            .map(|code| {
                let text = snapshot
                    .by_code
                    .get(code)
                    .map(|&index| {
                        let course = &snapshot.courses[index];
                        format!(
                            "{} — {}",
                            course.title,
                            credits_label(&course.credits)
                        )
                    })
                    .unwrap_or_else(|| {
                        "absent du catalogue de cours".to_string()
                    });
                SideBoxLine {
                    lead: Some(code.clone()),
                    text,
                }
            })
            .collect();
        lines.push(SideBoxLine {
            lead: None,
            text: "Reconnaissance des acquis : crédité, hors session."
                .to_string(),
        });
        boxes.push(SideBox { lines });
    }
    if let Some(requirement) =
        program.and_then(|program| program.language_requirement.as_ref())
    {
        boxes.push(SideBox {
            lines: vec![SideBoxLine {
                lead: Some("Exigence linguistique.".to_string()),
                text: language_text(requirement),
            }],
        });
    }
    boxes
}

fn language_text(requirement: &LanguageRequirement) -> String {
    let francophone = qualification_text(&requirement.francophone);
    match &requirement.non_francophone {
        Some(branch) => format!(
            "Réussir {francophone} pour diplômer; personne non \
             francophone : {}.",
            qualification_text(branch)
        ),
        None => format!("Réussir {francophone} pour diplômer."),
    }
}

// « ANL-2020 (ou VEPT ≥ 53) » — the course, then the placement tests that
// dispense from it, ANDed by the grammar so joined with « et ».
fn qualification_text(qualification: &LanguageQualification) -> String {
    if qualification.tests.is_empty() {
        return qualification.course.clone();
    }
    let tests: Vec<String> = qualification
        .tests
        .iter()
        .map(|test| format!("{} ≥ {}", test.name, test.score))
        .collect();
    format!("{} (ou {})", qualification.course, tests.join(" et "))
}

// « B-GEX, version A24 » — the program identity alone; only the ordinal
// document adds what it is (cheminement type), a personal plan carries no
// descriptive tail (retour d'Antoine, 2026-08-26).
fn build_subtitle(plan: &Plan, ordinal: bool) -> String {
    let identity = plan
        .program
        .as_ref()
        .map(|choice| format!("{}, version {}", choice.code, choice.semester))
        .unwrap_or_else(|| "programme non choisi".to_string());
    if ordinal {
        return format!("{identity} — cheminement type");
    }
    identity
}

fn build_meta(
    provenance: &ExportProvenance,
    generated_at: &str,
    share_url: &str,
) -> MetaBlock {
    MetaBlock {
        generated: generated_line(generated_at),
        data: format!("Données du répertoire : {}", provenance.scraped),
        build: format!(
            "app v{} — code {} — données {}",
            provenance.version, provenance.build, provenance.data
        ),
        repo_label: provenance
            .repo
            .strip_prefix("https://")
            .unwrap_or(&provenance.repo)
            .to_string(),
        repo_url: provenance.repo.clone(),
        share_url: share_url.to_string(),
        share_label: "Accéder à cet organigramme dans l'application"
            .to_string(),
    }
}

// « Généré le 2026-08-26 à 15:22 » — the boundary's local stamp, no zone
// (retour d'Antoine, 2026-08-26); a `Z`-suffixed stamp is still spelled
// out as UTC, and any other shape degrades to the raw string rather than
// being dropped or guessed at — the same rules as
// `provenance::format_generated`, capitalized for the meta block's lead.
fn generated_line(generated_at: &str) -> String {
    let (stamp, zone) = match generated_at.strip_suffix('Z') {
        Some(stamp) => (stamp, " UTC"),
        None => (generated_at, ""),
    };
    if let Some((date, time)) = stamp.split_once('T') {
        let mut parts = time.split(':');
        if let (Some(hour), Some(minute), Some(_second), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            if !date.is_empty() && !hour.is_empty() && !minute.is_empty() {
                return format!("Généré le {date} à {hour}:{minute}{zone}");
            }
        }
    }
    format!("Généré le {generated_at}")
}

// --- préalables -----------------------------------------------------------

// A whole-course `Prerequisites::Raw` (the source text fell entirely
// outside the grammar) is folded into the same single-operand shape as a
// nested `PrereqTree::Raw` — one code path carries both.
fn operands_of(prerequisites: Option<&Prerequisites>) -> Vec<Operand> {
    match prerequisites {
        None => Vec::new(),
        Some(Prerequisites::Raw { .. }) => vec![Operand::Raw],
        Some(Prerequisites::Parsed { tree, .. }) => collect_operands(tree),
    }
}

// Explicit stack, bounded by `MAX_TREE_NODES` — no recursion. `All`/`Any`
// operands are collected alike: the document draws every operand a course
// lists, never the boolean the tree evaluates to. Children are pushed
// reversed so popping yields them left-to-right, the order the source text
// (and the JSON array) itself lists them in.
fn collect_operands(tree: &PrereqTree) -> Vec<Operand> {
    let mut stack: Vec<&PrereqTree> = vec![tree];
    let mut operands = Vec::new();
    for _ in 0..MAX_TREE_NODES {
        let Some(node) = stack.pop() else {
            break;
        };
        match node {
            PrereqTree::Course(code) => {
                operands.push(Operand::Course(code.clone()));
            }
            PrereqTree::Concomitant { concomitant } => {
                operands.push(Operand::Concomitant(concomitant.clone()));
            }
            PrereqTree::Raw { .. } => operands.push(Operand::Raw),
            PrereqTree::ProgramCredits { program_credits } => {
                operands.push(Operand::Credits(program_credits.credits));
            }
            PrereqTree::All { all } => stack.extend(all.iter().rev()),
            PrereqTree::Any { any } => stack.extend(any.iter().rev()),
        }
    }
    operands
}

// `a…z`, then `aa`, `ab`, … — bijective base-26, 0-indexed (`token_letter(0)
// == "a"`), so a 27th distinct source rolls over instead of running out of
// single letters.
fn token_letter(index: usize) -> String {
    let mut n = index + 1;
    let mut letters = Vec::new();
    for _ in 0..MAX_LETTER_DIGITS {
        if n == 0 {
            break;
        }
        let remainder = (n - 1) % 26;
        letters.push((b'a' + u8::try_from(remainder).unwrap_or(0)) as char);
        n = (n - 1) / 26;
    }
    letters.iter().rev().collect()
}

// Transcribed from `gex_organigramme.pdf`'s « Note 1 » plus the design's
// own chips and dashed border — typed entries, so the dumb view draws the
// right swatch beside each sentence. Always the full six: the legend
// documents the grammar, whether or not a given document uses every token.
fn build_legend() -> Vec<LegendEntry> {
    vec![
        LegendEntry::Letter {
            text: "préalable : jeton de sortie à droite du cours source, \
                   jeton d'entrée à gauche du cours qui l'exige"
                .to_string(),
        },
        LegendEntry::Shaded {
            text: "concomitance permise : le préalable peut être suivi à \
                   la même session"
                .to_string(),
        },
        LegendEntry::Credits {
            text: "préalable de crédits : n crédits du programme réussis"
                .to_string(),
        },
        LegendEntry::Chip {
            chip: "R3".to_string(),
            text: "cours option choisi au titre de la règle correspondante \
                   (voir tableau des règles)"
                .to_string(),
        },
        LegendEntry::Chip {
            chip: "HP".to_string(),
            text: "cours hors programme".to_string(),
        },
        LegendEntry::Optional {
            text: "cours optionnel (stage additionnel, ajout hors programme)"
                .to_string(),
        },
    ]
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use crate::data::{parse_data, RawData};

    fn snapshot_with(courses_json: &str) -> Snapshot {
        parse_data(
            &RawData {
                courses: courses_json.to_string(),
                meta: Some(r#"{"scraped_at":"2026-08-01"}"#.to_string()),
                manual: None,
                programs: Vec::new(),
            },
            Vec::new(),
            Vec::new(),
        )
        .unwrap_or_else(|error| panic!("{error}"))
    }

    fn plan_with(
        study_sessions: usize,
        assignments: &[(usize, &[&str])],
    ) -> Plan {
        let mut plan = Plan {
            study_sessions,
            ..Plan::default()
        };
        for &(session, codes) in assignments {
            plan.manual.insert(
                session,
                codes.iter().map(|code| code.to_string()).collect(),
            );
        }
        plan
    }

    fn build(
        snapshot: &Snapshot,
        plan: &Plan,
        program: Option<&Program>,
    ) -> OrganigrammeDocument {
        organigramme_document(
            snapshot,
            plan,
            program,
            "2026-08-25T14:00:00",
            "app#plan",
        )
    }

    // GEX-1000: no prerequisites, becomes source « a ».
    // GEX-2000: requires GEX-1000 in concomitance (the répertoire's `*`,
    // a `{"concomitant": …}` leaf) — shaded, and placeable same session.
    // GEX-3000 (a later session): requires GEX-1000 strictly — not shaded,
    // same letter, so GEX-1000 carries one exit token for two dependents.
    // GEX-4000: requires 24 program credits — a numeric token.
    // GEX-5000: requires a `PrereqTree::Raw` operand — a note, no token.
    // GEX-6000: requires GEX-7000, a real course nowhere in the plan — a
    // note naming the off-document prerequisite.
    // ZZZ-9999: not in the catalogue at all — falls back to its own code.
    const COURSES: &str = r#"{"courses":[
      {"code":"GEX-1000","title":"Fondations","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"GEX-2000","title":"Suite concomitante","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000*",
         "tree":{"concomitant":"GEX-1000"}},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-3000","title":"Suite plus tard","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000","tree":"GEX-1000"},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-4000","title":"Seuil de crédits","credits":3,"cycle":1,
       "prerequisites":{"raw":"Crédits exigés : 24",
         "tree":{"program_credits":{"program":null,"credits":24}}},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-5000","title":"Examen","credits":3,"cycle":1,
       "prerequisites":{"raw":"Examen quelconque",
         "tree":{"raw":"Examen quelconque"}},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-6000","title":"Hors document","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-7000","tree":"GEX-7000"},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-7000","title":"Jamais placé","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"GEX-8000","title":"Texte non parsable","credits":3,"cycle":1,
       "prerequisites":{"raw":"Autorisation de la direction"},
       "equivalents":[],"seasons":{}},
      {"code":"GEX-9000","title":"Un des deux","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000 OU GEX-7000",
         "tree":{"any":["GEX-1000","GEX-7000"]}},
       "equivalents":[],"seasons":{}}
    ]}"#;

    // sessions 1 (A26) and 3 (É27): the summer boxes exercise the token
    // pass across the column/summer split
    fn document() -> OrganigrammeDocument {
        let snapshot = snapshot_with(COURSES);
        let plan = plan_with(
            2,
            &[
                (1, &["GEX-1000", "GEX-2000"]),
                (
                    3,
                    &[
                        "GEX-3000", "GEX-4000", "GEX-5000", "GEX-6000",
                        "ZZZ-9999", "GEX-8000", "GEX-9000",
                    ],
                ),
            ],
        );
        build(&snapshot, &plan, None)
    }

    fn find<'a>(
        document: &'a OrganigrammeDocument,
        code: &str,
    ) -> &'a CourseBox {
        document
            .columns
            .iter()
            .flat_map(|column| &column.boxes)
            .chain(document.summers.iter().flat_map(|summer| &summer.boxes))
            .find(|course_box| course_box.code == code)
            .unwrap_or_else(|| panic!("{code} not in the document"))
    }

    // --- jetons -----------------------------------------------------------

    #[test]
    fn the_source_carries_exactly_one_exit_token_for_two_dependents() {
        let document = document();
        let source = find(&document, "GEX-1000");
        let Some(Token::Letter { letter, shaded }) = &source.exit else {
            panic!(
                "GEX-1000 should have earned an exit token: {:?}",
                source.exit
            );
        };
        assert_eq!(letter, "a", "the first letter needed, in document order");
        assert!(!shaded, "an exit token is never itself shaded");

        let same_session = find(&document, "GEX-2000");
        assert_eq!(
            same_session.entry,
            vec![Token::Letter {
                letter: "a".to_string(),
                shaded: true
            }],
            "concomitant prerequisite: the shaded reading"
        );

        let later_session = find(&document, "GEX-3000");
        assert_eq!(
            later_session.entry,
            vec![Token::Letter {
                letter: "a".to_string(),
                shaded: false
            }],
            "later-session dependent: the plain, strictly-before reading"
        );
    }

    #[test]
    fn a_program_credits_operand_is_a_numeric_entry_token() {
        let document = document();
        let course_box = find(&document, "GEX-4000");
        assert_eq!(course_box.entry, vec![Token::Credits { credits: 24 }]);
        assert!(
            course_box.exit.is_none(),
            "a numeric threshold names no course"
        );
    }

    #[test]
    fn undrawable_operands_draw_nothing_and_say_nothing() {
        // raw prerequisites (GEX-5000, GEX-8000) and off-document sources
        // (GEX-6000 → GEX-7000) are the app's business, not the printed
        // document's — no token, no note (retour d'Antoine, 2026-08-26);
        // the only note left in this document is ZZZ-9999's missing-course
        // one
        let document = document();
        assert!(find(&document, "GEX-5000").entry.is_empty());
        assert!(find(&document, "GEX-8000").entry.is_empty());
        assert!(find(&document, "GEX-6000").entry.is_empty());
        assert_eq!(document.notes.len(), 1, "{:?}", document.notes);
        assert!(document.notes[0].contains("ZZZ-9999"));
    }

    #[test]
    fn an_any_operand_is_drawn_like_an_all_operand() {
        let document = document();
        let course_box = find(&document, "GEX-9000");
        // GEX-1000 yields a token; GEX-7000 (off document) draws nothing
        assert_eq!(
            course_box.entry,
            vec![Token::Letter {
                letter: "a".to_string(),
                shaded: false
            }]
        );
    }

    #[test]
    fn a_missing_course_falls_back_to_its_code_with_a_note() {
        let document = document();
        let course_box = find(&document, "ZZZ-9999");
        assert_eq!(course_box.title, "ZZZ-9999");
        assert_eq!(course_box.credits, "");
        assert!(document.notes.iter().any(|note| note.contains("ZZZ-9999")
            && note.contains("absent du catalogue")));
    }

    #[test]
    fn hues_follow_subject_rank_over_the_complete_organigramme() {
        let document = document();
        // GEX is the first of two matières and keeps one hue across the
        // regular column and the summer group.
        assert_eq!(find(&document, "GEX-1000").hue, 0.0);
        assert_eq!(find(&document, "GEX-2000").hue, 0.0);
        assert_eq!(find(&document, "GEX-3000").hue, 0.0);
        // ZZZ is the second of two matières, half a wheel after GEX.
        assert_eq!(find(&document, "ZZZ-9999").hue, 180.0);
    }

    #[test]
    fn token_letter_covers_the_single_and_double_letter_ranges() {
        assert_eq!(token_letter(0), "a");
        assert_eq!(token_letter(25), "z");
        assert_eq!(token_letter(26), "aa");
        assert_eq!(token_letter(27), "ab");
    }

    #[test]
    fn more_than_26_lettered_sources_roll_over_to_double_letters() {
        // 27 sources each required by one dependent — the 27th letter is
        // « aa »; built programmatically to keep the fixture readable
        let mut courses = String::from(r#"{"courses":["#);
        let mut sources = Vec::new();
        for index in 0..27 {
            let code = format!("SRC-{:04}", 1000 + index);
            courses.push_str(&format!(
                r#"{{"code":"{code}","title":"Source","credits":3,
                   "cycle":1,"prerequisites":null,"equivalents":[],
                   "seasons":{{}}}},"#
            ));
            sources.push(code);
        }
        for (index, source) in sources.iter().enumerate() {
            let comma = if index + 1 == sources.len() { "" } else { "," };
            let code = format!("DEP-{:04}", 1000 + index);
            courses.push_str(&format!(
                r#"{{"code":"{code}","title":"Dépendant","credits":3,
                   "cycle":1,"prerequisites":{{"raw":"{source}",
                   "tree":"{source}"}},"equivalents":[],"seasons":{{}}}}{comma}"#
            ));
        }
        courses.push_str("]}");
        let snapshot = snapshot_with(&courses);
        let source_refs: Vec<&str> =
            sources.iter().map(String::as_str).collect();
        let dependents: Vec<String> = (0..27)
            .map(|index| format!("DEP-{:04}", 1000 + index))
            .collect();
        let dependent_refs: Vec<&str> =
            dependents.iter().map(String::as_str).collect();
        let plan = plan_with(2, &[(1, &source_refs), (2, &dependent_refs)]);
        let document = build(&snapshot, &plan, None);
        let last_source = find(&document, "SRC-1026");
        assert_eq!(
            last_source.exit,
            Some(Token::Letter {
                letter: "aa".to_string(),
                shaded: false
            })
        );
    }

    #[test]
    fn an_empty_plan_yields_empty_columns_without_panicking() {
        let snapshot = snapshot_with(COURSES);
        let plan = plan_with(2, &[]);
        let document = build(&snapshot, &plan, None);
        assert_eq!(document.columns.len(), 2, "A26 and H27");
        assert!(document
            .columns
            .iter()
            .all(|column| column.boxes.is_empty()));
        assert!(document.summers.is_empty(), "an empty été is not drawn");
        assert_eq!(document.stats[0].value, "0");
        assert_eq!(document.stats[0].label, "crédits placés");
    }

    // --- mode ordinal vs personnel ----------------------------------------

    #[test]
    fn a_chosen_program_the_snapshot_lacks_still_names_its_code() {
        // the student chose a program but no `Program` value reached the
        // document (snapshot without it): the title degrades to the code
        let snapshot = snapshot_with(COURSES);
        let plan = Plan {
            program: option_plan().program,
            ..plan_with(2, &[(1, &["GEX-1000"])])
        };
        let document = build(&snapshot, &plan, None);
        assert_eq!(document.program_title, "B-GEX");
        assert!(document.subtitle.starts_with("B-GEX, version A26"));
    }

    #[test]
    fn mandatory_and_required_stage_moves_stay_ordinal() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":["GEX-1000"],
              "rules":[{"title":"Stages",
                "constraint":{"type":"course","min":1,"max":8},
                "courses":["GEX-1580","GEX-2590"]}],
              "concentrations":[{"title":"Hydraulique",
                "mandatory":["GCI-1000"],"rules":[]}],
              "profiles":[{"title":"Recherche",
                "mandatory":["MAT-1000"],"rules":[]}]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let mut plan = Plan {
            program: Some(crate::state::ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: Some("Hydraulique".to_string()),
                profile: Some("Recherche".to_string()),
            }),
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        assert!(
            is_ordinal(&plan, Some(&program)),
            "the automatic placement alone stays ordinal"
        );

        // This mirrors every placement representation, including
        // `place_course` adding a moved mandatory to `electives`.
        plan.electives.push("GEX-1000".to_string());
        plan.pinned_sessions.insert("GCI-1000".to_string(), 2);
        plan.manual.insert(3, vec!["MAT-1000".to_string()]);
        plan.credited.insert("GEX-1000".to_string());
        plan.rule_grants
            .insert("GCI-1000".to_string(), "c/Règle 1".to_string());
        assert!(
            is_ordinal(&plan, Some(&program)),
            "moving mandatory courses across effective scopes stays a type"
        );

        let mut stage_plan = plan.clone();
        stage_plan.electives.push("GEX-1580".to_string());
        stage_plan.pinned_sessions.insert("GEX-1580".to_string(), 3);
        assert!(
            is_ordinal(&stage_plan, Some(&program)),
            "moving the required first stage stays a type"
        );

        stage_plan.electives.push("GEX-2590".to_string());
        assert!(
            !is_ordinal(&stage_plan, Some(&program)),
            "an additional stage is still a personal choice"
        );

        plan.electives.push("CHM-1000".to_string());
        assert!(!is_ordinal(&plan, Some(&program)));

        let mut no_program = Plan::default();
        assert!(is_ordinal(&no_program, None));
        no_program.manual.insert(1, vec!["GEX-1000".to_string()]);
        assert!(!is_ordinal(&no_program, None));
    }

    #[test]
    fn a_required_stage_needs_a_positive_course_constraint_and_a_list() {
        let mut program = stage_program(&["GEX-1580"]);
        assert!(is_required_stage_course(Some(&program), "GEX-1580"));

        let Some(stage_rule) = program.rules.first_mut() else {
            panic!("stage_program should carry its Stages rule");
        };
        stage_rule.constraint = Some(Constraint::Course { min: 0, max: 8 });
        assert!(!is_required_stage_course(Some(&program), "GEX-1580"));

        let Some(stage_rule) = program.rules.first_mut() else {
            panic!("stage_program should still carry its Stages rule");
        };
        stage_rule.constraint = Some(Constraint::Course { min: 1, max: 8 });
        stage_rule.courses = RuleCourses::Raw {
            raw: "Stage convenu avec la direction".to_string(),
        };
        assert!(!is_required_stage_course(Some(&program), "GEX-1580"));
    }

    #[test]
    fn personal_columns_carry_full_years_and_credits() {
        let document = document();
        assert_eq!(document.columns[0].label, "A2026");
        assert_eq!(document.columns[1].label, "H2027");
        assert_eq!(
            document.columns[0].credits.as_deref(),
            Some("6 cr"),
            "two 3-credit boxes"
        );
        assert_eq!(document.summers.len(), 1);
        assert_eq!(document.summers[0].label.as_deref(), Some("É2027"));
        assert_eq!(
            (
                document.summers[0].first_column,
                document.summers[0].last_column
            ),
            (0, 1),
            "the trailing été straddles the last real pair"
        );
    }

    #[test]
    fn an_ordinal_plan_labels_columns_by_rank_without_credits() {
        let snapshot = snapshot_with(COURSES);
        let mut plan = Plan {
            study_sessions: 4,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        let document = build(&snapshot, &plan, None);
        let labels: Vec<&str> = document
            .columns
            .iter()
            .map(|column| column.label.as_str())
            .collect();
        assert_eq!(labels, ["A1", "H2", "A3", "H4"]);
        assert!(document
            .columns
            .iter()
            .all(|column| column.credits.is_none()));
        assert!(
            document.subtitle.ends_with("— cheminement type"),
            "{}",
            document.subtitle
        );
    }

    #[test]
    fn an_ordinal_summer_with_courses_is_still_drawn_defensively() {
        let snapshot = snapshot_with(COURSES);
        let mut plan = Plan {
            study_sessions: 2,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GEX-1000".to_string(), 3);
        let document = build(&snapshot, &plan, None);
        assert_eq!(document.summers.len(), 1);
        assert!(
            document.summers[0].label.is_none(),
            "an ordinal été carries no head"
        );
        assert!(document.summers[0].credits.is_none());
    }

    #[test]
    fn a_lone_column_clamps_the_summer_straddle_to_itself() {
        let snapshot = snapshot_with(COURSES);
        let mut plan = Plan {
            study_sessions: 1,
            ..Plan::default()
        };
        plan.start.season = Season::Winter;
        plan.manual.insert(2, vec!["GEX-1000".to_string()]);
        let document = build(&snapshot, &plan, None);
        assert_eq!(document.columns.len(), 1, "H alone");
        assert_eq!(
            (
                document.summers[0].first_column,
                document.summers[0].last_column
            ),
            (0, 0)
        );
    }

    #[test]
    fn a_personal_subtitle_is_the_program_identity_alone() {
        let document = document();
        assert_eq!(document.subtitle, "programme non choisi");
    }

    // --- option boxes, rules table, chips, stats --------------------------

    // Règle 1 (course, min 2 max 2): 2 slots, none selected.
    // Règle 2 (credits, min 4 max 12): missing 4 credits — ceil(4/3) = 2.
    // Règle 3 (credits, min 3 max 3, "negotiated"): reported, 1 slot.
    // Règle 4 (no constraint, list): reported, 1 slot.
    // Stages (course, min 1 max 2, credits_in_addition): STG-1000 selected
    // satisfies it at the minimum — no slot, no option box.
    const OPTION_PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex",
      "semester":"A26","title":"Génie des eaux","cycle":1,
      "credits_required":90,"mandatory":[],
      "rules":[
        {"title":"Règle 1","constraint":{"type":"course","min":2,"max":2},
         "courses":["OPT-A","OPT-B","OPT-C"]},
        {"title":"Règle 2","constraint":{"type":"credits","min":4,"max":12},
         "courses":["CRD-A","CRD-B"]},
        {"title":"Règle 3","constraint":{"type":"credits","min":3,"max":3},
         "courses":"negotiated","raw":"convenus avec la direction"},
        {"title":"Règle 4","courses":["OPT-D","OPT-E"]},
        {"title":"Stages","constraint":{"type":"course","min":1,"max":2},
         "courses":["STG-1000","STG-2000"],"credits_in_addition":true}
      ],
      "concentrations":[],"profiles":[],
      "language_requirement":{
        "francophone":{"course":"ANL-2020","raw":"ANL-2020"}}
    }"#;

    const OPTION_COURSES: &str = r#"{"courses":[
      {"code":"GEX-1000","title":"Fondations","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"GEX-2000","title":"Suite","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"GEX-3000","title":"Suite 2","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"STG-1000","title":"Stage I","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"ANL-2020","title":"Anglais","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}}
    ]}"#;

    fn option_program() -> Program {
        serde_json::from_str(OPTION_PROGRAM)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    fn option_snapshot() -> Snapshot {
        snapshot_with(OPTION_COURSES)
    }

    // `study_sessions: 2` (A26, H27, É27), credit_cap 6: col0 (GEX-1000,
    // 3 cr) has room for one slot; col1 (GEX-2000 + GEX-3000 + ANL-2020,
    // 9 cr) is over capacity; STG-1000 sits in the été, which is no longer
    // a column — the second slot of « Règle 1 » therefore has nowhere to
    // go and lands in the last column with a note.
    fn option_plan() -> Plan {
        Plan {
            study_sessions: 2,
            credit_cap: 6,
            program: Some(crate::state::ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            manual: BTreeMap::from([
                (1, vec!["GEX-1000".to_string()]),
                (
                    2,
                    vec![
                        "GEX-2000".to_string(),
                        "GEX-3000".to_string(),
                        "ANL-2020".to_string(),
                    ],
                ),
                (3, vec!["STG-1000".to_string()]),
            ]),
            ..Plan::default()
        }
    }

    fn option_document() -> OrganigrammeDocument {
        build(&option_snapshot(), &option_plan(), Some(&option_program()))
    }

    fn find_option_rule<'a>(
        document: &'a OrganigrammeDocument,
        title: &str,
    ) -> &'a OptionRule {
        document
            .columns
            .iter()
            .flat_map(|column| &column.options)
            .flat_map(|option_box| &option_box.rules)
            .find(|rule| rule.title == title)
            .unwrap_or_else(|| panic!("{title} not placed in any column"))
    }

    #[test]
    fn every_roomy_session_offers_the_rules_that_fit_there() {
        // option_plan: cap 6 — col0 has 3 cr of room, col1 none: every
        // rule lands in col0 (unknown candidate codes are never hidden on
        // a guess), the keyword rule states its sentence there too (last
        // roomy column), and col1 shows nothing
        let document = option_document();
        assert_eq!(document.columns.len(), 2, "the été left the grid");
        assert_eq!(document.columns[0].options.len(), 1);
        assert_eq!(document.columns[0].options[0].heading, "Cours option");
        let col0: Vec<&str> = document.columns[0].options[0]
            .rules
            .iter()
            .map(|rule| rule.title.as_str())
            .collect();
        assert_eq!(col0, ["Règle 1", "Règle 2", "Règle 3", "Règle 4"]);
        assert!(document.columns[1].options.is_empty());
        assert!(document
            .notes
            .iter()
            .all(|note| !note.contains("Aucune session")));
    }

    // GEX-1000 (s1) and GEX-2000 (s2) are the placed ground truth; the
    // candidates exercise every feasibility verdict: no prerequisite,
    // strict course, credit threshold, oversized credits, unknown source,
    // concomitance, an `any`, an `all`.
    const FEASIBILITY_COURSES: &str = r#"{"courses":[
      {"code":"GEX-1000","title":"Base 1","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"GEX-2000","title":"Base 2","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"OPT-1000","title":"Libre","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"OPT-2000","title":"Après base","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000","tree":"GEX-1000"},
       "equivalents":[],"seasons":{}},
      {"code":"OPT-3000","title":"Seuil","credits":3,"cycle":1,
       "prerequisites":{"raw":"Crédits : 6",
         "tree":{"program_credits":{"program":null,"credits":6}}},
       "equivalents":[],"seasons":{}},
      {"code":"OPT-4000","title":"Gros","credits":9,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"OPT-5000","title":"Chaîne inconnue","credits":3,"cycle":1,
       "prerequisites":{"raw":"OPT-9999","tree":"OPT-9999"},
       "equivalents":[],"seasons":{}},
      {"code":"OPT-6000","title":"Concomitant","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-2000*",
         "tree":{"concomitant":"GEX-2000"}},
       "equivalents":[],"seasons":{}},
      {"code":"OPT-7000","title":"Ou bien","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000 OU 90 crédits",
         "tree":{"any":["GEX-1000",
           {"program_credits":{"program":null,"credits":90}}]}},
       "equivalents":[],"seasons":{}},
      {"code":"OPT-8000","title":"Les deux","credits":3,"cycle":1,
       "prerequisites":{"raw":"GEX-1000 ET GEX-2000",
         "tree":{"all":["GEX-1000","GEX-2000"]}},
       "equivalents":[],"seasons":{}}
    ]}"#;

    #[test]
    fn candidates_appear_only_where_room_and_prerequisites_allow() {
        let snapshot = snapshot_with(FEASIBILITY_COURSES);
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[
                {"title":"Règle 1",
                 "constraint":{"type":"course","min":2,"max":2},
                 "courses":["OPT-1000","OPT-2000","OPT-3000","OPT-5000",
                            "OPT-6000","OPT-7000","OPT-8000"]},
                {"title":"Règle 2",
                 "constraint":{"type":"course","min":1,"max":1},
                 "courses":["OPT-4000"]}
              ],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let plan = Plan {
            study_sessions: 3,
            credit_cap: 8,
            program: option_plan().program,
            manual: BTreeMap::from([
                (1, vec!["GEX-1000".to_string()]),
                (2, vec!["GEX-2000".to_string()]),
            ]),
            ..Plan::default()
        };
        let document = build(&snapshot, &plan, Some(&program));

        let offered = |column: usize| -> Vec<&str> {
            document.columns[column]
                .options
                .iter()
                .flat_map(|option_box| &option_box.rules)
                .flat_map(|rule| &rule.choices)
                .map(String::as_str)
                .collect()
        };
        assert_eq!(
            offered(0),
            ["OPT-1000", "OPT-5000"],
            "session 1: nothing placed before it — only the free course \
             and the unknown-source chain pass"
        );
        assert_eq!(
            offered(1),
            ["OPT-1000", "OPT-2000", "OPT-5000", "OPT-6000", "OPT-7000"],
            "session 2: GEX-1000 is behind, GEX-2000 alongside"
        );
        assert_eq!(
            offered(2),
            [
                "OPT-1000", "OPT-2000", "OPT-3000", "OPT-5000", "OPT-6000",
                "OPT-7000", "OPT-8000",
            ],
            "session 4: 6 credits behind, both bases behind"
        );
        // OPT-4000's 9 credits never fit an 8-credit cap: its whole rule
        // is offered nowhere and says so
        assert!(document
            .notes
            .iter()
            .any(|note| note.contains("Aucune session")
                && note.contains("Règle 2")));
    }

    #[test]
    fn reference_and_raw_rules_state_their_sentence_like_a_keyword() {
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 1,
            ..Plan::default()
        };
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[
                {"title":"Règle 7","raw":"selon entente avec la direction"},
                {"title":"Règle 8",
                 "courses":{"concentration":"Génie urbain",
                            "rule":"Règle 1"},
                 "raw":"tous les cours de la Règle 1 de Génie urbain"}
              ],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let rule_report = |title: &str| RuleReport {
            scope: Scope::Program,
            title: title.to_string(),
            status: RuleStatus::Reported,
            counted: None,
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        let report = CoverageReport {
            mandatory: Vec::new(),
            rules: vec![rule_report("Règle 7"), rule_report("Règle 8")],
            language_requirement: None,
        };
        let mut columns = vec![Column {
            label: "A1".to_string(),
            credits: None,
            boxes: Vec::new(),
            options: Vec::new(),
        }];
        let mut notes = Vec::new();
        place_option_boxes(
            &mut columns,
            &[1],
            1,
            &snapshot,
            &plan,
            &report,
            &program,
            None,
            None,
            &mut notes,
        );
        let raws: Vec<&str> = columns[0].options[0]
            .rules
            .iter()
            .filter_map(|rule| rule.raw.as_deref())
            .collect();
        assert_eq!(
            raws,
            [
                "selon entente avec la direction",
                "tous les cours de la Règle 1 de Génie urbain",
            ],
            "no resolved list, the sentence alone"
        );
        assert!(notes.is_empty());
    }

    #[test]
    fn a_rule_past_ten_courses_points_at_the_app_instead_of_listing() {
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 2,
            ..Plan::default()
        };
        let eleven: Vec<String> = (0..11)
            .map(|index| format!("LNG-{:04}", 1000 + index))
            .collect();
        let ten = &eleven[..10];
        let program: Program = serde_json::from_str(&format!(
            r#"{{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[
                {{"title":"Règle 1",
                 "constraint":{{"type":"credits","min":3,"max":3}},
                 "courses":[{}]}},
                {{"title":"Règle 2",
                 "constraint":{{"type":"credits","min":3,"max":3}},
                 "courses":[{}]}}
              ],
              "concentrations":[],"profiles":[]}}"#,
            eleven
                .iter()
                .map(|code| format!("\"{code}\""))
                .collect::<Vec<_>>()
                .join(","),
            ten.iter()
                .map(|code| format!("\"{code}\""))
                .collect::<Vec<_>>()
                .join(","),
        ))
        .unwrap_or_else(|error| panic!("{error}"));
        let document = build(&snapshot, &plan, Some(&program));

        // eleven courses: the sentence, once, in the last roomy column
        assert!(document.columns[0]
            .options
            .iter()
            .flat_map(|option_box| &option_box.rules)
            .all(|rule| rule.title != "Règle 1"));
        let capped = document.columns[1]
            .options
            .iter()
            .flat_map(|option_box| &option_box.rules)
            .find(|rule| rule.title == "Règle 1")
            .unwrap_or_else(|| panic!("Règle 1 missing"));
        assert!(capped.choices.is_empty());
        assert_eq!(
            capped.raw.as_deref(),
            Some(
                "voir les cours disponibles dans l'application ou la page \
                 web du programme"
            )
        );

        // exactly ten: still listed, in every roomy column
        let listed = document.columns[0]
            .options
            .iter()
            .flat_map(|option_box| &option_box.rules)
            .find(|rule| rule.title == "Règle 2")
            .unwrap_or_else(|| panic!("Règle 2 missing"));
        assert_eq!(listed.choices.len(), 10);
        assert!(listed.raw.is_none());
    }

    #[test]
    fn tree_allows_folds_degenerate_and_nested_nodes() {
        let placed = BTreeMap::new();
        let credits = vec![0u32; 3];
        assert!(tree_allows(
            &PrereqTree::All { all: Vec::new() },
            1,
            &placed,
            &credits
        ));
        assert!(tree_allows(
            &PrereqTree::Any { any: Vec::new() },
            1,
            &placed,
            &credits
        ));
        // an `any` nested in an `all`, one branch dead, one alive
        let tree = PrereqTree::All {
            all: vec![PrereqTree::Any {
                any: vec![
                    PrereqTree::ProgramCredits {
                        program_credits:
                            ulaval_scheduler_core::ProgramCredits {
                                program: None,
                                credits: 99,
                            },
                    },
                    PrereqTree::Raw {
                        raw: "au jugé".to_string(),
                    },
                ],
            }],
        };
        assert!(tree_allows(&tree, 1, &placed, &credits));
    }

    #[test]
    fn the_last_rule_lands_in_the_last_column_with_room() {
        // two columns, both roomy: the reverse walk and right-to-left fill
        // put the single missing slot under the LAST session, the official
        // document's own habit for option courses
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 2,
            program: option_plan().program,
            ..Plan::default()
        };
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));
        assert!(
            document.columns[0].options.is_empty()
                || document.columns[1].options.iter().any(|option_box| {
                    option_box.rules.iter().any(|rule| rule.title == "Règle 4")
                }),
            "the last-walked rules stay rightmost"
        );
        let last = &document.columns[1].options[0];
        assert!(
            last.rules.iter().any(|rule| rule.title == "Règle 4"),
            "{:?}",
            last.rules
        );
    }

    #[test]
    fn option_rules_carry_their_constraint_choices_and_raw_text() {
        let document = option_document();
        let listed = find_option_rule(&document, "Règle 1");
        assert_eq!(listed.constraint, "2 cours");
        assert_eq!(listed.choices, ["OPT-A", "OPT-B", "OPT-C"]);
        assert!(listed.raw.is_none());

        let negotiated = find_option_rule(&document, "Règle 3");
        assert_eq!(negotiated.constraint, "3 crédits");
        assert!(negotiated.choices.is_empty());
        assert_eq!(
            negotiated.raw.as_deref(),
            Some("convenus avec la direction")
        );

        let unconstrained = find_option_rule(&document, "Règle 4");
        assert_eq!(unconstrained.constraint, "(contrainte non chiffrée)");
    }

    #[test]
    fn a_report_rule_the_program_lacks_is_skipped_never_a_panic() {
        // defensive branch: `coverage_report` and the program come from the
        // same source, so a mismatch cannot happen through the public entry
        // — exercised directly to prove the degradation stays total
        let snapshot = option_snapshot();
        let plan = option_plan();
        let program = option_program();
        let report = CoverageReport {
            mandatory: Vec::new(),
            rules: vec![RuleReport {
                scope: Scope::Program,
                title: "Règle fantôme".to_string(),
                status: RuleStatus::Incomplete,
                counted: None,
                elsewhere: Vec::new(),
                missing: None,
                candidates: None,
                raw: None,
            }],
            language_requirement: None,
        };
        let mut columns = vec![Column {
            label: "A1".to_string(),
            credits: None,
            boxes: Vec::new(),
            options: Vec::new(),
        }];
        let mut notes = Vec::new();
        place_option_boxes(
            &mut columns,
            &[1],
            1,
            &snapshot,
            &plan,
            &report,
            &program,
            None,
            None,
            &mut notes,
        );
        assert!(columns[0].options.is_empty());
        assert!(
            notes.len() == 1 && notes[0].contains("Règle fantôme"),
            "the unknown rule is offered nowhere, so it is named: {notes:?}"
        );
    }

    #[test]
    fn the_rules_table_reports_constraints_chosen_and_credits() {
        let document = option_document();
        let row = |title: &str| {
            document
                .rules_table
                .iter()
                .find(|row| row.rule == title)
                .unwrap_or_else(|| panic!("{title} missing from the table"))
        };

        let incomplete = row("Règle 1");
        assert_eq!(incomplete.constraint, "2 cours");
        assert_eq!(incomplete.chosen, "à choisir — voir cases grises");
        assert!(!incomplete.resolved);
        assert_eq!(incomplete.credits, "—");

        let ranged = row("Règle 2");
        assert_eq!(ranged.constraint, "4 à 12 crédits");

        let unconstrained = row("Règle 4");
        assert_eq!(unconstrained.constraint, "—");

        let stages = row("Stages");
        assert_eq!(stages.constraint, "1 à 2 cours");
        assert!(stages.resolved);
        assert_eq!(stages.chosen, "STG-1000");
        assert_eq!(stages.credits, "3");
    }

    #[test]
    fn the_preparatory_rule_never_reaches_the_printed_document() {
        let snapshot = option_snapshot();
        // `preparatory_done: false`, or the selection would carry the 0xxx
        // codes and the rule would be skipped as satisfied before the
        // filter under test ever runs
        let plan = Plan {
            preparatory_done: false,
            ..option_plan()
        };
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[{"title":"Scolarité préparatoire",
                "constraint":{"type":"course","min":1,"max":1},
                "courses":["MAT-0150"]}],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let document = build(&snapshot, &plan, Some(&program));
        assert!(document.rules_table.is_empty());
        assert!(document
            .columns
            .iter()
            .all(|column| column.options.is_empty()));
    }

    #[test]
    fn an_incomplete_stage_rule_names_its_candidates() {
        // the same program, nothing selected at all: the stages row lists
        // the stage codes themselves — they have no grey box to point at
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 2,
            program: option_plan().program,
            manual: BTreeMap::from([(1, vec!["GEX-1000".to_string()])]),
            ..Plan::default()
        };
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));
        let stages = document
            .rules_table
            .iter()
            .find(|row| row.rule == "Stages")
            .unwrap_or_else(|| panic!("Stages missing"));
        assert!(!stages.resolved);
        assert_eq!(stages.chosen, "à choisir — STG-1000, STG-2000");
    }

    #[test]
    fn chosen_text_covers_partial_and_candidate_less_branches() {
        let report = RuleReport {
            scope: Scope::Program,
            title: "Règle 1".to_string(),
            status: RuleStatus::Incomplete,
            counted: None,
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        assert_eq!(
            chosen_text(&report, &["OPT-A"], false),
            "OPT-A — à compléter, voir cases grises"
        );
        let stages = RuleReport {
            title: STAGES_RULE_TITLE.to_string(),
            candidates: Some(Vec::new()),
            ..report
        };
        assert_eq!(
            chosen_text(&stages, &[], false),
            "à choisir — voir cases grises",
            "an empty candidate list falls back like any other rule"
        );
    }

    #[test]
    fn counted_courses_earn_their_rule_chip_and_hp_its_dashed_state() {
        let snapshot = snapshot_with(
            r#"{"courses":[
              {"code":"GEX-1000","title":"Obligatoire","credits":3,"cycle":1,
               "prerequisites":null,"equivalents":[],"seasons":{}},
              {"code":"OPT-A","title":"Option","credits":3,"cycle":1,
               "prerequisites":null,"equivalents":[],"seasons":{}},
              {"code":"STG-1000","title":"Stage I","credits":9,"cycle":1,
               "prerequisites":null,"equivalents":[],"seasons":{}},
              {"code":"LIB-1000","title":"Libre","credits":3,"cycle":1,
               "prerequisites":null,"equivalents":[],"seasons":{}}
            ]}"#,
        );
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":["GEX-1000"],
              "rules":[
                {"title":"Règle 2",
                 "constraint":{"type":"course","min":1,"max":1},
                 "courses":["OPT-A"]},
                {"title":"Stages",
                 "constraint":{"type":"course","min":1,"max":2},
                 "courses":["STG-1000"],"credits_in_addition":true},
                {"title":"Hors programme","courses":["LIB-1000"]}
              ],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let plan = Plan {
            study_sessions: 2,
            manual: BTreeMap::from([
                (1, vec!["GEX-1000".to_string(), "OPT-A".to_string()]),
                (2, vec!["LIB-1000".to_string()]),
                (3, vec!["STG-1000".to_string()]),
            ]),
            ..Plan::default()
        };
        let document = build(&snapshot, &plan, Some(&program));

        assert_eq!(find(&document, "OPT-A").tag.as_deref(), Some("R2"));
        assert!(!find(&document, "OPT-A").optional);
        let libre = find(&document, "LIB-1000");
        assert_eq!(libre.tag.as_deref(), Some("HP"));
        assert!(libre.optional, "hors programme draws dashed");
        assert!(
            !document
                .rules_table
                .iter()
                .any(|row| row.rule == "Hors programme"),
            "« Hors programme » is bookkeeping, never a table row"
        );
        assert!(
            find(&document, "STG-1000").tag.is_none(),
            "a stage box is a plain course"
        );
        assert!(find(&document, "GEX-1000").tag.is_none());

        // the personal strip: 18 placed − 9 stage − 3 HP = 6 in-program
        let values: Vec<(&str, &str)> = document
            .stats
            .iter()
            .map(|stat| (stat.value.as_str(), stat.label.as_str()))
            .collect();
        // every rule is either satisfied or reported-with-courses: a
        // complete plan shows no grey box at all (retour d'Antoine)
        assert!(document
            .columns
            .iter()
            .all(|column| column.options.is_empty()));

        assert_eq!(
            values,
            [
                ("6 / 90", "crédits choisis"),
                ("1", "stages"),
                ("3 cr", "hors programme"),
            ],
            "no « reconnus » cell: nothing credited"
        );
    }

    #[test]
    fn rule_tags_require_a_number_or_the_hp_title() {
        assert_eq!(rule_tag("Règle 12").as_deref(), Some("R12"));
        assert_eq!(rule_tag("Hors programme").as_deref(), Some("HP"));
        assert!(rule_tag("Stages").is_none());
        assert!(rule_tag("Règle indéfinie").is_none());
        assert!(rule_tag("Scolarité préparatoire").is_none());
    }

    #[test]
    fn credited_courses_count_toward_the_program_and_fill_a_sidebox() {
        let snapshot = option_snapshot();
        let mut plan = option_plan();
        plan.credited.insert("GEX-3000".to_string());
        plan.credited.insert("ZZZ-9999".to_string());
        plan.manual = BTreeMap::from([(1, vec!["GEX-1000".to_string()])]);
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));

        assert!(document
            .stats
            .iter()
            .any(|stat| stat.value == "3 cr" && stat.label == "reconnus"));

        let sidebox = &document.requirements[0];
        assert_eq!(sidebox.lines[0].lead.as_deref(), Some("GEX-3000"));
        assert_eq!(sidebox.lines[0].text, "Suite 2 — 3 cr");
        assert_eq!(
            sidebox.lines[1].text, "absent du catalogue de cours",
            "an unknown credited code stays visible, never dropped"
        );
        assert_eq!(
            sidebox.lines[2].text,
            "Reconnaissance des acquis : crédité, hors session."
        );
    }

    #[test]
    fn ordinal_stats_report_the_remaining_credits_to_choose() {
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 2,
            program: option_plan().program,
            ..Plan::default()
        };
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));
        assert_eq!(document.stats[0].value, "0 / 90");
        assert_eq!(document.stats[0].label, "crédits choisis");
        assert_eq!(document.stats[1].value, "90 cr");
        assert_eq!(document.stats[1].label, "à choisir, règles 1 à 4");
    }

    #[test]
    fn rules_range_label_degrades_without_numbered_rules() {
        let report_with = |titles: &[&str]| CoverageReport {
            mandatory: Vec::new(),
            rules: titles
                .iter()
                .map(|title| RuleReport {
                    scope: Scope::Program,
                    title: title.to_string(),
                    status: RuleStatus::Reported,
                    counted: None,
                    elsewhere: Vec::new(),
                    missing: None,
                    candidates: None,
                    raw: None,
                })
                .collect(),
            language_requirement: None,
        };
        assert_eq!(rules_range_label(&report_with(&["Stages"])), "à choisir");
        assert_eq!(
            rules_range_label(&report_with(&["Règle 1"])),
            "à choisir, règle 1"
        );
    }

    #[test]
    fn a_coverage_error_degrades_to_a_note_never_a_panic() {
        // a plan naming a concentration the program does not carry makes
        // `coverage_report` fail — the document must still build
        let snapshot = option_snapshot();
        let mut plan = option_plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Inexistante".to_string());
        }
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));
        assert!(document.rules_table.is_empty());
        assert!(document
            .notes
            .iter()
            .any(|note| note.contains("Impossible de calculer")));
        assert_eq!(
            document.stats[0].label, "crédits placés",
            "no report, no program total to claim"
        );
    }

    #[test]
    fn preparatory_done_extends_the_selection_without_changing_this_program() {
        // nothing in `OPTION_PROGRAM` names a préparatoire rule, so the
        // outcome must be identical with the checkbox on or off — this
        // exercises both branches of `if plan.preparatory_done`
        let with = option_document();
        let mut plan = option_plan();
        plan.preparatory_done = false;
        let without =
            build(&option_snapshot(), &plan, Some(&option_program()));
        assert_eq!(with.rules_table, without.rules_table);
    }

    // --- cours de stage du cheminement type (mode ordinal) ----------------

    fn stage_program(stage_codes: &[&str]) -> Program {
        let list = stage_codes
            .iter()
            .map(|code| format!("\"{code}\""))
            .collect::<Vec<_>>()
            .join(",");
        serde_json::from_str(&format!(
            r#"{{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[{{"title":"Stages",
                "constraint":{{"type":"course","min":1,"max":8}},
                "courses":[{list}],"credits_in_addition":true}}],
              "concentrations":[],"profiles":[]}}"#
        ))
        .unwrap_or_else(|error| panic!("{error}"))
    }

    // STG-1000 carries a program-credits prerequisite: its synthetic card
    // must show the numeric token like any placed box would
    const STAGE_COURSES: &str = r#"{"courses":[
      {"code":"STG-1000","title":"Stage I","credits":9,"cycle":1,
       "prerequisites":{"raw":"Crédits exigés : 24",
         "tree":{"program_credits":{"program":null,"credits":24}}},
       "equivalents":[],"seasons":{}},
      {"code":"STG-2000","title":"Stage II","credits":9,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"STG-3000","title":"Stage III","credits":9,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"STG-4000","title":"Stage IV","credits":9,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}}
    ]}"#;

    #[test]
    fn ordinal_stage_cards_sit_after_each_winter_and_stop_at_the_horizon() {
        let snapshot = snapshot_with(STAGE_COURSES);
        let plan = Plan {
            study_sessions: 8,
            program: option_plan().program,
            ..Plan::default()
        };
        let program =
            stage_program(&["STG-1000", "STG-2000", "STG-3000", "STG-4000"]);
        let document = build(&snapshot, &plan, Some(&program));
        let groups: Vec<(usize, usize, &str, bool)> = document
            .summers
            .iter()
            .map(|group| {
                (
                    group.first_column,
                    group.last_column,
                    group.boxes[0].code.as_str(),
                    group.boxes[0].optional,
                )
            })
            .collect();
        assert_eq!(
            groups,
            [
                (1, 2, "STG-1000", false),
                (3, 4, "STG-2000", true),
                (5, 6, "STG-3000", true),
            ],
            "the first stage is the expected path, the later ones optional; \
             stage IV has no été before an eighth-plus column to sit in"
        );
        assert!(document
            .summers
            .iter()
            .all(|group| group.label.is_none() && group.credits.is_none()));
        let first = &document.summers[0].boxes[0];
        assert_eq!(first.title, "Stage I");
        assert_eq!(first.credits, "9 cr");
        assert_eq!(
            first.entry,
            vec![Token::Credits { credits: 24 }],
            "a synthetic card carries its tokens like any placed box"
        );
    }

    #[test]
    fn one_stage_per_ete_placed_or_synthetic_never_two() {
        // horizon A…H over 6 sessions: étés at anchors 1 and 3. STG-4000
        // (not in this rule) occupies the first été without spending it,
        // so stage I (STG-9999, unknown to the catalogue) joins its group;
        // STG-1000 placed in the second été spends that anchor, and
        // STG-2000 finds no free été left — not drawn, never stacked
        let snapshot = snapshot_with(STAGE_COURSES);
        let mut plan = Plan {
            study_sessions: 6,
            program: option_plan().program,
            ..Plan::default()
        };
        plan.displayed_placement.insert("STG-4000".to_string(), 3);
        plan.displayed_placement.insert("STG-1000".to_string(), 6);
        let program = stage_program(&["STG-9999", "STG-1000", "STG-2000"]);
        let document = build(&snapshot, &plan, Some(&program));
        let groups: Vec<(usize, usize, Vec<&str>)> = document
            .summers
            .iter()
            .map(|group| {
                (
                    group.first_column,
                    group.last_column,
                    group
                        .boxes
                        .iter()
                        .map(|course_box| course_box.code.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            [
                (1, 2, vec!["STG-4000", "STG-9999"]),
                (3, 4, vec!["STG-1000"]),
            ],
            "no second stage ever joins an été that already holds one"
        );
        assert!(
            !document.summers[0].boxes[1].optional,
            "the first stage of the rule stays the expected path"
        );
        assert!(document.notes.iter().any(|note| note.contains("STG-9999")
            && note.contains("absent du catalogue")));
    }

    #[test]
    fn a_later_stage_never_lands_before_a_placed_earlier_one() {
        let snapshot = snapshot_with(STAGE_COURSES);
        let mut plan = Plan {
            study_sessions: 8,
            program: option_plan().program,
            ..Plan::default()
        };
        plan.displayed_placement.insert("STG-1000".to_string(), 6);
        let program = stage_program(&["STG-1000", "STG-2000", "STG-3000"]);
        let document = build(&snapshot, &plan, Some(&program));
        let groups: Vec<(usize, usize, Vec<&str>)> = document
            .summers
            .iter()
            .map(|group| {
                (
                    group.first_column,
                    group.last_column,
                    group
                        .boxes
                        .iter()
                        .map(|course_box| course_box.code.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            [(3, 4, vec!["STG-1000"]), (5, 6, vec!["STG-2000"]),],
            "a stage after stage I never uses an earlier été"
        );

        let mut regular_plan = Plan {
            study_sessions: 8,
            program: option_plan().program,
            ..Plan::default()
        };
        regular_plan
            .displayed_placement
            .insert("STG-1000".to_string(), 4);
        let document = build(&snapshot, &regular_plan, Some(&program));
        let groups: Vec<(usize, usize, Vec<&str>)> = document
            .summers
            .iter()
            .map(|group| {
                (
                    group.first_column,
                    group.last_column,
                    group
                        .boxes
                        .iter()
                        .map(|course_box| course_box.code.as_str())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            groups,
            [(3, 4, vec!["STG-2000"]), (5, 6, vec!["STG-3000"]),],
            "a stage in a regular session also bounds following stages"
        );
    }

    #[test]
    fn stage_cards_need_an_ordinal_plan_and_a_listed_stage_rule() {
        // personal mode: only the really placed été, no synthetic card
        let personal = option_document();
        assert_eq!(personal.summers.len(), 1);
        assert_eq!(personal.summers[0].boxes[0].code, "STG-1000");

        let snapshot = snapshot_with(STAGE_COURSES);
        let ordinal_plan = Plan {
            study_sessions: 8,
            program: option_plan().program,
            ..Plan::default()
        };
        let no_rules: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],"rules":[],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let document = build(&snapshot, &ordinal_plan, Some(&no_rules));
        assert!(document.summers.is_empty());

        let negotiated: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[{"title":"Stages","courses":"negotiated",
                "raw":"convenus avec la direction"}],
              "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let document = build(&snapshot, &ordinal_plan, Some(&negotiated));
        assert!(
            document.summers.is_empty(),
            "a keyword stage rule lists no course to draw"
        );
    }

    // --- exigences, légende, méta, entête ---------------------------------

    #[test]
    fn the_language_requirement_becomes_an_exigence_sidebox() {
        let document = option_document();
        let language = document
            .requirements
            .iter()
            .find(|sidebox| {
                sidebox.lines[0].lead.as_deref()
                    == Some("Exigence linguistique.")
            })
            .unwrap_or_else(|| panic!("language sidebox missing"));
        assert_eq!(language.lines[0].text, "Réussir ANL-2020 pour diplômer.");
    }

    #[test]
    fn language_text_spells_tests_and_the_non_francophone_branch() {
        let requirement: LanguageRequirement = serde_json::from_str(
            r#"{"francophone":{"course":"ANL-2020",
                "tests":[{"name":"VEPT","score":53}],"raw":"ANL-2020"},
              "non_francophone":{"course":"FLS-2093",
                "tests":[{"name":"TCF-TP","score":400},
                         {"name":"TCF-TP/ÉÉ","score":14}],
                "raw":"FLS-2093"}}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            language_text(&requirement),
            "Réussir ANL-2020 (ou VEPT ≥ 53) pour diplômer; personne non \
             francophone : FLS-2093 (ou TCF-TP ≥ 400 et TCF-TP/ÉÉ ≥ 14)."
        );
    }

    #[test]
    fn a_document_without_requirements_shows_no_empty_box() {
        let document = document();
        assert!(document.requirements.is_empty());
    }

    #[test]
    fn the_legend_carries_the_six_typed_entries() {
        let legend = build_legend();
        assert_eq!(legend.len(), 6);
        assert!(matches!(&legend[0], LegendEntry::Letter { .. }));
        assert!(matches!(&legend[1], LegendEntry::Shaded { text }
            if text.contains("concomitance")));
        assert!(matches!(&legend[2], LegendEntry::Credits { .. }));
        assert!(matches!(&legend[3], LegendEntry::Chip { chip, .. }
            if chip == "R3"));
        assert!(matches!(&legend[4], LegendEntry::Chip { chip, .. }
            if chip == "HP"));
        assert!(matches!(&legend[5], LegendEntry::Optional { text }
            if text.contains("optionnel")));
    }

    #[test]
    fn the_meta_block_carries_provenance_and_the_share_link() {
        let document = document();
        assert_eq!(document.meta.generated, "Généré le 2026-08-25 à 14:00");
        assert_eq!(document.meta.data, "Données du répertoire : 2026-08-01");
        assert!(document.meta.build.starts_with("app v"));
        assert!(document.meta.build.contains("code dev"));
        assert!(document.meta.build.contains("données dev"));
        assert_eq!(
            document.meta.repo_label,
            "github.com/antoinelb/ulaval-generateur-horaire"
        );
        assert!(document.meta.repo_url.starts_with("https://"));
        assert_eq!(document.meta.share_url, "app#plan");
        assert_eq!(
            document.meta.share_label,
            "Accéder à cet organigramme dans l'application"
        );
        assert_eq!(document.kicker, "Organigramme des cours");
        assert_eq!(document.program_title, "Programme non choisi");
        assert!(document.disclaimer.contains("version officielle"));
    }

    #[test]
    fn generated_line_degrades_shape_by_shape() {
        assert_eq!(
            generated_line("2026-08-25T14:00:00"),
            "Généré le 2026-08-25 à 14:00"
        );
        assert_eq!(
            generated_line("2026-08-25T14:00:00Z"),
            "Généré le 2026-08-25 à 14:00 UTC",
            "a Z stamp is spelled out as UTC"
        );
        assert_eq!(
            generated_line("hier"),
            "Généré le hier",
            "an unexpected shape degrades to the raw string"
        );
        assert_eq!(
            generated_line("2026-08-25T14:00"),
            "Généré le 2026-08-25T14:00",
            "a stamp without seconds is not the boundary's shape"
        );
        assert_eq!(
            generated_line("T14:00:00"),
            "Généré le T14:00:00",
            "a dateless stamp degrades whole rather than half-formatted"
        );
    }

    #[test]
    fn find_rule_resolves_every_scope_and_defends_against_mismatches() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
              "title":"Génie des eaux","cycle":1,"credits_required":90,
              "mandatory":[],
              "rules":[{"title":"Règle 1","courses":["OPT-A"]}],
              "concentrations":[{"title":"Génie urbain","mandatory":[],
                "rules":[{"title":"Règle C","courses":["CON-A"]}]}],
              "profiles":[{"title":"Profil international","mandatory":[],
                "rules":[{"title":"Règle P","courses":["PRO-A"]}]}]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));

        assert!(find_rule(&program, None, None, Scope::Program, "Règle 1")
            .is_some());
        assert!(find_rule(&program, None, None, Scope::Program, "Absente")
            .is_none());
        assert!(find_rule(
            &program,
            Some("Génie urbain"),
            None,
            Scope::Concentration,
            "Règle C"
        )
        .is_some());
        assert!(
            find_rule(&program, None, None, Scope::Concentration, "Règle C")
                .is_none(),
            "no chosen concentration, nothing to search"
        );
        assert!(find_rule(
            &program,
            None,
            Some("Profil international"),
            Scope::Profile,
            "Règle P"
        )
        .is_some());
        assert!(
            find_rule(
                &program,
                None,
                Some("Inconnu"),
                Scope::Profile,
                "Règle P"
            )
            .is_none(),
            "an unknown profile resolves no rule list"
        );
    }

    #[test]
    fn collect_operands_flattens_all_and_any_in_source_order() {
        let tree = PrereqTree::All {
            all: vec![
                PrereqTree::Course("GEX-1000".to_string()),
                PrereqTree::Any {
                    any: vec![
                        PrereqTree::Course("GEX-2000".to_string()),
                        PrereqTree::Course("GEX-3000".to_string()),
                    ],
                },
            ],
        };
        assert_eq!(
            collect_operands(&tree),
            vec![
                Operand::Course("GEX-1000".to_string()),
                Operand::Course("GEX-2000".to_string()),
                Operand::Course("GEX-3000".to_string()),
            ],
            "left to right, the source text's own order"
        );
    }

    #[test]
    fn ordinal_stats_omit_a_zero_remaining_cell() {
        let snapshot = option_snapshot();
        let plan = Plan {
            study_sessions: 2,
            program: option_plan().program,
            ..Plan::default()
        };
        let mut program = option_program();
        program.credits_required = 0;
        let document = build(&snapshot, &plan, Some(&program));
        assert_eq!(document.stats.len(), 1, "nothing left to choose");
    }

    #[test]
    fn zero_study_sessions_still_reports_rules_but_places_no_options() {
        let snapshot = option_snapshot();
        let mut plan = Plan {
            study_sessions: 0,
            program: option_plan().program,
            ..Plan::default()
        };
        // GEX-1000 is not mandatory in this fixture, so its pin is a
        // supplemental-course choice and makes the plan personal.
        plan.pinned_sessions.insert("GEX-1000".to_string(), 1);
        let program = option_program();
        let document = build(&snapshot, &plan, Some(&program));
        assert!(document.columns.is_empty());
        assert!(!document.rules_table.is_empty());
        assert_eq!(document.subtitle, "B-GEX, version A26");
    }
}
