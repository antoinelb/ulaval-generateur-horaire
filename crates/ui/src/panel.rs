use std::collections::BTreeSet;

use ulaval_scheduler_core::{
    coverage_report, prerequisites_met, Constraint, CourseCycle, PrereqStatus,
    Program, Rule, RuleCourses, RuleStatus, Scope, Season,
};

use crate::data::Snapshot;
use crate::solve::{self, weekly_schedule};
use crate::state::{self, Plan};

// « matière » = the course-code prefix (plan § faits du domaine)
pub fn subject_of(code: &str) -> &str {
    code.split_once('-')
        .map(|(subject, _)| subject)
        .unwrap_or(code)
}

// What the left panel renders — built pure from the snapshot and the plan,
// so every badge, row and reason is testable without a browser.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelModel {
    // ERR-5: a coverage error degrades this region alone, named in French
    pub coverage_error: Option<String>,
    pub mandatory: Option<Section>,
    pub rules: Vec<Section>,
    // « Exigence linguistique - ANL-2020 ou VEPT ≥ 53 » (± ✓)
    pub language_note: Option<String>,
    // program prose no grammar covers — surfaced, never dropped
    pub notes: Vec<String>,
    // ententes that could not be applied — said, never dropped
    pub warnings: Vec<String>,
}

impl PanelModel {
    fn empty() -> Self {
        PanelModel {
            coverage_error: None,
            mandatory: None,
            rules: Vec::new(),
            language_note: None,
            notes: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    // stable expansion identity (persisted in View.expanded_rule)
    pub key: String,
    pub title: String,
    // « 1 parmi », « 3–9 cr », « 3 cr - en sus »
    pub constraint: Option<String>,
    pub badge: Badge,
    pub rows: Vec<Row>,
    // rule text outside the grammar — always displayed
    pub raw: Option<String>,
    pub notes: Vec<String>,
    // a « tous les cours » rule: the rows come from a catalogue browse
    pub free: bool,
    // the Obligatoires bar: (satisfied, total)
    pub progress: Option<(usize, usize)>,
}

// never colour alone: the badge text itself carries the state (INP-3)
#[derive(Debug, Clone, PartialEq)]
pub enum Badge {
    Ok(String),
    Partial(String),
    Missing(String),
    Neutral(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub code: String,
    pub title: String,
    pub credits: String,
    // « réussi ✓ », « placé en A3-A27 », « offert A-H », « préalables non
    // remplis », « absent du catalogue »
    pub sub: String,
    pub state: RowState,
    // presumptions the prerequisite verdict relied on — surfaced
    pub assumed: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Placed,
    // préparatoire course covered by the checked « déjà faite » box:
    // counted ✓ by hypothesis, never actionable — no +, no chips
    Acquired,
    // in `electives`, waiting for the solver to place it (jalon 9's
    // « cours voulus »)
    Chosen,
    // takeable: the + (or the chips) applies
    Available,
    // dimmed, no action — jalon 6's filter
    PrereqUnmet,
    // no Course in the snapshot: named, never actionable
    Unknown,
}

pub fn panel_model(snapshot: &Snapshot, plan: &Plan) -> PanelModel {
    let Some(chosen) = chosen_program(snapshot, plan) else {
        return PanelModel::empty();
    };
    // the ententes ride as data: the rules gain their granted courses
    // before core counts anything
    let (granted, warnings) = granted_program(chosen, &plan.rule_grants);
    let program = &granted;
    let mut selection = selection(plan);
    // « préparatoire faite » : its courses count for the coverage without
    // occupying any session
    if plan.preparatory_done {
        selection.extend(crate::solve::preparatory_codes(program));
    }
    // `chosen_program` proved the choice exists; and_then keeps it total
    let concentration = plan
        .program
        .as_ref()
        .and_then(|choice| choice.concentration.as_deref());
    let profile = plan
        .program
        .as_ref()
        .and_then(|choice| choice.profile.as_deref());
    let report = match coverage_report(
        program,
        concentration,
        profile,
        &selection,
        &snapshot.courses,
    ) {
        Ok(report) => report,
        Err(error) => {
            // the counting failed, never the display: every section still
            // renders, badges neutral — the page must not go blank (ERR-5,
            // rapport étudiante 2026-08-13)
            return uncounted_panel(
                snapshot,
                plan,
                program,
                concentration,
                profile,
                coverage_error_message(&error),
                warnings,
            );
        }
    };
    let mandatory = mandatory_section(snapshot, plan, &report.mandatory);
    let mut rules: Vec<Section> = report
        .rules
        .iter()
        .map(|rule_report| {
            let rule = find_rule(
                program,
                concentration,
                profile,
                rule_report.scope,
                &rule_report.title,
            );
            rule_section(snapshot, plan, rule_report, rule)
        })
        .collect();
    preparatory_badge(&mut rules, plan);
    PanelModel {
        coverage_error: None,
        mandatory: Some(mandatory),
        rules,
        language_note: language_note(program, &report),
        notes: program.notes.clone(),
        warnings,
    }
}

// The préparatoire rule carries no constraint, so core reports it without
// a verdict — but the checkbox knows: checked is done, unchecked counts
// what remains (rapport étudiante : un badge « — » immuable ne dit rien).
fn preparatory_badge(rules: &mut [Section], plan: &Plan) {
    let key = format!("p/{}", ulaval_scheduler_core::PREPARATORY_RULE_TITLE);
    for section in rules {
        if section.key != key {
            continue;
        }
        section.badge = if plan.preparatory_done {
            Badge::Ok("✓ déjà faite".to_string())
        } else {
            let remaining = section
                .rows
                .iter()
                .filter(|row| row.state != RowState::Placed)
                .count();
            if remaining == 0 {
                Badge::Ok("✓".to_string())
            } else {
                Badge::Missing(format!("{remaining} à faire"))
            }
        };
    }
}

// the two reachable-by-clicking errors get real French with a way out;
// the rest (data defects) a generic French wrapper around the detail
fn coverage_error_message(
    error: &ulaval_scheduler_core::CoverageError,
) -> String {
    use ulaval_scheduler_core::CoverageError;
    match error {
        CoverageError::CreditsOverMax { rule, total, max } => format!(
            "{rule} : les cours sélectionnés y totalisent {total} crédits, \
             au-dessus de son maximum de {max}. Retirez-en un (ou déplacez \
             une entente) ; en attendant, les règles s'affichent sans \
             comptage."
        ),
        CoverageError::CountOverMax { rule, total, max } => format!(
            "{rule} : {total} cours sélectionnés y comptent, au-dessus de \
             son maximum de {max}. Retirez-en un (ou déplacez une entente) \
             ; en attendant, les règles s'affichent sans comptage."
        ),
        other => format!(
            "Les règles ne peuvent pas être comptées pour l'instant — \
             elles s'affichent sans comptage. Détail : {other}."
        ),
    }
}

// the whole panel, badges neutral: sections, rows and raw texts straight
// from the program, no verdict pretended
#[allow(clippy::too_many_arguments)]
fn uncounted_panel(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    message: String,
    warnings: Vec<String>,
) -> PanelModel {
    let mandatory_rows: Vec<Row> = program
        .mandatory
        .iter()
        .map(|code| row(snapshot, plan, code))
        .collect();
    let mandatory = Section {
        key: "obligatoires".to_string(),
        title: "Obligatoires".to_string(),
        constraint: None,
        badge: Badge::Neutral("—".to_string()),
        rows: mandatory_rows,
        raw: None,
        notes: Vec::new(),
        free: false,
        progress: None,
    };
    let mut rules: Vec<Section> = program
        .rules
        .iter()
        .map(|rule| bare_section(snapshot, plan, 'p', rule))
        .collect();
    if let Some(block) = program
        .concentrations
        .iter()
        .find(|block| Some(block.title.as_str()) == concentration)
    {
        rules.extend(
            block
                .rules
                .iter()
                .map(|rule| bare_section(snapshot, plan, 'c', rule)),
        );
    }
    if let Some(block) = program
        .profiles
        .iter()
        .find(|block| Some(block.title.as_str()) == profile)
    {
        rules.extend(
            block
                .rules
                .iter()
                .map(|rule| bare_section(snapshot, plan, 'f', rule)),
        );
    }
    preparatory_badge(&mut rules, plan);
    PanelModel {
        coverage_error: Some(message),
        mandatory: Some(mandatory),
        rules,
        language_note: language_parts(program),
        notes: program.notes.clone(),
        warnings,
    }
}

// one rule as a section without any counting — same rows, same raw texts
fn bare_section(
    snapshot: &Snapshot,
    plan: &Plan,
    scope_prefix: char,
    rule: &Rule,
) -> Section {
    let free = matches!(
        rule.courses,
        RuleCourses::Keyword {
            courses: ulaval_scheduler_core::Keyword::Any,
            ..
        }
    );
    let rows = match &rule.courses {
        RuleCourses::List { courses } => courses
            .iter()
            .map(|code| row(snapshot, plan, code))
            .collect(),
        _ => Vec::new(),
    };
    let raw = match &rule.courses {
        RuleCourses::Reference { raw, .. }
        | RuleCourses::Keyword { raw, .. }
        | RuleCourses::Raw { raw } => Some(raw.clone()),
        RuleCourses::List { .. } => None,
    };
    Section {
        key: format!("{scope_prefix}/{}", rule.title),
        title: rule.title.clone(),
        constraint: constraint_label(rule),
        badge: Badge::Neutral("—".to_string()),
        rows,
        raw,
        notes: rule.notes.clone(),
        free,
        progress: None,
    }
}

// The program as the direction's agreements amend it: each granted code
// joins its rule's course list — a « negotiated » rule (no fixed list)
// becomes the list of its grants. Pure data surgery; the counting stays
// core's. An inapplicable grant is named, never dropped.
pub fn granted_program(
    program: &Program,
    grants: &std::collections::BTreeMap<String, String>,
) -> (Program, Vec<String>) {
    let mut granted = program.clone();
    let mut warnings = Vec::new();
    for (code, key) in grants {
        let rule = grant_target(&mut granted, key);
        let Some(rule) = rule else {
            warnings.push(format!(
                "Entente pour {code} : la règle « {} » est introuvable dans \
                 ce programme — le cours n'y est pas compté.",
                key.split_once('/').map(|(_, title)| title).unwrap_or(key)
            ));
            continue;
        };
        let applied = match &mut rule.courses {
            RuleCourses::List { courses } => {
                if !courses.iter().any(|held| held == code) {
                    courses.push(code.clone());
                }
                true
            }
            RuleCourses::Keyword {
                courses: ulaval_scheduler_core::Keyword::Negotiated,
                ..
            } => {
                rule.courses = RuleCourses::List {
                    courses: vec![code.clone()],
                };
                true
            }
            // an « any » rule counts every course already; the other
            // shapes cannot host a list
            _ => {
                warnings.push(format!(
                    "Entente pour {code} : la règle « {} » n'accepte pas de \
                     liste de cours — le cours n'y est pas compté.",
                    rule.title
                ));
                false
            }
        };
        // an entente MOVES the course: it must stop counting in any other
        // rule's list, or one course credits two rules at once (rapport
        // étudiante 2026-08-13)
        if applied {
            strip_from_other_lists(&mut granted, code, key);
        }
    }
    (granted, warnings)
}

fn strip_from_other_lists(program: &mut Program, code: &str, keep_key: &str) {
    fn strip(prefix: char, rules: &mut [Rule], code: &str, keep_key: &str) {
        for rule in rules {
            if format!("{prefix}/{}", rule.title) == keep_key {
                continue;
            }
            if let RuleCourses::List { courses } = &mut rule.courses {
                courses.retain(|held| held != code);
            }
        }
    }
    strip('p', &mut program.rules, code, keep_key);
    for block in &mut program.concentrations {
        strip('c', &mut block.rules, code, keep_key);
    }
    for block in &mut program.profiles {
        strip('f', &mut block.rules, code, keep_key);
    }
}

// the rule a section key (« p/Règle 2 », « c/… », « f/… ») names
fn grant_target<'a>(
    program: &'a mut Program,
    key: &str,
) -> Option<&'a mut Rule> {
    let (scope, title) = key.split_once('/')?;
    match scope {
        "p" => program.rules.iter_mut().find(|rule| rule.title == title),
        "c" => program
            .concentrations
            .iter_mut()
            .flat_map(|concentration| concentration.rules.iter_mut())
            .find(|rule| rule.title == title),
        "f" => program
            .profiles
            .iter_mut()
            .flat_map(|profile| profile.rules.iter_mut())
            .find(|rule| rule.title == title),
        _ => None,
    }
}

// the coverage selection: everything the student has laid out or chosen
pub fn selection(plan: &Plan) -> BTreeSet<String> {
    plan.displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .chain(plan.electives.iter())
        .cloned()
        .collect()
}

pub fn chosen_program<'a>(
    snapshot: &'a Snapshot,
    plan: &Plan,
) -> Option<&'a Program> {
    let choice = plan.program.as_ref()?;
    snapshot.programs.iter().find(|program| {
        program.code == choice.code
            && program.semester.to_string() == choice.semester
    })
}

