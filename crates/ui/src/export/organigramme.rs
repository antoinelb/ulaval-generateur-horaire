// The pure model of the printed « organigramme » document — colonnes,
// cases de cours, jetons de préalables — reproducing the token grammar of
// `gex_organigramme.pdf`'s « Note 1 » legend (read before writing this
// file): a letter token right of the box that serves as a prerequisite
// (jeton de sortie), the same letter left of every box that requires it
// (jeton d'entrée), shaded when the requirement is concomitant, and a
// numeric token for a program-credits threshold. No Dioxus, no web-sys, no
// clock: `components/print/organigramme.rs` (task 5) renders this model.

use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    coverage_report, horizon_sessions, Constraint, Course, CoverageReport,
    Missing, PrereqTree, Prerequisites, Program, Rule, RuleCourses,
    RuleReport, RuleStatus, Scope, STAGES_RULE_TITLE,
};

use crate::data::Snapshot;
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
    pub title: String,
    pub program_title: String,
    pub version: String,
    pub columns: Vec<Column>,
    pub legend: Vec<String>,
    pub notes: Vec<String>,
    // « Microprogramme de stage I/II/III… » bands, spanning the columns
    // between the session preceding a stage's placement and the stage
    // itself
    pub spans: Vec<SpanBox>,
    // « Cours de langue » box — `None` when the program has no language
    // requirement at all
    pub language: Option<LanguageBox>,
    // the official document's small rules table: one row per rule of the
    // effective program (program scope plus the chosen concentration and
    // profile). The document's own per-cheminement columns (Commun / Plus
    // de conception / Moins de conception) are NOT computable from what
    // core exposes — no data source ties a course to one of those three
    // cheminements — so this table stays a flat rule/constraint/chosen
    // list; nobody should re-add the per-cheminement split without a new
    // data source to compute it from.
    pub rules_table: Vec<RulesRow>,
    // the « N.B. Vous devez faire … » sentence(s), built only from counts
    // this module itself computed — empty when there is nothing to say
    pub bands: Vec<String>,
    pub provenance: ExportProvenance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    // the full session identity, « A1-A26 » (`state::session_label`)
    pub label: String,
    // the official document's column head, « A1 » (`state::session_short`)
    pub semester: String,
    pub boxes: Vec<CourseBox>,
    // the grey « Cours option » box for this column, when at least one
    // unsatisfied rule placed a slot here — at most one per column
    // (`place_option_boxes` never pushes a second)
    pub options: Vec<OptionBox>,
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
pub struct SpanBox {
    pub label: String,
    pub first_column: usize,
    pub last_column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageBox {
    pub label: String,
    pub detail: String,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RulesRow {
    pub rule: String,
    pub constraint: String,
    pub chosen: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CourseBox {
    pub code: String,
    pub title: String,
    pub credits: String,
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
    Raw(String),
}

pub fn organigramme_document(
    snapshot: &Snapshot,
    plan: &Plan,
    program: Option<&Program>,
    generated_at: &str,
) -> OrganigrammeDocument {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let semesters = state::session_semesters(plan.start, &seasons);

    let mut notes: Vec<String> = program
        .map(|program| program.notes.clone())
        .unwrap_or_default();

    // built in the same pass as the boxes themselves, so `by_code` and
    // `courses` are each read exactly once per box
    let mut columns: Vec<Column> = Vec::with_capacity(semesters.len());
    let mut operands: Vec<Vec<Vec<Operand>>> =
        Vec::with_capacity(semesters.len());
    // first document position of each code — the token pass's only source
    // of truth for « is this a box in the document »
    let mut box_position: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for column_index in 0..semesters.len() {
        let session = column_index + 1;
        let codes = state::session_codes(plan, session);
        let mut boxes = Vec::with_capacity(codes.len());
        let mut column_operands = Vec::with_capacity(codes.len());
        for (row_index, code) in codes.into_iter().enumerate() {
            box_position
                .entry(code.clone())
                .or_insert((column_index, row_index));
            let course: Option<&Course> = snapshot
                .by_code
                .get(&code)
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
            column_operands.push(operands_of(
                course.and_then(|course| course.prerequisites.as_ref()),
            ));
            boxes.push(CourseBox {
                code,
                title,
                credits,
                entry: Vec::new(),
                exit: None,
            });
        }
        columns.push(Column {
            label: state::session_label(&semesters, column_index),
            semester: state::session_short(&semesters, column_index),
            boxes,
            options: Vec::new(),
        });
        operands.push(column_operands);
    }

    assign_tokens(&mut columns, &operands, &box_position, &mut notes);

    let concentration = plan
        .program
        .as_ref()
        .and_then(|choice| choice.concentration.as_deref());
    let profile = plan
        .program
        .as_ref()
        .and_then(|choice| choice.profile.as_deref());

    let mut spans = Vec::new();
    let mut language = None;
    let mut rules_table = Vec::new();
    let mut bands = Vec::new();

    // every field below depends on a chosen program: no program, nothing
    // to place or report — the columns and their courses still stand alone
    if let Some(program) = program {
        spans = build_spans(program, &box_position, &mut notes);
        language = build_language(program, &box_position);

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
                let total_slots = place_option_boxes(
                    &mut columns,
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
                    program,
                    concentration,
                    profile,
                );
                bands = build_bands(total_slots, &language);
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

    let program_title = program
        .map(|program| program.title.clone())
        .or_else(|| plan.program.as_ref().map(|choice| choice.code.clone()))
        .unwrap_or_else(|| "Programme non choisi".to_string());
    let version = plan
        .program
        .as_ref()
        .map(|choice| format!("Version {}", choice.semester))
        .unwrap_or_else(|| "Version inconnue".to_string());

    OrganigrammeDocument {
        title: "Organigramme des cours".to_string(),
        program_title,
        version,
        columns,
        legend: legend_text(),
        notes,
        spans,
        language,
        rules_table,
        bands,
        provenance: export_provenance(
            generated_at,
            snapshot.provenance.scraped_at.as_deref(),
            snapshot.provenance.course_count,
        ),
    }
}

// The token-assignment walk: every box in document order (column, then
// row), every operand of its own tree in tree order. A source code that is
// a box in the document earns its letter here, the first time some
// dependent needs it — never when the source box itself is walked, which
// is what keeps a source with no dependent silently letter-less.
fn assign_tokens(
    columns: &mut [Column],
    operands: &[Vec<Vec<Operand>>],
    box_position: &BTreeMap<String, (usize, usize)>,
    notes: &mut Vec<String>,
) {
    let mut letters: BTreeMap<String, String> = BTreeMap::new();
    let mut next_letter = 0usize;

    for column_index in 0..columns.len() {
        for (row_index, box_operands) in
            operands[column_index].iter().enumerate()
        {
            let dependent =
                columns[column_index].boxes[row_index].code.clone();
            for operand in box_operands {
                match operand {
                    Operand::Course(source) | Operand::Concomitant(source) => {
                        let Some(&(source_column, source_row)) =
                            box_position.get(source)
                        else {
                            notes.push(format!(
                                "{source}, préalable de {dependent}, n'est \
                                 dans aucune session du document."
                            ));
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
                        if columns[source_column].boxes[source_row]
                            .exit
                            .is_none()
                        {
                            columns[source_column].boxes[source_row].exit =
                                Some(Token::Letter {
                                    letter: letter.clone(),
                                    shaded: false,
                                });
                        }
                        // the legend's « cours concomitant » : the
                        // répertoire's `*` on the prerequisite itself —
                        // the source may be taken before or alongside,
                        // whatever column the plan placed it in
                        let shaded =
                            matches!(operand, Operand::Concomitant(_));
                        columns[column_index].boxes[row_index]
                            .entry
                            .push(Token::Letter { letter, shaded });
                    }
                    Operand::Credits(credits) => {
                        columns[column_index].boxes[row_index]
                            .entry
                            .push(Token::Credits { credits: *credits });
                    }
                    Operand::Raw(raw) => {
                        notes.push(format!(
                            "{dependent} : préalable non représenté sur \
                             l'organigramme : {raw}"
                        ));
                    }
                }
            }
        }
    }
}

// --- « cours option » boxes, microprogrammes de stage, langue, règles -----

// The répertoire's ordinary course weight — one unfilled option slot is
// priced at this many credits when sizing how much room it needs in a
// column, and the same figure sizes a `Missing::Credits` verdict down to a
// slot count. A deliberate, documented choice (the export feature's ADR):
// nothing in a `Rule` says how many credits its own missing courses are
// worth individually, so the ordinary course weight stands in.
const OPTION_SLOT_CREDITS: i64 = 3;

// far above any real program's number of missing option slots — an
// explicit bound, not an open-ended loop
const MAX_OPTION_SLOTS: usize = 500;

// Fills `Column::options` from the coverage report: every rule that is not
// `Satisfied` earns an entry, sized from how much it still misses and
// placed into the columns that still have room, left to right — the
// deliberate, documented choice (same ADR as `OPTION_SLOT_CREDITS`) that
// mirrors the official document's own boxes sitting under the earliest
// columns that still have space once the mandatory courses are laid out.
// Returns the total number of slots demanded (placed or not), for `bands`.
#[allow(clippy::too_many_arguments)]
fn place_option_boxes(
    columns: &mut [Column],
    snapshot: &Snapshot,
    plan: &Plan,
    report: &CoverageReport,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    notes: &mut Vec<String>,
) -> usize {
    if columns.is_empty() {
        return 0;
    }

    // one entry per column, decremented as slots land — shared across
    // every rule so a later rule sees the room an earlier one already used
    let mut remaining: Vec<i64> = (0..columns.len())
        .map(|index| {
            let session = index + 1;
            let used =
                crate::solve::session_credits(snapshot, plan, session).total;
            i64::from(plan.credit_cap) - i64::from(used)
        })
        .collect();

    let mut assigned: Vec<Vec<OptionRule>> = vec![Vec::new(); columns.len()];
    // a rule shows at most once per column's box, even when several of its
    // slots land there
    let mut already_in_column: Vec<BTreeSet<String>> =
        vec![BTreeSet::new(); columns.len()];
    let mut total_slots = 0usize;

    for rule_report in &report.rules {
        if rule_report.status == RuleStatus::Satisfied {
            continue;
        }
        let Some(rule) = find_rule(
            program,
            concentration,
            profile,
            rule_report.scope,
            &rule_report.title,
        ) else {
            // defensive: `coverage_report` reports exactly the rules the
            // same `program` carries, so this cannot happen — but the
            // document degrades by skipping the rule rather than panicking
            continue;
        };
        let needed = needed_slots(rule_report).min(MAX_OPTION_SLOTS);
        total_slots += needed;
        let option_rule = build_option_rule(rule_report, rule);

        for _ in 0..needed {
            let has_room =
                remaining.iter().any(|&room| room >= OPTION_SLOT_CREDITS);
            let target = if has_room {
                remaining
                    .iter()
                    .position(|&room| room >= OPTION_SLOT_CREDITS)
                    // `has_room` just proved one exists; the fallback is
                    // unreachable but keeps this total rather than panicky
                    .unwrap_or(0)
            } else {
                notes.push(format!(
                    "Un choix de « {} » n'a pas trouvé de place dans \
                     l'horizon.",
                    rule_report.title
                ));
                columns.len() - 1
            };
            if has_room {
                remaining[target] -= OPTION_SLOT_CREDITS;
            }
            if already_in_column[target].insert(rule_report.title.clone()) {
                assigned[target].push(option_rule.clone());
            }
        }
    }

    for (index, rules) in assigned.into_iter().enumerate() {
        if rules.is_empty() {
            continue;
        }
        columns[index].options.push(OptionBox {
            heading: format!(
                "Cours option\nChoix possibles en {}",
                columns[index].semester
            ),
            rules,
        });
    }

    total_slots
}

// `Missing::Count` converts to slots directly; `Missing::Credits` divides
// by the slot weight, rounded up so a missing amount that does not divide
// evenly still earns a whole extra slot. A rule with no `Missing` at all
// (constraint `None`, or a keyword/reference/raw list with nothing to
// count) still needs to be shown once — a single representative slot,
// since there is no numeric target to size it from otherwise.
fn needed_slots(rule_report: &RuleReport) -> usize {
    match rule_report.missing {
        Some(Missing::Count { count }) => {
            usize::try_from(count.max(0)).unwrap_or(0)
        }
        Some(Missing::Credits { credits }) => {
            let credits = credits.max(0);
            usize::try_from(
                (credits + OPTION_SLOT_CREDITS - 1) / OPTION_SLOT_CREDITS,
            )
            .unwrap_or(0)
        }
        None => 1,
    }
}

// `choices` only ever enumerates a plain `RuleCourses::List` — a Reference
// resolves to a list too, but is kept out of `choices` on purpose (the
// task's own rule): only its `raw` text is shown, same as a keyword or a
// bare raw rule.
fn build_option_rule(rule_report: &RuleReport, rule: &Rule) -> OptionRule {
    let (choices, raw) = match &rule.courses {
        RuleCourses::List { .. } => {
            (rule_report.candidates.clone().unwrap_or_default(), None)
        }
        RuleCourses::Reference { raw, .. }
        | RuleCourses::Keyword { raw, .. }
        | RuleCourses::Raw { raw } => (Vec::new(), Some(raw.clone())),
    };
    OptionRule {
        title: rule_report.title.clone(),
        constraint: constraint_text(
            &rule_report.title,
            rule.constraint.as_ref(),
        ),
        choices,
        raw,
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

// « Règle N – k à m cours/crédits » — the rule's own title already reads
// « Règle N », so this only appends the counted unit; a rule with no
// number at all (ADR `2026-07-contrainte-de-regle-optionnelle`) renders the
// fixed « (contrainte non chiffrée) » instead, with no title prefix, since
// there is no number to attach to one.
fn constraint_text(title: &str, constraint: Option<&Constraint>) -> String {
    match constraint {
        None => "(contrainte non chiffrée)".to_string(),
        Some(&Constraint::Course { min, max }) if min == max => {
            format!("{title} – {min} cours")
        }
        Some(&Constraint::Course { min, max }) => {
            format!("{title} – {min} à {max} cours")
        }
        Some(&Constraint::Credits { min, max }) => {
            format!("{title} – {min} à {max} crédits")
        }
    }
}

// The document's small rules table: every rule of the effective program,
// satisfied or not (unlike the option boxes, which skip satisfied rules).
fn build_rules_table(
    report: &CoverageReport,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> Vec<RulesRow> {
    report
        .rules
        .iter()
        .map(|rule_report| {
            let constraint = find_rule(
                program,
                concentration,
                profile,
                rule_report.scope,
                &rule_report.title,
            )
            .and_then(|rule| rule.constraint.as_ref());
            RulesRow {
                rule: rule_report.title.clone(),
                constraint: constraint_text(&rule_report.title, constraint),
                chosen: chosen_label(rule_report, constraint),
            }
        })
        .collect()
}

// « k / m » : how many the student has chosen against how many the rule
// asks for. For an evaluated rule (a `Constraint` exists) `m` is the
// constraint's own minimum and `k` is read back off `Missing` when the
// rule is short of it, or taken to have reached `m` otherwise (satisfied,
// or an evaluated rule the report never actually verdicted). For a
// constraint-less rule there is no minimum to read `m` from, so `k`/`m`
// fall back to how many of the listed courses are already counted, when
// the rule's list was resolved at all; a keyword/raw rule resolves no list
// and renders the explicit « — / — ».
fn chosen_label(
    rule_report: &RuleReport,
    constraint: Option<&Constraint>,
) -> String {
    match constraint {
        Some(&Constraint::Course { min, .. }) => {
            let chosen = match rule_report.missing {
                Some(Missing::Count { count }) => (min - count).max(0),
                _ => min,
            };
            format!("{chosen} / {min}")
        }
        Some(&Constraint::Credits { min, .. }) => {
            let chosen = match rule_report.missing {
                Some(Missing::Credits { credits }) => (min - credits).max(0),
                _ => min,
            };
            format!("{chosen} / {min}")
        }
        None => match (&rule_report.counted, &rule_report.candidates) {
            (Some(counted), Some(candidates)) => format!(
                "{} / {}",
                counted.len(),
                counted.len() + candidates.len()
            ),
            _ => "— / —".to_string(),
        },
    }
}

// « Cours de langue » : `None` when the program carries no requirement at
// all. `column` names where the francophone branch's own course sits when
// the student actually placed it — the non-francophone branch is not
// consulted, matching the task's own rule for `detail`.
fn build_language(
    program: &Program,
    box_position: &BTreeMap<String, (usize, usize)>,
) -> Option<LanguageBox> {
    let requirement = program.language_requirement.as_ref()?;
    let sigle = &requirement.francophone.course;
    let column = box_position
        .get(sigle)
        .map(|&(column_index, _)| column_index);
    Some(LanguageBox {
        label: "Cours de langue".to_string(),
        detail: format!("voir description du programme ({sigle})"),
        column,
    })
}

// The « N.B. Vous devez faire … » sentence, built only from `total_slots`
// (the option-box count this module just computed) and whether a language
// box exists — never a number invented independently of them. No slots and
// no language requirement means nothing to say, so the band list stays
// empty rather than announcing zero of nothing.
fn build_bands(
    total_slots: usize,
    language: &Option<LanguageBox>,
) -> Vec<String> {
    if total_slots == 0 && language.is_none() {
        return Vec::new();
    }
    let mut sentence =
        format!("N.B. Vous devez faire {total_slots} cours option au total");
    if language.is_some() {
        sentence
            .push_str(" et 1 cours de langue (voir description du programme)");
    }
    sentence.push('.');
    vec![sentence]
}

// One `SpanBox` per stage course the program's « Stages » rule lists, in
// the rule's own order — never from a hand-encoded position. A stage the
// plan never placed spans nothing and is skipped, named in a note instead.
// This function never touches `place_option_boxes`'s `remaining`: a
// stage's `credits_in_addition` credits count toward `session_credits`
// like any other placed course's (the same, single formula defines a
// column's used credits everywhere in this module), but building a span
// never spends an option slot's room — the two are separate code paths by
// construction.
fn build_spans(
    program: &Program,
    box_position: &BTreeMap<String, (usize, usize)>,
    notes: &mut Vec<String>,
) -> Vec<SpanBox> {
    let Some(stage_rule) = program
        .rules
        .iter()
        .find(|rule| rule.title == STAGES_RULE_TITLE)
    else {
        return Vec::new();
    };
    let RuleCourses::List { courses } = &stage_rule.courses else {
        return Vec::new();
    };
    courses
        .iter()
        .enumerate()
        .filter_map(|(index, code)| {
            let Some(&(column, _)) = box_position.get(code) else {
                notes.push(format!(
                    "Le microprogramme de stage {} ({code}) n'est pas \
                     placé dans le plan — aucune bande n'est ajoutée.",
                    roman(index + 1)
                ));
                return None;
            };
            Some(SpanBox {
                label: format!("Microprogramme de stage {}", roman(index + 1)),
                first_column: column.saturating_sub(1),
                last_column: column,
            })
        })
        .collect()
}

// Stage counts are small in every real program (the promoted « Stages »
// rule caps at 8, ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`)
// — a lookup table covers every case that can occur; anything past it
// falls back to the plain number rather than growing a general roman-
// numeral algorithm for a case that never happens.
fn roman(index: usize) -> String {
    match index {
        1 => "I".to_string(),
        2 => "II".to_string(),
        3 => "III".to_string(),
        4 => "IV".to_string(),
        5 => "V".to_string(),
        6 => "VI".to_string(),
        7 => "VII".to_string(),
        8 => "VIII".to_string(),
        other => other.to_string(),
    }
}

// A whole-course `Prerequisites::Raw` (the source text fell entirely
// outside the grammar) is folded into the same single-operand shape as a
// nested `PrereqTree::Raw` — one code path surfaces both instead of the
// top-level case being silently skipped.
fn operands_of(prerequisites: Option<&Prerequisites>) -> Vec<Operand> {
    match prerequisites {
        None => Vec::new(),
        Some(Prerequisites::Raw { raw }) => vec![Operand::Raw(raw.clone())],
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
            PrereqTree::Raw { raw } => {
                operands.push(Operand::Raw(raw.clone()))
            }
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

// Transcribed from `gex_organigramme.pdf`'s « Note 1 » : exit token right
// of the source box, entry tokens left of the box that requires it, shaded
// = concomitant, numeric = program credits required. Data, not markup — the
// render component (task 5) only lays these sentences out.
fn legend_text() -> Vec<String> {
    vec![
        "Un jeton lettré (a, b, c…) identifie un cours : il apparaît à \
         droite de la case du cours qui sert de préalable (jeton de \
         sortie) et à gauche de la case de chaque cours qui l'exige \
         (jeton d'entrée)."
            .to_string(),
        "Un cours exigeant un ou plusieurs préalables, ou un nombre de \
         crédits déjà réussis, porte un ou plusieurs jetons à l'entrée, à \
         gauche de sa case."
            .to_string(),
        "Un jeton ombré représente un cours concomitant : il peut se \
         faire avant ou en même temps que le cours pour lequel on le \
         retrouve à l'entrée."
            .to_string(),
        "Un jeton numérique (par exemple 24, 30, 45, 60 ou 90) indique un \
         nombre de crédits de programme exigés comme préalable."
            .to_string(),
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
        organigramme_document(&snapshot, &plan, None, "2026-08-25T14:00:00Z")
    }

    fn find<'a>(
        document: &'a OrganigrammeDocument,
        code: &str,
    ) -> &'a CourseBox {
        document
            .columns
            .iter()
            .flat_map(|column| &column.boxes)
            .find(|course_box| course_box.code == code)
            .unwrap_or_else(|| panic!("{code} not in the document"))
    }

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
    fn a_raw_operand_is_a_note_with_no_token() {
        let document = document();
        let course_box = find(&document, "GEX-5000");
        assert!(course_box.entry.is_empty());
        assert!(
            document.notes.iter().any(|note| note.contains("GEX-5000")
                && note.contains("Examen quelconque")),
            "{:?}",
            document.notes
        );
    }

    #[test]
    fn a_whole_course_raw_prerequisite_is_also_a_note_with_no_token() {
        // `Prerequisites::Raw` — the whole source text fell outside the
        // grammar, distinct from a nested `PrereqTree::Raw` operand inside
        // an otherwise-parsed tree (covered above by GEX-5000)
        let document = document();
        let course_box = find(&document, "GEX-8000");
        assert!(course_box.entry.is_empty());
        assert!(
            document.notes.iter().any(|note| note.contains("GEX-8000")
                && note.contains("Autorisation de la direction")),
            "{:?}",
            document.notes
        );
    }

    #[test]
    fn an_any_operand_is_drawn_like_an_all_operand() {
        // the document draws every operand of an `Any`, the same as an
        // `All` — it does not evaluate which branch the boolean picks
        let document = document();
        let course_box = find(&document, "GEX-9000");
        assert_eq!(
            course_box.entry,
            vec![Token::Letter {
                letter: "a".to_string(),
                shaded: false
            }],
            "GEX-1000, already lettered, is the operand drawn"
        );
        assert!(
            document
                .notes
                .iter()
                .any(|note| note.contains("GEX-7000")
                    && note.contains("GEX-9000")),
            "GEX-7000, off the document, is named too: {:?}",
            document.notes
        );
    }

    #[test]
    fn a_prerequisite_off_the_document_is_named_and_earns_no_token() {
        let document = document();
        let course_box = find(&document, "GEX-6000");
        assert!(course_box.entry.is_empty());
        assert!(
            document.notes.iter().any(|note| note.contains("GEX-7000")
                && note.contains("GEX-6000")
                && note.contains("n'est dans aucune session du document")),
            "{:?}",
            document.notes
        );
        // GEX-7000 is a real course, just never placed: it earns no exit
        // token anywhere in the document
        assert!(!document
            .columns
            .iter()
            .flat_map(|column| &column.boxes)
            .any(|course_box| course_box.code == "GEX-7000"));
    }

    #[test]
    fn an_unknown_code_falls_back_to_itself_with_a_note() {
        let document = document();
        let course_box = find(&document, "ZZZ-9999");
        assert_eq!(course_box.title, "ZZZ-9999");
        assert_eq!(course_box.credits, "");
        assert!(
            document.notes.iter().any(|note| note.contains("ZZZ-9999")
                && note.contains("absent du catalogue")),
            "{:?}",
            document.notes
        );
    }

    #[test]
    fn more_than_26_lettered_sources_roll_over_to_double_letters() {
        let mut courses = String::from(r#"{"courses":["#);
        for index in 0..27 {
            courses.push_str(&format!(
                r#"{{"code":"SRC-{index:03}","title":"Source","credits":1,
                    "cycle":1,"prerequisites":null,"equivalents":[],
                    "seasons":{{}}}},"#
            ));
        }
        let all_sources: Vec<String> =
            (0..27).map(|index| format!("\"SRC-{index:03}\"")).collect();
        courses.push_str(&format!(
            r#"{{"code":"DEP-9000","title":"Dépendant","credits":1,"cycle":1,
                "prerequisites":{{"raw":"toutes les sources",
                  "tree":{{"all":[{}]}}}},
                "equivalents":[],"seasons":{{}}}}"#,
            all_sources.join(",")
        ));
        courses.push_str("]}");

        let snapshot = snapshot_with(&courses);
        let owned_codes: Vec<String> =
            (0..27).map(|index| format!("SRC-{index:03}")).collect();
        let mut assignments: Vec<&str> =
            owned_codes.iter().map(String::as_str).collect();
        assignments.push("DEP-9000");
        let plan = plan_with(1, &[(1, &assignments)]);

        let document = organigramme_document(
            &snapshot,
            &plan,
            None,
            "2026-08-25T14:00:00Z",
        );
        // SRC-000..SRC-025 (26 sources) get the single letters a..z, in
        // that exact order; SRC-026, the 27th, rolls over to "aa" — an
        // exit token is never shaded, whatever the dependent's session
        assert_eq!(
            find(&document, "SRC-000").exit,
            Some(Token::Letter {
                letter: "a".to_string(),
                shaded: false
            })
        );
        assert_eq!(
            find(&document, "SRC-025").exit,
            Some(Token::Letter {
                letter: "z".to_string(),
                shaded: false
            }),
            "the 26th source is still a single letter"
        );
        assert_eq!(
            find(&document, "SRC-026").exit,
            Some(Token::Letter {
                letter: "aa".to_string(),
                shaded: false
            }),
            "the 27th source rolls over to a double letter"
        );
        let dependent = find(&document, "DEP-9000");
        let letters: Vec<String> = dependent
            .entry
            .iter()
            .map(|token| match token {
                Token::Letter { letter, .. } => letter.clone(),
                Token::Credits { .. } => panic!("no numeric token expected"),
            })
            .collect();
        assert_eq!(letters[0], "a");
        assert_eq!(letters[25], "z");
        assert_eq!(letters[26], "aa", "the 27th source rolls over");
    }

    #[test]
    fn letters_are_assigned_in_column_then_row_order_and_stay_stable() {
        let first = document();
        let second = document();
        assert_eq!(first, second, "no hidden non-determinism (no HashMap)");
    }

    #[test]
    fn an_empty_plan_yields_empty_columns_without_panicking() {
        let snapshot = snapshot_with(COURSES);
        let plan = Plan {
            study_sessions: 0,
            ..Plan::default()
        };
        let document = organigramme_document(
            &snapshot,
            &plan,
            None,
            "2026-08-25T14:00:00Z",
        );
        assert!(document.columns.is_empty());
        assert!(document.notes.is_empty());
    }

    #[test]
    fn a_populated_session_with_no_prerequisites_needs_no_tokens_or_notes() {
        let snapshot = snapshot_with(COURSES);
        let plan = plan_with(1, &[(1, &["GEX-1000"])]);
        let document = organigramme_document(
            &snapshot,
            &plan,
            None,
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(document.columns.len(), 1);
        let course_box = &document.columns[0].boxes[0];
        assert!(course_box.entry.is_empty());
        assert!(course_box.exit.is_none());
        assert!(document.notes.is_empty());
    }

    #[test]
    fn header_reads_the_program_when_given_and_falls_back_otherwise() {
        let snapshot = snapshot_with(COURSES);
        let mut plan = Plan::default();
        let document = organigramme_document(
            &snapshot,
            &plan,
            None,
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(document.title, "Organigramme des cours");
        assert_eq!(document.program_title, "Programme non choisi");
        assert_eq!(document.version, "Version inconnue");

        plan.program = Some(crate::state::ProgramChoice {
            code: "B-GEX".to_string(),
            semester: "A26".to_string(),
            concentration: None,
            profile: None,
        });
        let document = organigramme_document(
            &snapshot,
            &plan,
            None,
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(
            document.program_title, "B-GEX",
            "no Program given: the plan's own code is the fallback"
        );
        assert_eq!(document.version, "Version A26");

        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26",
                "title":"Génie des eaux","cycle":1,"credits_required":120,
                "mandatory":[],"rules":[],"concentrations":[],
                "profiles":[],"notes":["Note du programme"]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let document = organigramme_document(
            &snapshot,
            &plan,
            Some(&program),
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(document.program_title, "Génie des eaux");
        assert_eq!(
            document.notes[0], "Note du programme",
            "the program's own notes lead, untouched"
        );
    }

    #[test]
    fn the_legend_names_every_token_rule_from_note_1() {
        let legend = legend_text();
        assert_eq!(legend.len(), 4);
        let joined = legend.join(" ");
        assert!(joined.contains("jeton de sortie"));
        assert!(joined.contains("jeton d'entrée"));
        assert!(joined.contains("ombré") && joined.contains("concomitant"));
        assert!(joined.contains("numérique"));
    }

    #[test]
    fn token_letter_covers_the_single_and_double_letter_ranges() {
        assert_eq!(token_letter(0), "a");
        assert_eq!(token_letter(25), "z");
        assert_eq!(token_letter(26), "aa");
        assert_eq!(token_letter(27), "ab");
    }

    // --- « cours option », microprogrammes de stage, langue, tableau ------

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

    // `study_sessions: 2` (Fall, Winter) expands to 3 real columns —
    // `horizon_sessions` always pulls a Summer in right after a Winter — so
    // this yields exactly the 3 columns below, credit_cap 6: col0
    // (GEX-1000, 3cr) has room for one slot; col1 (GEX-2000+GEX-3000+
    // ANL-2020, 9cr) is over capacity — no room; col2 (STG-1000, 3cr) has
    // room for one slot too. ANL-2020 sits in col1 so `language.column`
    // resolves; STG-1000 sits alone in col2 so its span reaches back into
    // col1.
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
        organigramme_document(
            &option_snapshot(),
            &option_plan(),
            Some(&option_program()),
            "2026-08-25T14:00:00Z",
        )
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
    fn a_rule_missing_two_courses_places_two_slots_and_skips_a_full_column() {
        let document = option_document();
        assert_eq!(document.columns[0].options.len(), 1);
        assert!(document.columns[0].options[0]
            .rules
            .iter()
            .any(|rule| rule.title == "Règle 1"));

        assert!(
            document.columns[1].options.is_empty(),
            "col1 is over capacity: it must be skipped, {:?}",
            document.columns[1].options
        );

        let col2_titles: Vec<&str> = document.columns[2].options[0]
            .rules
            .iter()
            .map(|rule| rule.title.as_str())
            .collect();
        assert!(
            col2_titles.contains(&"Règle 1"),
            "Règle 1's second slot spills into the next column with room: \
             {col2_titles:?}"
        );
    }

    #[test]
    fn preparatory_done_false_never_extends_the_selection() {
        // the same document as `option_document`, just with the checkbox
        // off — nothing in `OPTION_PROGRAM` names a préparatoire rule, so
        // the outcome must be identical either way; this only exists to
        // exercise the `false` branch of `if plan.preparatory_done`
        let plan = Plan {
            preparatory_done: false,
            ..option_plan()
        };
        let document = organigramme_document(
            &option_snapshot(),
            &plan,
            Some(&option_program()),
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(document.bands, option_document().bands);
    }

    #[test]
    fn build_option_rule_hides_choices_for_every_non_list_shape() {
        // `Reference` and `Raw` share `Keyword`'s match arm (an or-pattern),
        // but each pattern is its own region — exercised directly since the
        // full-document scenario only ever reaches the `Keyword` one
        let incomplete = RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Incomplete,
            counted: None,
            elsewhere: Vec::new(),
            missing: Some(Missing::Count { count: 1 }),
            candidates: None,
            raw: None,
        };

        let reference_rule: Rule = serde_json::from_str(
            r#"{"title":"R","courses":{"concentration":"Géo",
                "rule":"Règle 1"},"raw":"tous les cours de la Règle 1"}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let option_rule = build_option_rule(&incomplete, &reference_rule);
        assert!(option_rule.choices.is_empty());
        assert_eq!(
            option_rule.raw.as_deref(),
            Some("tous les cours de la Règle 1")
        );

        let raw_rule: Rule = serde_json::from_str(
            r#"{"title":"R","raw":"texte hors grammaire"}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let option_rule = build_option_rule(&incomplete, &raw_rule);
        assert!(option_rule.choices.is_empty());
        assert_eq!(option_rule.raw.as_deref(), Some("texte hors grammaire"));
    }

    #[test]
    fn a_slot_with_nowhere_to_go_lands_in_the_last_column_with_a_note() {
        let document = option_document();
        // by the time Règle 2/3/4 are placed every column is already full
        // (0 or negative room), so all three fall back to the last column
        let last_titles: Vec<&str> = document.columns[2].options[0]
            .rules
            .iter()
            .map(|rule| rule.title.as_str())
            .collect();
        assert!(last_titles.contains(&"Règle 2"), "{last_titles:?}");
        assert!(last_titles.contains(&"Règle 3"), "{last_titles:?}");
        assert!(last_titles.contains(&"Règle 4"), "{last_titles:?}");
        assert!(
            document
                .notes
                .iter()
                .any(|note| note.contains("n'a pas trouvé de place")),
            "{:?}",
            document.notes
        );
    }

    #[test]
    fn a_negotiated_rule_shows_its_raw_text_and_no_choices() {
        let document = option_document();
        let rule = find_option_rule(&document, "Règle 3");
        assert_eq!(rule.raw.as_deref(), Some("convenus avec la direction"));
        assert!(rule.choices.is_empty());
        assert_eq!(rule.constraint, "Règle 3 – 3 à 3 crédits");
    }

    #[test]
    fn a_constraint_less_rule_is_still_shown() {
        let document = option_document();
        let rule = find_option_rule(&document, "Règle 4");
        assert_eq!(rule.constraint, "(contrainte non chiffrée)");
        assert_eq!(
            rule.choices,
            vec!["OPT-D".to_string(), "OPT-E".to_string()]
        );
    }

    #[test]
    fn a_satisfied_rule_produces_no_option_box() {
        let document = option_document();
        assert!(document
            .columns
            .iter()
            .flat_map(|column| &column.options)
            .flat_map(|option_box| &option_box.rules)
            .all(|rule| rule.title != "Stages"));
    }

    #[test]
    fn the_language_box_names_the_francophone_course_and_its_column() {
        let document = option_document();
        let language = document
            .language
            .as_ref()
            .unwrap_or_else(|| panic!("expected a language box"));
        assert_eq!(language.label, "Cours de langue");
        assert!(language.detail.contains("ANL-2020"), "{}", language.detail);
        assert_eq!(language.column, Some(1));
    }

    #[test]
    fn a_program_without_a_language_requirement_yields_none() {
        let mut program = option_program();
        program.language_requirement = None;
        let document = organigramme_document(
            &option_snapshot(),
            &option_plan(),
            Some(&program),
            "2026-08-25T14:00:00Z",
        );
        assert_eq!(document.language, None);
    }

    #[test]
    fn the_language_box_has_no_column_when_the_course_is_not_placed() {
        let program = option_program();
        let language = build_language(&program, &BTreeMap::new())
            .unwrap_or_else(|| panic!("expected a language box"));
        assert_eq!(language.column, None);
    }

    #[test]
    fn stage_spans_come_from_the_placement_and_the_unplaced_one_is_skipped() {
        let document = option_document();
        assert_eq!(document.spans.len(), 1);
        let span = &document.spans[0];
        assert_eq!(span.label, "Microprogramme de stage I");
        assert_eq!(span.first_column, 1, "the session preceding STG-1000's");
        assert_eq!(span.last_column, 2, "STG-1000's own column");
        assert!(
            document
                .notes
                .iter()
                .any(|note| note.contains("stage II")
                    && note.contains("STG-2000")),
            "{:?}",
            document.notes
        );
    }

    #[test]
    fn build_spans_yields_nothing_without_a_stages_rule() {
        let program = option_program();
        let mut program = program;
        program.rules.retain(|rule| rule.title != "Stages");
        let mut notes = Vec::new();
        assert!(build_spans(&program, &BTreeMap::new(), &mut notes).is_empty());
        assert!(notes.is_empty());
    }

    #[test]
    fn build_spans_yields_nothing_when_the_stages_rule_is_not_a_list() {
        let program: Program = serde_json::from_str(
            r#"{"code":"P","slug":"p","semester":"A26","title":"P",
                "cycle":1,"credits_required":10,"mandatory":[],
                "concentrations":[],"profiles":[],
                "rules":[{"title":"Stages","courses":"negotiated",
                          "raw":"convenus avec la direction"}]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let mut notes = Vec::new();
        assert!(build_spans(&program, &BTreeMap::new(), &mut notes).is_empty());
    }

    #[test]
    fn the_rules_table_renders_course_and_credits_constraints() {
        let document = option_document();
        let by_title = |title: &str| -> &RulesRow {
            document
                .rules_table
                .iter()
                .find(|row| row.rule == title)
                .unwrap_or_else(|| panic!("{title} missing from the table"))
        };
        let rule_1 = by_title("Règle 1");
        assert_eq!(rule_1.constraint, "Règle 1 – 2 cours");
        assert_eq!(rule_1.chosen, "0 / 2");

        let rule_2 = by_title("Règle 2");
        assert_eq!(rule_2.constraint, "Règle 2 – 4 à 12 crédits");
        assert_eq!(rule_2.chosen, "0 / 4");

        let stages = by_title("Stages");
        assert_eq!(stages.constraint, "Stages – 1 à 2 cours");
        assert_eq!(
            stages.chosen, "1 / 1",
            "satisfied at the minimum: no Missing to read a lower k from"
        );
    }

    #[test]
    fn the_band_sums_every_computed_slot_plus_the_language_course() {
        let document = option_document();
        assert_eq!(
            document.bands,
            vec!["N.B. Vous devez faire 6 cours option au total et 1 cours \
                 de langue (voir description du programme)."
                .to_string()]
        );
    }

    #[test]
    fn build_bands_omits_the_language_clause_without_a_requirement() {
        assert_eq!(
            build_bands(3, &None),
            vec!["N.B. Vous devez faire 3 cours option au total.".to_string()]
        );
    }

    #[test]
    fn build_bands_says_nothing_with_no_slots_and_no_language() {
        assert_eq!(build_bands(0, &None), Vec::<String>::new());
    }

    #[test]
    fn a_coverage_error_degrades_to_a_note_never_a_panic() {
        let mut plan = option_plan();
        plan.program = Some(crate::state::ProgramChoice {
            code: "B-GEX".to_string(),
            semester: "A26".to_string(),
            concentration: Some("Inconnue".to_string()),
            profile: None,
        });
        let document = organigramme_document(
            &option_snapshot(),
            &plan,
            Some(&option_program()),
            "2026-08-25T14:00:00Z",
        );
        assert!(document.rules_table.is_empty());
        assert!(document.bands.is_empty());
        assert!(
            document.notes.iter().any(|note| note
                .contains("Impossible de calculer les règles restantes")),
            "{:?}",
            document.notes
        );
        // the columns and their courses still render (ERR-5): the counting
        // failure never blanks the whole document
        assert!(!document.columns.is_empty());
    }

    #[test]
    fn no_program_means_no_options_language_rules_table_or_bands() {
        let document = organigramme_document(
            &option_snapshot(),
            &option_plan(),
            None,
            "2026-08-25T14:00:00Z",
        );
        assert!(document
            .columns
            .iter()
            .all(|column| column.options.is_empty()));
        assert_eq!(document.language, None);
        assert!(document.rules_table.is_empty());
        assert!(document.bands.is_empty());
        assert!(document.spans.is_empty());
    }

    #[test]
    fn zero_study_sessions_still_reports_rules_but_places_no_options() {
        let plan = Plan {
            study_sessions: 0,
            ..option_plan()
        };
        let document = organigramme_document(
            &option_snapshot(),
            &plan,
            Some(&option_program()),
            "2026-08-25T14:00:00Z",
        );
        assert!(document.columns.is_empty());
        assert!(document.rules_table.iter().any(|row| row.rule == "Règle 1"));
        // `place_option_boxes` returns 0 immediately when there are no
        // columns to place into — the language clause still shows since
        // the requirement itself does not depend on any column existing
        assert_eq!(
            document.bands,
            vec!["N.B. Vous devez faire 0 cours option au total et 1 cours \
                 de langue (voir description du programme)."
                .to_string()]
        );
    }

    #[test]
    fn place_option_boxes_skips_a_rule_report_with_no_matching_rule() {
        let snapshot = option_snapshot();
        let plan = option_plan();
        let program = option_program();
        let mut columns = vec![Column {
            label: "A1-A26".to_string(),
            semester: "A1".to_string(),
            boxes: Vec::new(),
            options: Vec::new(),
        }];
        let report = CoverageReport {
            mandatory: Vec::new(),
            rules: vec![RuleReport {
                scope: Scope::Program,
                title: "Introuvable".to_string(),
                status: RuleStatus::Incomplete,
                counted: None,
                elsewhere: Vec::new(),
                missing: Some(Missing::Count { count: 1 }),
                candidates: None,
                raw: None,
            }],
            language_requirement: None,
        };
        let mut notes = Vec::new();
        let total_slots = place_option_boxes(
            &mut columns,
            &snapshot,
            &plan,
            &report,
            &program,
            None,
            None,
            &mut notes,
        );
        assert_eq!(total_slots, 0, "an unmatched rule contributes no slot");
        assert!(columns[0].options.is_empty());
    }

    #[test]
    fn find_rule_resolves_every_scope_and_defends_against_mismatches() {
        let program: Program = serde_json::from_str(
            r#"{"code":"P","slug":"p","semester":"A26","title":"P",
                "cycle":1,"credits_required":10,"mandatory":[],
                "rules":[{"title":"Règle P","courses":["A-1"]}],
                "concentrations":[{"title":"Géo","mandatory":[],
                  "rules":[{"title":"Règle C","courses":["A-1"]}]}],
                "profiles":[{"title":"Intl","mandatory":[],
                  "rules":[{"title":"Règle Pr","courses":["A-1"]}]}]}"#,
        )
        .unwrap_or_else(|error| panic!("{error}"));

        assert!(find_rule(&program, None, None, Scope::Program, "Règle P")
            .is_some());
        assert!(find_rule(&program, None, None, Scope::Program, "Absente")
            .is_none());

        assert!(find_rule(
            &program,
            Some("Géo"),
            None,
            Scope::Concentration,
            "Règle C"
        )
        .is_some());
        assert!(
            find_rule(&program, None, None, Scope::Concentration, "Règle C")
                .is_none(),
            "no concentration chosen"
        );
        assert!(find_rule(
            &program,
            Some("Absente"),
            None,
            Scope::Concentration,
            "Règle C"
        )
        .is_none());

        assert!(find_rule(
            &program,
            None,
            Some("Intl"),
            Scope::Profile,
            "Règle Pr"
        )
        .is_some());
        assert!(
            find_rule(&program, None, None, Scope::Profile, "Règle Pr")
                .is_none(),
            "no profile chosen"
        );
        assert!(find_rule(
            &program,
            None,
            Some("Absent"),
            Scope::Profile,
            "Règle Pr"
        )
        .is_none());
    }

    #[test]
    fn needed_slots_reads_missing_count_directly() {
        let report = RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Incomplete,
            counted: None,
            elsewhere: Vec::new(),
            missing: Some(Missing::Count { count: 3 }),
            candidates: None,
            raw: None,
        };
        assert_eq!(needed_slots(&report), 3);
    }

    #[test]
    fn needed_slots_rounds_missing_credits_up_to_a_whole_slot() {
        let missing = |credits: i64| RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Incomplete,
            counted: None,
            elsewhere: Vec::new(),
            missing: Some(Missing::Credits { credits }),
            candidates: None,
            raw: None,
        };
        assert_eq!(needed_slots(&missing(3)), 1, "an exact multiple");
        assert_eq!(needed_slots(&missing(4)), 2, "rounds up past one slot");
        assert_eq!(needed_slots(&missing(6)), 2, "an exact multiple of two");
        assert_eq!(needed_slots(&missing(7)), 3, "rounds up past two slots");
    }

    #[test]
    fn needed_slots_defaults_to_one_representative_slot_with_no_missing() {
        let report = RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Reported,
            counted: None,
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: Some("raw".to_string()),
        };
        assert_eq!(needed_slots(&report), 1);
    }

    #[test]
    fn chosen_label_falls_back_to_a_dash_when_nothing_was_resolved() {
        let report = RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Reported,
            counted: None,
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: Some("raw".to_string()),
        };
        assert_eq!(chosen_label(&report, None), "— / —");
    }

    #[test]
    fn roman_covers_the_stage_range_and_falls_back_past_it() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(2), "II");
        assert_eq!(roman(3), "III");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(5), "V");
        assert_eq!(roman(6), "VI");
        assert_eq!(roman(7), "VII");
        assert_eq!(roman(8), "VIII");
        assert_eq!(roman(9), "9", "past the lookup table: the plain number");
    }
}