// what the solver and the coverage must see: the chosen program with the
// direction's agreements applied (inapplicable ones surface in the panel)
pub fn effective_program(snapshot: &Snapshot, plan: &Plan) -> Option<Program> {
    chosen_program(snapshot, plan)
        .map(|program| granted_program(program, &plan.rule_grants).0)
}

// the rules an agreement can attach a course to — a plain list, or a
// « negotiated » rule waiting for exactly that; keyed like the sections
pub fn grantable_rules(program: &Program) -> Vec<(String, String)> {
    let grantable = |rule: &Rule| {
        // never the préparatoire: attaching a course there would make it
        // « acquis » the moment the checkbox is on — no entente means that
        rule.title != ulaval_scheduler_core::PREPARATORY_RULE_TITLE
            && matches!(
                rule.courses,
                RuleCourses::List { .. }
                    | RuleCourses::Keyword {
                        courses: ulaval_scheduler_core::Keyword::Negotiated,
                        ..
                    }
            )
    };
    let keyed = |prefix: char, rule: &Rule| {
        (format!("{prefix}/{}", rule.title), rule.title.clone())
    };
    program
        .rules
        .iter()
        .filter(|rule| grantable(rule))
        .map(|rule| keyed('p', rule))
        .chain(
            program
                .concentrations
                .iter()
                .flat_map(|concentration| &concentration.rules)
                .filter(|rule| grantable(rule))
                .map(|rule| keyed('c', rule)),
        )
        .chain(
            program
                .profiles
                .iter()
                .flat_map(|profile| &profile.rules)
                .filter(|rule| grantable(rule))
                .map(|rule| keyed('f', rule)),
        )
        .collect()
}

fn mandatory_section(
    snapshot: &Snapshot,
    plan: &Plan,
    mandatory: &[ulaval_scheduler_core::MandatoryReport],
) -> Section {
    let satisfied: Vec<&String> = mandatory
        .iter()
        .flat_map(|scope| &scope.satisfied)
        .collect();
    let missing: Vec<&String> =
        mandatory.iter().flat_map(|scope| &scope.missing).collect();
    let total = satisfied.len() + missing.len();
    let rows = satisfied
        .iter()
        .chain(missing.iter())
        .map(|code| row(snapshot, plan, code))
        .collect();
    Section {
        key: "obligatoires".to_string(),
        title: "Obligatoires".to_string(),
        constraint: None,
        badge: if missing.is_empty() && total > 0 {
            Badge::Ok(format!("{}/{total}", satisfied.len()))
        } else if satisfied.is_empty() {
            Badge::Missing(format!("0/{total}"))
        } else {
            Badge::Partial(format!("{}/{total}", satisfied.len()))
        },
        rows,
        raw: None,
        notes: Vec::new(),
        free: false,
        progress: Some((satisfied.len(), total)),
    }
}

fn rule_section(
    snapshot: &Snapshot,
    plan: &Plan,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
) -> Section {
    let free = matches!(
        rule.map(|rule| &rule.courses),
        Some(RuleCourses::Keyword {
            courses: ulaval_scheduler_core::Keyword::Any,
            ..
        })
    );
    let rows = match rule.map(|rule| &rule.courses) {
        Some(RuleCourses::List { courses }) => courses
            .iter()
            .map(|code| row(snapshot, plan, code))
            .collect(),
        // a free rule browses the catalogue (search + matière); the other
        // shapes show their raw text below
        _ => Vec::new(),
    };
    let scope_prefix = match report.scope {
        Scope::Program => "p",
        Scope::Concentration => "c",
        Scope::Profile => "f",
    };
    Section {
        key: format!("{scope_prefix}/{}", report.title),
        title: report.title.clone(),
        constraint: rule.and_then(constraint_label),
        badge: rule_badge(snapshot, report, rule),
        rows,
        raw: report.raw.clone(),
        notes: rule.map(|rule| rule.notes.clone()).unwrap_or_default(),
        free,
        progress: None,
    }
}

// « 1 parmi », « 3–9 cr », suffixed « - en sus » when the credits do not
// count toward the diploma (the promoted Stages rule)
fn constraint_label(rule: &Rule) -> Option<String> {
    let label = match rule.constraint {
        None => None,
        Some(Constraint::Course { min, max }) if min == max => {
            Some(format!("{min} parmi"))
        }
        Some(Constraint::Course { min, max }) => {
            Some(format!("{min}–{max} parmi"))
        }
        Some(Constraint::Credits { min, max }) if min == max => {
            Some(format!("{min} cr"))
        }
        Some(Constraint::Credits { min, max }) => {
            Some(format!("{min}–{max} cr"))
        }
    };
    match (label, rule.credits_in_addition) {
        (Some(label), true) => Some(format!("{label} - en sus")),
        (label, _) => label,
    }
}

fn rule_badge(
    snapshot: &Snapshot,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
) -> Badge {
    match report.status {
        RuleStatus::Satisfied => match report.counted.as_deref() {
            Some([only]) => Badge::Ok(format!("✓ {only}")),
            _ => Badge::Ok("✓".to_string()),
        },
        RuleStatus::Reported => Badge::Neutral("—".to_string()),
        RuleStatus::Incomplete => incomplete_badge(snapshot, report, rule),
    }
}

// « 0/3 » (credits counted / minimum) or « 1/2 » (courses) — the design's
// wording, computed from what the report already counted
fn incomplete_badge(
    snapshot: &Snapshot,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
) -> Badge {
    let counted = report.counted.as_deref().unwrap_or_default();
    match rule.and_then(|rule| rule.constraint.as_ref()) {
        Some(Constraint::Course { min, .. }) => {
            let badge = format!("{}/{min}", counted.len());
            if counted.is_empty() {
                Badge::Missing(badge)
            } else {
                Badge::Partial(badge)
            }
        }
        Some(Constraint::Credits { min, .. }) => {
            let sum: u32 = counted
                .iter()
                .filter_map(|code| snapshot.by_code.get(code))
                .map(|&index| snapshot.courses[index].credits.planning())
                .sum();
            let badge = format!("{sum}/{min} cr");
            if sum == 0 {
                Badge::Missing(badge)
            } else {
                Badge::Partial(badge)
            }
        }
        // no constraint to count against: name what remains if given
        None => match report.missing {
            Some(ulaval_scheduler_core::Missing::Count { count }) => {
                Badge::Missing(format!("{count} à combler"))
            }
            Some(ulaval_scheduler_core::Missing::Credits { credits }) => {
                Badge::Missing(format!("{credits} cr à combler"))
            }
            None => Badge::Neutral("—".to_string()),
        },
    }
}

fn find_rule<'a>(
    program: &'a Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    scope: Scope,
    title: &str,
) -> Option<&'a Rule> {
    let rules = match scope {
        Scope::Program => &program.rules,
        Scope::Concentration => {
            &program
                .concentrations
                .iter()
                .find(|c| Some(c.title.as_str()) == concentration)?
                .rules
        }
        Scope::Profile => {
            &program
                .profiles
                .iter()
                .find(|p| Some(p.title.as_str()) == profile)?
                .rules
        }
    };
    rules.iter().find(|rule| rule.title == title)
}

fn language_note(
    program: &Program,
    report: &ulaval_scheduler_core::CoverageReport,
) -> Option<String> {
    let line = language_parts(program)?;
    let satisfied = matches!(
        report.language_requirement,
        Some(ulaval_scheduler_core::LanguageReport {
            status: ulaval_scheduler_core::LanguageStatus::Satisfied,
        })
    );
    Some(if satisfied {
        format!("{line} ✓")
    } else {
        line
    })
}

// the requirement line alone — what the uncounted fallback can still say
fn language_parts(program: &Program) -> Option<String> {
    let requirement = program.language_requirement.as_ref()?;
    let branch = &requirement.francophone;
    let mut parts = vec![branch.course.clone()];
    parts.extend(
        branch
            .tests
            .iter()
            .map(|test| format!("{} ≥ {}", test.name, test.score)),
    );
    Some(format!("Exigence linguistique - {}", parts.join(" ou ")))
}

// --- one course row --------------------------------------------------------

pub fn row(snapshot: &Snapshot, plan: &Plan, code: &str) -> Row {
    let mut row = base_row(snapshot, plan, code);
    // a hand-entered course says so wherever it appears (ADR
    // `2026-07-contribution-de-cours-manuels` : le flag est affiché)
    if snapshot.manual_codes.contains(code) {
        row.sub = format!("{} - manuel", row.sub);
    }
    // so does a course a direction agreement attached to a rule
    if plan.rule_grants.contains_key(code) {
        row.sub = format!("{} - entente", row.sub);
    }
    row
}

fn base_row(snapshot: &Snapshot, plan: &Plan, code: &str) -> Row {
    let Some(&index) = snapshot.by_code.get(code) else {
        return Row {
            code: code.to_string(),
            title: code.to_string(),
            credits: String::new(),
            sub: "absent du catalogue".to_string(),
            state: RowState::Unknown,
            assumed: Vec::new(),
        };
    };
    let course = &snapshot.courses[index];
    let title = course.title.clone();
    let credits = crate::present::credits_label(&course.credits);
    // the source text, extracted for every course: the Unmet row names
    // *which* prerequisites (rapport étudiante), and only a Parsed tree
    // can be Unmet anyway
    let prerequisites_source = match &course.prerequisites {
        Some(ulaval_scheduler_core::Prerequisites::Parsed { raw, .. }) => {
            format!(" ({raw})")
        }
        _ => String::new(),
    };
    // checked before Placed: a leftover placement (the healing effect is
    // about to purge it) must never be re-offered meanwhile
    if crate::solve::acquired_preparatory(snapshot, plan).contains(code) {
        return Row {
            code: code.to_string(),
            title,
            credits,
            sub: "considéré comme déjà fait - décochez la case pour le \
                  placer"
                .to_string(),
            state: RowState::Acquired,
            assumed: Vec::new(),
        };
    }
    if let Some(&session) = plan.displayed_placement.get(code) {
        return Row {
            code: code.to_string(),
            title,
            credits,
            sub: format!("placé en {}", placed_label(plan, session)),
            state: RowState::Placed,
            assumed: Vec::new(),
        };
    }
    // a course added by hand to a session is placed there too — without
    // this the row would offer to add it a second time
    let by_hand = plan
        .manual
        .iter()
        .find(|(_, codes)| codes.iter().any(|held| held == code))
        .map(|(&session, _)| session);
    if let Some(session) = by_hand {
        return Row {
            code: code.to_string(),
            title,
            credits,
            sub: format!("ajouté en {}", placed_label(plan, session)),
            state: RowState::Placed,
            assumed: Vec::new(),
        };
    }
    if plan.electives.iter().any(|chosen| chosen == code) {
        return Row {
            code: code.to_string(),
            title,
            credits,
            sub: "choisi - à placer par le solveur".to_string(),
            state: RowState::Chosen,
            assumed: Vec::new(),
        };
    }
    let (held, held_credits) = acquired(snapshot, plan);
    match prerequisites_met(course, &held, held_credits) {
        Ok(PrereqStatus::Met { assumed }) => Row {
            code: code.to_string(),
            title,
            credits,
            sub: format!("offert {}", offered_letters(course)),
            state: RowState::Available,
            assumed: assumed.into_iter().collect(),
        },
        Ok(PrereqStatus::Unmet) => Row {
            code: code.to_string(),
            title,
            credits,
            sub: format!("préalables non remplis{prerequisites_source}"),
            state: RowState::PrereqUnmet,
            assumed: Vec::new(),
        },
        // an unreadable tree is surfaced on the row, never hidden
        Err(error) => Row {
            code: code.to_string(),
            title,
            credits,
            sub: format!("préalables illisibles : {error}"),
            state: RowState::PrereqUnmet,
            assumed: Vec::new(),
        },
    }
}

// what the student holds: everything laid on the organigramme, with the
// planning credits of those courses (the static prerequisite question of
// `prerequisites_met`)
fn acquired(snapshot: &Snapshot, plan: &Plan) -> (BTreeSet<String>, u32) {
    let held: BTreeSet<String> = plan
        .displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .cloned()
        .collect();
    let credits = held
        .iter()
        .filter_map(|code| snapshot.by_code.get(code))
        .map(|&index| snapshot.courses[index].credits.planning())
        .sum();
    (held, credits)
}

fn placed_label(plan: &Plan, session: usize) -> String {
    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let semesters = state::session_semesters(plan.start, &seasons);
    state::session_label(&semesters, session.wrapping_sub(1))
}

// « offert A-H-É » — the seasons the snapshot carries for the course
fn offered_letters(course: &ulaval_scheduler_core::Course) -> String {
    let letters: Vec<&str> = course
        .seasons
        .keys()
        .map(|season| match season {
            Season::Fall => "A",
            Season::Winter => "H",
            Season::Summer => "É",
        })
        .collect();
    if letters.is_empty() {
        "—".to_string()
    } else {
        letters.join("-")
    }
}

// --- the search over courses ----------------------------------------------

pub const SEARCH_LIMIT: usize = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub rows: Vec<Row>,
    // full match count — truncation is announced, never silent
    pub matched: usize,
    pub masked_by_fit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchScope<'a> {
    // restrict to the courses offered in this session's season
    pub session: Option<usize>,
    // restrict to a matière (code prefix)
    pub subject: Option<&'a str>,
    // keep only first-cycle courses (the « cours libres » browse)
    pub first_cycle_only: bool,
    pub only_fitting: bool,
}

pub fn search_courses(
    snapshot: &Snapshot,
    plan: &Plan,
    scope: SearchScope,
    query: &str,
) -> SearchResults {
    let needle = query.trim().to_uppercase();
    let season = scope
        .session
        .and_then(|session| solve::session_semester(plan, session))
        .map(|semester| semester.season);
    // the probe is built once; each candidate then costs a mask overlap
    let fit = match (scope.only_fitting, scope.session) {
        (true, Some(session)) => fit_probe(snapshot, plan, session),
        _ => None,
    };
    let mut rows = Vec::new();
    let mut matched = 0usize;
    let mut masked_by_fit = 0usize;
    for course in &snapshot.courses {
        if let Some(season) = season {
            if !course.seasons.contains_key(&season) {
                continue;
            }
        }
        if let Some(subject) = scope.subject {
            if subject_of(&course.code) != subject {
                continue;
            }
        }
        if scope.first_cycle_only && course.cycle != CourseCycle::First {
            continue;
        }
        if !needle.is_empty()
            && !course.code.contains(&needle)
            && !course.title.to_uppercase().contains(&needle)
        {
            continue;
        }
        if let Some(probe) = &fit {
            if !matches!(
                quick_fit(probe, snapshot, course),
                Fit::Fits | Fit::AlreadyIn
            ) {
                masked_by_fit += 1;
                continue;
            }
        }
        matched += 1;
        if rows.len() < SEARCH_LIMIT {
            rows.push(row(snapshot, plan, &course.code));
        }
    }
    SearchResults {
        rows,
        matched,
        masked_by_fit,
    }
}

// the matières of the catalogue with their course counts, for the free
// rule's select — « matière = préfixe du code », plan § faits du domaine
pub fn subjects(snapshot: &Snapshot) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for course in &snapshot.courses {
        *counts
            .entry(subject_of(&course.code).to_string())
            .or_default() += 1;
    }
    counts.into_iter().collect()
}

// --- « rentrerait dans l'horaire » ----------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Fits,
    Conflicts,
    // offered, but nothing drawable (schedule unpublished…)
    NoSchedule,
    AlreadyIn,
}

// The current session's selections, folded into one mask: built once per
// search, then each candidate costs one `build_domain` and a mask overlap —
// core machinery end to end, thousands of rows stay under 16 ms (LAT-3).
// Swap semantics on purpose: the other courses hold their selected option,
// exactly what the grid shows.
#[derive(Debug, Clone, PartialEq)]
pub struct FitProbe {
    season: Season,
    fixed: ulaval_scheduler_core::WeekMask,
    already: Vec<String>,
}

pub fn fit_probe(
    snapshot: &Snapshot,
    plan: &Plan,
    session: usize,
) -> Option<FitProbe> {
    let season = solve::session_semester(plan, session)?.season;
    let schedule = weekly_schedule(snapshot, plan, session);
    let fixed = schedule
        .report
        .courses
        .iter()
        .flat_map(|course| &course.selected)
        .fold(ulaval_scheduler_core::WeekMask::EMPTY, |mask, section| {
            mask.merge(&ulaval_scheduler_core::slots_to_mask(&section.slots))
        });
    Some(FitProbe {
        season,
        fixed,
        already: state::session_codes(plan, session),
    })
}

pub fn quick_fit(
    probe: &FitProbe,
    snapshot: &Snapshot,
    course: &ulaval_scheduler_core::Course,
) -> Fit {
    if probe.already.iter().any(|code| code == &course.code) {
        return Fit::AlreadyIn;
    }
    let Some(offering) = season_offering(snapshot, course, probe.season)
    else {
        return Fit::NoSchedule;
    };
    if offering.options.is_none() {
        return Fit::NoSchedule;
    }
    let domain = ulaval_scheduler_core::build_domain(offering);
    if domain.is_empty() {
        return Fit::NoSchedule;
    }
    if domain.iter().any(|opt| !opt.mask.overlaps(&probe.fixed)) {
        Fit::Fits
    } else {
        Fit::Conflicts
    }
}

// the offering the weekly path would use: the course's own, out-voted by a
// more recent equivalent's vintage (same rule as `resolve_offering`)
fn season_offering<'a>(
    snapshot: &'a Snapshot,
    course: &'a ulaval_scheduler_core::Course,
    season: Season,
) -> Option<&'a ulaval_scheduler_core::SeasonOffering> {
    course
        .equivalents
        .iter()
        .filter_map(|code| snapshot.by_code.get(code))
        .map(|&index| &snapshot.courses[index])
        .fold(course.seasons.get(&season), |own, equivalent| {
            ulaval_scheduler_core::resolve_offering(
                own,
                equivalent.seasons.get(&season),
            )
        })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use crate::data::{parse_data, RawData};
    use crate::state::ProgramChoice;

    // GEX-1000 (fall, monday), GEX-2000 (fall, monday — clashes 1000),
    // GMN-1000 (fall, tuesday), GAE-1000 (winter only), GEX-9000 (needs an
    // unknown university code), MAT-0130 hidden préuniversitaire prereq on
    // GEX-3000, ANL-2020 second-cycle? no — first cycle language course
    const COURSES: &str = r#"{"courses":[
      {"code":"ANL-2020","title":"Intermediate English II","credits":3,
       "cycle":1,"prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":null}}},
      {"code":"GAE-1000","title":"Irrigation","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"winter":{"last_offered":2026,"options":null}}},
      {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"11111","section":"A","mode":"in-person","slots":[
            {"day":"monday","start":"08:30","end":"11:20"}]}]
       ]}}},
      {"code":"GEX-2000","title":"Hydraulique","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"22222","section":"A","mode":"in-person","slots":[
            {"day":"monday","start":"09:30","end":"12:20"}]}]
       ]}}},
      {"code":"GEX-3000","title":"Avec préparatoire","credits":3,"cycle":1,
       "prerequisites":{"raw":"MAT-0130","tree":"MAT-0130"},
       "equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":null}}},
      {"code":"GEX-9000","title":"Bloqué","credits":3,"cycle":1,
       "prerequisites":{"raw":"ZZZ-1111","tree":"ZZZ-1111"},
       "equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":null}}},
      {"code":"ETE-1000","title":"Cours d'été","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"summer":{"last_offered":2026,"options":null}}},
      {"code":"HOR-0000","title":"Hors saison","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],"seasons":{}},
      {"code":"VID-1000","title":"Options vides","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[]}}},
      {"code":"EQU-1000","title":"Par équivalence","credits":3,"cycle":1,
       "prerequisites":null,"equivalents":["GEX-1000"],
       "seasons":{"winter":{"last_offered":2020,"options":null}}},
      {"code":"GMN-1000","title":"Santé minière","credits":3,"cycle":2,
       "prerequisites":null,"equivalents":[],
       "seasons":{"fall":{"last_offered":2026,"options":[
         [{"nrc":"33333","section":"A","mode":"in-person","slots":[
            {"day":"tuesday","start":"08:30","end":"11:20"}]}]
       ]}}}
    ]}"#;

    const PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
      "title":"Baccalauréat en génie des eaux","cycle":1,
      "credits_required":120,
      "mandatory":["GEX-1000","GEX-2000"],
      "rules":[
        {"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},
         "courses":["GMN-1000"]},
        {"title":"Règle 2","constraint":{"type":"credits","min":6,"max":9},
         "courses":["GAE-1000","GEX-9000","GHOST-1"],
         "notes":["une note de règle"]},
        {"title":"Règle 3","constraint":{"type":"credits","min":3,"max":3},
         "courses":"any",
         "raw":"Tous les cours de premier cycle"},
        {"title":"Règle 4","raw":"des cours convenus"}
      ],
      "concentrations":[{"title":"Génie urbain","credits_required":null,
        "mandatory":[],
        "rules":[{"title":"Règle C1",
                  "constraint":{"type":"course","min":2,"max":2},
                  "courses":["GAE-1000","GMN-1000"]}],
        "notes":[]}],
      "profiles":[{"title":"Profil international","credits_required":null,
        "mandatory":[],
        "rules":[{"title":"Règle P1","courses":"negotiated",
                  "raw":"convenus avec la direction"}],
        "notes":[]}],
      "notes":["une note de programme"],
      "language_requirement":{"francophone":{"course":"ANL-2020",
        "tests":[{"name":"VEPT","score":53}],
        "raw":"Réussir ANL-2020 ou VEPT 53"}}}"#;

    fn snapshot() -> Snapshot {
        parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    PROGRAM.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    fn plan() -> Plan {
        Plan {
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            // GEX-1000 sits in a past session: that alone makes it acquired
            displayed_placement: std::collections::BTreeMap::from([
                ("GMN-1000".to_string(), 1),
                ("GEX-1000".to_string(), 2),
            ]),
            ..Plan::default()
        }
    }

    #[test]
    fn without_a_program_the_panel_is_empty_and_calm() {
        let model = panel_model(&snapshot(), &Plan::default());
        assert_eq!(model, PanelModel::empty());
    }

    #[test]
    fn the_model_counts_sections_badges_and_notes() {
        let model = panel_model(&snapshot(), &plan());
        assert!(model.coverage_error.is_none());
        let mandatory = model.mandatory.expect("a program was chosen");
        assert_eq!(mandatory.progress, Some((1, 2)));
        assert_eq!(mandatory.badge, Badge::Partial("1/2".to_string()));
        assert_eq!(mandatory.rows[0].state, RowState::Placed);
        assert_eq!(mandatory.rows[1].code, "GEX-2000");

        assert_eq!(model.rules.len(), 4);
        assert_eq!(
            model.rules[0].badge,
            Badge::Ok("✓ GMN-1000".to_string()),
            "the single counted course is named"
        );
        assert_eq!(model.rules[0].constraint.as_deref(), Some("1 parmi"));
        assert_eq!(model.rules[1].badge, Badge::Missing("0/6 cr".to_string()));
        assert_eq!(model.rules[1].notes, ["une note de règle"]);
        assert!(model.rules[2].free, "the any rule browses the catalogue");
        assert_eq!(
            model.rules[2].raw.as_deref(),
            Some("Tous les cours de premier cycle")
        );
        assert_eq!(model.rules[3].badge, Badge::Neutral("—".to_string()));
        assert_eq!(model.rules[3].raw.as_deref(), Some("des cours convenus"));
        assert_eq!(
            model.language_note.as_deref(),
            Some("Exigence linguistique - ANL-2020 ou VEPT ≥ 53")
        );
        assert_eq!(model.notes, ["une note de programme"]);

        // préparatoire unchecked: the selection gains nothing (this
        // program has no préparatoire rule — same sections either way)
        let mut unchecked = plan();
        unchecked.preparatory_done = false;
        assert_eq!(panel_model(&snapshot(), &unchecked).rules.len(), 4);
    }

    #[test]
    fn rule_rows_carry_their_states_and_reasons() {
        let model = panel_model(&snapshot(), &plan());
        let rows = &model.rules[1].rows;
        assert_eq!(rows[0].code, "GAE-1000");
        assert_eq!(rows[0].state, RowState::Available);
        assert_eq!(rows[0].sub, "offert H");
        assert_eq!(rows[1].code, "GEX-9000");
        assert_eq!(rows[1].state, RowState::PrereqUnmet);
        assert_eq!(rows[1].sub, "préalables non remplis (ZZZ-1111)");
        assert_eq!(rows[2].code, "GHOST-1");
        assert_eq!(rows[2].state, RowState::Unknown);
        assert_eq!(rows[2].sub, "absent du catalogue");
    }

    #[test]
    fn a_preuniversity_presumption_is_surfaced_on_the_row() {
        let with_prep = row(&snapshot(), &plan(), "GEX-3000");
        assert_eq!(with_prep.state, RowState::Available);
        assert_eq!(with_prep.assumed, ["MAT-0130"]);
    }

    #[test]
    fn a_placed_course_names_its_session() {
        let placed = row(&snapshot(), &plan(), "GMN-1000");
        assert_eq!(placed.state, RowState::Placed);
        assert_eq!(placed.sub, "placé en A1-A26");
    }

    #[test]
    fn an_unknown_concentration_degrades_the_counting_never_the_panel() {
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Aucune".to_string());
        }
        let model = panel_model(&snapshot(), &plan);
        let error = model.coverage_error.clone().expect("must be named");
        assert!(error.contains("Aucune"), "{error}");
        assert!(error.contains("sans comptage"), "{error}");
        // the sections still render, badges neutral — never a blank panel
        let mandatory = model.mandatory.expect("still shown");
        assert_eq!(mandatory.rows.len(), 2);
        assert_eq!(mandatory.badge, Badge::Neutral("—".to_string()));
        assert_eq!(model.rules.len(), 4, "the program rules still render");
        assert!(model
            .rules
            .iter()
            .all(|section| section.badge == Badge::Neutral("—".to_string())));
        assert!(model.rules[2].free, "the any rule still browses");
        assert_eq!(
            model.rules[3].raw.as_deref(),
            Some("des cours convenus"),
            "raw texts survive the fallback"
        );
        assert!(model
            .language_note
            .as_deref()
            .is_some_and(|note| note.starts_with("Exigence linguistique")));
    }

    #[test]
    fn an_overfilled_rule_speaks_french_and_keeps_the_sections() {
        // two courses counted in a « 1 parmi » rule: core refuses to count
        // (semantics await the director) — the message must be French and
        // actionable, the panel intact (rapport étudiante 2026-08-13)
        let snapshot = snapshot();
        let mut plan = plan();
        plan.displayed_placement.insert("GMN-1000".to_string(), 1);
        plan.rule_grants = std::collections::BTreeMap::from([(
            "GEX-1000".to_string(),
            "p/Règle 1".to_string(),
        )]);
        let model = panel_model(&snapshot, &plan);
        let error = model.coverage_error.expect("over-max must be named");
        assert!(error.contains("Règle 1"), "{error}");
        assert!(error.contains("au-dessus de son maximum de 1"), "{error}");
        assert!(error.contains("Retirez-en"), "{error}");
        assert!(model.mandatory.is_some(), "panel never blank");
        // the chosen concentration's rules join the fallback sections
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
            choice.profile = Some("Profil international".to_string());
        }
        let model = panel_model(&snapshot, &plan);
        let keys: Vec<&str> = model
            .rules
            .iter()
            .map(|section| section.key.as_str())
            .collect();
        assert!(keys.contains(&"c/Règle C1"), "{keys:?}");
        assert!(keys.contains(&"f/Règle P1"), "{keys:?}");
    }

    #[test]
    fn coverage_error_messages_speak_french_for_both_over_max_shapes() {
        let credits = coverage_error_message(
            &ulaval_scheduler_core::CoverageError::CreditsOverMax {
                rule: "Règle 2".to_string(),
                total: 12,
                max: 9,
            },
        );
        assert!(credits.contains("12 crédits"), "{credits}");
        assert!(credits.contains("maximum de 9"), "{credits}");
    }

    #[test]
    fn a_reference_rule_keeps_its_raw_text_in_the_uncounted_fallback() {
        let rule: Rule = serde_json::from_str(
            r#"{"title":"R","courses":{"concentration":"X","rule":"Règle 1"},
                "raw":"tous les cours de la Règle 1 du cheminement X"}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let section = bare_section(&snapshot(), &plan(), 'p', &rule);
        assert_eq!(
            section.raw.as_deref(),
            Some("tous les cours de la Règle 1 du cheminement X")
        );
        assert!(section.rows.is_empty());
    }

    #[test]
    fn a_grant_moves_the_course_out_of_its_original_rule() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        // GMN-1000 sits in Règle 1's list; the entente moves it to Règle 2
        let grants = std::collections::BTreeMap::from([(
            "GMN-1000".to_string(),
            "p/Règle 2".to_string(),
        )]);
        let (granted, warnings) = granted_program(program, &grants);
        assert!(warnings.is_empty(), "{warnings:?}");
        let rule1 = granted
            .rules
            .iter()
            .find(|rule| rule.title == "Règle 1")
            .expect("kept");
        assert_eq!(
            rule1.courses,
            RuleCourses::List {
                courses: Vec::new()
            },
            "one course must never credit two rules at once"
        );
        let rule2 = granted
            .rules
            .iter()
            .find(|rule| rule.title == "Règle 2")
            .expect("kept");
        assert!(matches!(
            &rule2.courses,
            RuleCourses::List { courses }
                if courses.contains(&"GMN-1000".to_string())
        ));
    }

    #[test]
    fn the_preparatory_badge_follows_the_checkbox() {
        let preparatory_program = r#"{"code":"B-GEX","slug":"gex",
            "semester":"A26","title":"P","cycle":1,"credits_required":6,
            "mandatory":[],
            "rules":[{"title":"Scolarité préparatoire",
                      "courses":["GEX-1000","GAE-1000"]}],
            "concentrations":[],"profiles":[]}"#;
        let snapshot = parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    preparatory_program.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = plan();
        let model = panel_model(&snapshot, &plan);
        assert_eq!(
            model.rules[0].badge,
            Badge::Ok("✓ déjà faite".to_string())
        );
        plan.preparatory_done = false;
        let model = panel_model(&snapshot, &plan);
        assert_eq!(
            model.rules[0].badge,
            Badge::Missing("1 à faire".to_string()),
            "GEX-1000 is placed, GAE-1000 remains"
        );
        plan.displayed_placement.insert("GAE-1000".to_string(), 2);
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.rules[0].badge, Badge::Ok("✓".to_string()));
    }

    #[test]
    fn checked_preparatory_rows_are_acquired_in_rules_and_search() {
        let preparatory_program = r#"{"code":"B-GEX","slug":"gex",
            "semester":"A26","title":"P","cycle":1,"credits_required":6,
            "mandatory":[],
            "rules":[{"title":"Scolarité préparatoire",
                      "courses":["GEX-1000","GAE-1000"]}],
            "concentrations":[],"profiles":[]}"#;
        let snapshot = parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    preparatory_program.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = plan();
        let model = panel_model(&snapshot, &plan);
        let rows = &model.rules[0].rows;
        assert!(
            rows.iter().all(|row| row.state == RowState::Acquired),
            "{rows:?}"
        );
        assert!(rows[0].sub.contains("décochez la case"), "{}", rows[0].sub);
        // the search offers the same course: same verdict, no side door
        let everywhere = SearchScope {
            session: None,
            subject: None,
            first_cycle_only: false,
            only_fitting: false,
        };
        let results =
            search_courses(&snapshot, &plan, everywhere, "hydrologie");
        assert_eq!(results.rows[0].code, "GEX-1000");
        assert_eq!(results.rows[0].state, RowState::Acquired);
        // an entente moved the course into another rule: ordinary again
        // (here it is placed in session 2 by the plan)
        plan.rule_grants
            .insert("GEX-1000".to_string(), "p/Règle 1".to_string());
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.rules[0].rows[0].state, RowState::Placed);
        plan.rule_grants.clear();
        // unchecked: ordinary work to place
        plan.preparatory_done = false;
        let model = panel_model(&snapshot, &plan);
        assert!(model.rules[0]
            .rows
            .iter()
            .all(|row| row.state != RowState::Acquired));
    }

    #[test]
    fn the_search_filters_by_query_season_subject_and_cycle() {
        let snapshot = snapshot();
        let plan = plan();
        let everywhere = SearchScope {
            session: None,
            subject: None,
            first_cycle_only: false,
            only_fitting: false,
        };
        let all = search_courses(&snapshot, &plan, everywhere, "");
        assert_eq!(all.matched, 11);

        let by_query = search_courses(&snapshot, &plan, everywhere, "hydro");
        assert_eq!(by_query.matched, 1);
        assert_eq!(by_query.rows[0].code, "GEX-1000");

        let fall_only = search_courses(
            &snapshot,
            &plan,
            SearchScope {
                session: Some(1),
                ..everywhere
            },
            "",
        );
        assert_eq!(fall_only.matched, 7, "GAE, ETE, HOR et EQU hors automne");

        let gex = search_courses(
            &snapshot,
            &plan,
            SearchScope {
                subject: Some("GEX"),
                ..everywhere
            },
            "",
        );
        assert_eq!(gex.matched, 4);

        let first_cycle = search_courses(
            &snapshot,
            &plan,
            SearchScope {
                first_cycle_only: true,
                ..everywhere
            },
            "",
        );
        assert_eq!(first_cycle.matched, 10, "GMN-1000 is second cycle");
    }

    #[test]
    fn the_fit_filter_masks_and_counts_what_it_hides() {
        let snapshot = snapshot();
        let mut plan = plan();
        plan.manual.insert(1, vec!["GEX-2000".to_string()]);
        let results = search_courses(
            &snapshot,
            &plan,
            SearchScope {
                session: Some(1),
                subject: Some("GEX"),
                first_cycle_only: false,
                only_fitting: true,
            },
            "",
        );
        // GEX-1000 clashes with the placed GEX-2000; GEX-3000/9000 have no
        // published schedule; GEX-2000 itself is AlreadyIn and stays
        assert_eq!(results.masked_by_fit, 3);
        assert_eq!(results.matched, 1);
        assert_eq!(results.rows[0].code, "GEX-2000");
    }

    #[test]
    fn the_quick_fit_answers_all_four_ways() {
        let snapshot = snapshot();
        let mut plan = Plan::default();
        plan.manual.insert(1, vec!["GEX-2000".to_string()]);
        let probe = fit_probe(&snapshot, &plan, 1).expect("session 1 exists");
        let course = |code: &str| &snapshot.courses[snapshot.by_code[code]];
        assert_eq!(
            quick_fit(&probe, &snapshot, course("GMN-1000")),
            Fit::Fits
        );
        assert_eq!(
            quick_fit(&probe, &snapshot, course("GEX-1000")),
            Fit::Conflicts
        );
        assert_eq!(
            quick_fit(&probe, &snapshot, course("GEX-3000")),
            Fit::NoSchedule,
            "an unpublished schedule cannot answer"
        );
        assert_eq!(
            quick_fit(&probe, &snapshot, course("GAE-1000")),
            Fit::NoSchedule,
            "not offered in this season"
        );
        assert_eq!(
            quick_fit(&probe, &snapshot, course("GEX-2000")),
            Fit::AlreadyIn
        );
        assert!(
            fit_probe(&snapshot, &plan, 99).is_none(),
            "outside the horizon there is nothing to probe"
        );
    }

    #[test]
    fn the_mandatory_badge_covers_all_done_and_nothing_done() {
        let snapshot = snapshot();
        let mut all_placed = plan();
        all_placed
            .displayed_placement
            .insert("GEX-2000".to_string(), 4);
        let model = panel_model(&snapshot, &all_placed);
        let mandatory = model.mandatory.expect("program chosen");
        assert_eq!(mandatory.badge, Badge::Ok("2/2".to_string()));

        let mut nothing = plan();
        nothing.displayed_placement.clear();
        let model = panel_model(&snapshot, &nothing);
        let mandatory = model.mandatory.expect("program chosen");
        assert_eq!(mandatory.badge, Badge::Missing("0/2".to_string()));
    }

    #[test]
    fn a_chosen_concentration_and_profile_bring_their_rules() {
        let snapshot = snapshot();
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
            choice.profile = Some("Profil international".to_string());
        }
        // GAE-1000 laid out too: the concentration rule counts 2/2
        plan.displayed_placement.insert("GAE-1000".to_string(), 2);
        let model = panel_model(&snapshot, &plan);
        let c1 = model
            .rules
            .iter()
            .find(|section| section.key == "c/Règle C1")
            .expect("the concentration rule is reported");
        assert_eq!(c1.badge, Badge::Ok("✓".to_string()));
        let p1 = model
            .rules
            .iter()
            .find(|section| section.key == "f/Règle P1")
            .expect("the profile rule is reported");
        assert_eq!(p1.badge, Badge::Neutral("—".to_string()));
        assert_eq!(p1.raw.as_deref(), Some("convenus avec la direction"));
    }

    #[test]
    fn a_partially_counted_rule_shows_its_progress() {
        let snapshot = snapshot();
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
        }
        // only GMN-1000 counted of the two: 1/2 courses
        let model = panel_model(&snapshot, &plan);
        let c1 = model
            .rules
            .iter()
            .find(|section| section.key == "c/Règle C1")
            .expect("reported");
        assert_eq!(c1.badge, Badge::Partial("1/2".to_string()));
        // Règle 2 (6–9 cr) with GAE-1000 laid out: 3/6 cr partial
        plan.displayed_placement.insert("GAE-1000".to_string(), 2);
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.rules[1].badge, Badge::Partial("3/6 cr".to_string()));
    }

    #[test]
    fn constraint_labels_speak_every_shape() {
        let rule = |constraint: &str, en_sus: bool| -> Rule {
            serde_json::from_str(&format!(
                r#"{{"title":"R","constraint":{constraint},
                     "courses":["X-1"],
                     "credits_in_addition":{en_sus}}}"#
            ))
            .unwrap_or_else(|e| panic!("rule literal: {e}"))
        };
        let label =
            |constraint, en_sus| constraint_label(&rule(constraint, en_sus));
        assert_eq!(
            label(r#"{"type":"course","min":1,"max":1}"#, false).as_deref(),
            Some("1 parmi")
        );
        assert_eq!(
            label(r#"{"type":"course","min":1,"max":3}"#, false).as_deref(),
            Some("1–3 parmi")
        );
        assert_eq!(
            label(r#"{"type":"credits","min":3,"max":3}"#, false).as_deref(),
            Some("3 cr")
        );
        assert_eq!(
            label(r#"{"type":"credits","min":1,"max":8}"#, true).as_deref(),
            Some("1–8 cr - en sus")
        );
        let bare: Rule =
            serde_json::from_str(r#"{"title":"R","courses":["X-1"]}"#)
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(constraint_label(&bare), None);
    }

    #[test]
    fn an_unconstrained_incomplete_rule_names_what_remains() {
        let report = |missing: Option<ulaval_scheduler_core::Missing>| {
            ulaval_scheduler_core::RuleReport {
                scope: Scope::Program,
                title: "R".to_string(),
                status: RuleStatus::Incomplete,
                counted: None,
                missing,
                candidates: None,
                raw: None,
            }
        };
        let snapshot = snapshot();
        assert_eq!(
            incomplete_badge(
                &snapshot,
                &report(Some(ulaval_scheduler_core::Missing::Count {
                    count: 2
                })),
                None
            ),
            Badge::Missing("2 à combler".to_string())
        );
        assert_eq!(
            incomplete_badge(
                &snapshot,
                &report(Some(ulaval_scheduler_core::Missing::Credits {
                    credits: 6
                })),
                None
            ),
            Badge::Missing("6 cr à combler".to_string())
        );
        assert_eq!(
            incomplete_badge(&snapshot, &report(None), None),
            Badge::Neutral("—".to_string())
        );
    }

    #[test]
    fn an_unreadable_prerequisite_tree_is_surfaced_on_the_row() {
        let mut snapshot = snapshot();
        let deep = (0..10_000).fold(
            ulaval_scheduler_core::PrereqTree::Course("X-1".to_string()),
            |child, _| ulaval_scheduler_core::PrereqTree::All {
                all: vec![child],
            },
        );
        let index = snapshot.by_code["GAE-1000"];
        snapshot.courses[index].prerequisites =
            Some(ulaval_scheduler_core::Prerequisites::Parsed {
                raw: "deep".to_string(),
                tree: deep,
            });
        let row = row(&snapshot, &Plan::default(), "GAE-1000");
        assert_eq!(row.state, RowState::PrereqUnmet);
        assert!(row.sub.contains("préalables illisibles"), "{}", row.sub);
    }

    #[test]
    fn offered_letters_cover_the_ete_and_the_seasonless_course() {
        let snapshot = snapshot();
        let summer = row(&snapshot, &Plan::default(), "ETE-1000");
        assert_eq!(summer.sub, "offert É");
        let never = row(&snapshot, &Plan::default(), "HOR-0000");
        assert_eq!(never.sub, "offert —");
    }

    #[test]
    fn the_quick_fit_sees_through_equivalents_and_empty_options() {
        let snapshot = snapshot();
        let plan = Plan::default();
        let probe = fit_probe(&snapshot, &plan, 1).expect("session 1");
        let course = |code: &str| &snapshot.courses[snapshot.by_code[code]];
        // EQU-1000 has no fall of its own; its equivalent GEX-1000 does,
        // with a newer vintage — the resolved offering answers
        assert_eq!(
            quick_fit(&probe, &snapshot, course("EQU-1000")),
            Fit::Fits
        );
        // an offering with zero valid combination cannot answer either
        assert_eq!(
            quick_fit(&probe, &snapshot, course("VID-1000")),
            Fit::NoSchedule
        );
    }

    #[test]
    fn a_search_beyond_the_limit_announces_its_truncation() {
        let mut entries: Vec<String> = Vec::new();
        for i in 0..40 {
            entries.push(format!(
                r#"{{"code":"TST-{i:04}","title":"Cours {i}","credits":3,
                     "cycle":1,"prerequisites":null,"equivalents":[],
                     "seasons":{{}}}}"#
            ));
        }
        let snapshot = crate::data::parse_data(
            &RawData {
                courses: format!(r#"{{"courses":[{}]}}"#, entries.join(",")),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                programs: Vec::new(),
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let results = search_courses(
            &snapshot,
            &Plan::default(),
            SearchScope {
                session: None,
                subject: None,
                first_cycle_only: false,
                only_fitting: false,
            },
            "",
        );
        assert_eq!(results.matched, 40);
        assert_eq!(results.rows.len(), SEARCH_LIMIT, "capped, and said so");
    }

    #[test]
    fn a_manual_course_is_marked_wherever_it_appears() {
        let mut snapshot = snapshot();
        let course = crate::data::build_manual_course(
            &crate::data::ManualDraft {
                code: "ZZZ-9000".to_string(),
                title: "Maison".to_string(),
                credits: "3".to_string(),
                nrc: String::new(),
                slots: Vec::new(),
            },
            Season::Fall,
            2026,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        crate::data::add_manual_course(&mut snapshot, course)
            .unwrap_or_else(|e| panic!("{e}"));
        let row = row(&snapshot, &Plan::default(), "ZZZ-9000");
        assert!(row.sub.ends_with("- manuel"), "{}", row.sub);
    }

    #[test]
    fn a_chosen_elective_says_it_waits_for_the_solver() {
        let mut plan = plan();
        plan.electives.push("GAE-1000".to_string());
        let chosen = row(&snapshot(), &plan, "GAE-1000");
        assert_eq!(chosen.state, RowState::Chosen);
        assert_eq!(chosen.sub, "choisi - à placer par le solveur");
    }

    #[test]
    fn find_rule_answers_none_for_a_scope_it_cannot_resolve() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        assert!(find_rule(
            program,
            Some("Inconnue"),
            None,
            Scope::Concentration,
            "Règle C1"
        )
        .is_none());
        assert!(find_rule(
            program,
            None,
            Some("Inconnu"),
            Scope::Profile,
            "Règle P1"
        )
        .is_none());
    }

    #[test]
    fn the_language_note_marks_satisfaction_and_abstains_without_one() {
        let snapshot = snapshot();
        let mut plan = plan();
        plan.displayed_placement.insert("ANL-2020".to_string(), 3);
        let model = panel_model(&snapshot, &plan);
        let note = model.language_note.expect("a requirement exists");
        assert!(note.ends_with("✓"), "{note}");

        let bare: Program = serde_json::from_str(
            r#"{"code":"X","slug":"x","semester":"A26","title":"X",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let report = ulaval_scheduler_core::CoverageReport {
            mandatory: Vec::new(),
            rules: Vec::new(),
            language_requirement: None,
        };
        assert!(language_note(&bare, &report).is_none());
    }

    #[test]
    fn a_grant_joins_its_rule_and_turns_a_negotiated_rule_into_a_list() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        let grants = std::collections::BTreeMap::from([
            // an existing List rule gains the course once
            ("XYZ-1000".to_string(), "p/Règle 1".to_string()),
            // already listed: not doubled
            ("GMN-1000".to_string(), "p/Règle 1".to_string()),
            // the negotiated profile rule becomes the list of its grants
            ("XYZ-2000".to_string(), "f/Règle P1".to_string()),
            // a concentration rule hosts a grant too
            ("XYZ-3000".to_string(), "c/Règle C1".to_string()),
        ]);
        let (granted, warnings) = granted_program(program, &grants);
        assert!(warnings.is_empty(), "{warnings:?}");
        let c1 = granted.concentrations[0]
            .rules
            .iter()
            .find(|rule| rule.title == "Règle C1")
            .expect("kept");
        assert!(matches!(
            &c1.courses,
            RuleCourses::List { courses } if courses.contains(&"XYZ-3000".to_string())
        ));
        let rule1 = granted
            .rules
            .iter()
            .find(|rule| rule.title == "Règle 1")
            .expect("kept");
        assert_eq!(
            rule1.courses,
            RuleCourses::List {
                courses: vec!["GMN-1000".to_string(), "XYZ-1000".to_string()]
            }
        );
        let p1 = granted.profiles[0]
            .rules
            .iter()
            .find(|rule| rule.title == "Règle P1")
            .expect("kept");
        assert_eq!(
            p1.courses,
            RuleCourses::List {
                courses: vec!["XYZ-2000".to_string()]
            }
        );
        // and the coverage now counts the granted course
        let mut plan = plan();
        plan.rule_grants = std::collections::BTreeMap::from([(
            "GEX-1000".to_string(),
            "p/Règle 2".to_string(),
        )]);
        let model = panel_model(&snapshot, &plan);
        assert_eq!(
            model.rules[1].badge,
            Badge::Partial("3/6 cr".to_string()),
            "GEX-1000 (placé) compte maintenant dans la Règle 2"
        );
        let entente = &model.rules[1].rows[3];
        assert_eq!(entente.code, "GEX-1000");
        assert!(entente.sub.ends_with("- entente"), "{}", entente.sub);
    }

    #[test]
    fn an_inapplicable_grant_is_named_never_dropped() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        let grants = std::collections::BTreeMap::from([
            ("AAA-1000".to_string(), "p/Règle fantôme".to_string()),
            ("BBB-1000".to_string(), "sans-slash".to_string()),
            // Règle 3 is « any »: every course counts already
            ("CCC-1000".to_string(), "p/Règle 3".to_string()),
            ("DDD-1000".to_string(), "x/Règle 1".to_string()),
        ]);
        let (granted, warnings) = granted_program(program, &grants);
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings[0].contains("Règle fantôme"), "{}", warnings[0]);
        assert!(warnings[1].contains("sans-slash"), "{}", warnings[1]);
        assert!(warnings[2].contains("Règle 3"), "{}", warnings[2]);
        assert!(warnings[3].contains("Règle 1"), "{}", warnings[3]);
        assert_eq!(&granted.rules, &snapshot.programs[0].rules, "untouched");
        // the warnings reach the panel
        let mut plan = plan();
        plan.rule_grants = grants;
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.warnings.len(), 4);
    }

    #[test]
    fn the_effective_program_is_the_chosen_one_with_its_grants_applied() {
        let snapshot = snapshot();
        let mut plan = plan();
        plan.rule_grants = std::collections::BTreeMap::from([(
            "XYZ-1000".to_string(),
            "p/Règle 1".to_string(),
        )]);
        let effective =
            effective_program(&snapshot, &plan).expect("a program is chosen");
        let rule1 = effective
            .rules
            .iter()
            .find(|rule| rule.title == "Règle 1")
            .expect("kept");
        assert!(matches!(
            &rule1.courses,
            RuleCourses::List { courses }
                if courses.contains(&"XYZ-1000".to_string())
        ));
        assert!(
            effective_program(&snapshot, &Plan::default()).is_none(),
            "no program chosen, nothing effective"
        );
    }

    #[test]
    fn grantable_rules_offer_lists_and_negotiated_rules_across_scopes() {
        let snapshot = snapshot();
        let rules = grantable_rules(&snapshot.programs[0]);
        let keys: Vec<&str> =
            rules.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            ["p/Règle 1", "p/Règle 2", "c/Règle C1", "f/Règle P1"],
            "« any » (Règle 3) and raw (Règle 4) shapes stay out"
        );
        assert_eq!(rules[3].1, "Règle P1");

        // the préparatoire rule is never an entente target: attaching a
        // course there would make it « acquis » with the checkbox
        let with_preparatory: Program = serde_json::from_str(
            r#"{"code":"X","slug":"x","semester":"A26","title":"X",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[{"title":"Scolarité préparatoire",
                          "courses":["MAT-0130"]}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(grantable_rules(&with_preparatory).is_empty());
    }

    #[test]
    fn subjects_count_the_catalogue_by_prefix() {
        let subjects = subjects(&snapshot());
        assert!(subjects.contains(&("GEX".to_string(), 4)));
        assert!(subjects.contains(&("ANL".to_string(), 1)));
        assert_eq!(subject_of("SANS-TIRET"), "SANS");
        assert_eq!(subject_of("BRUT"), "BRUT");
    }
}
