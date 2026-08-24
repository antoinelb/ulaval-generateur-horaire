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
    pub groups: Vec<PanelGroup>,
    pub preparatory: Option<Section>,
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
            groups: Vec::new(),
            preparatory: None,
            language_note: None,
            notes: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PanelGroup {
    pub title: String,
    pub progress: Option<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    // stable expansion identity (persisted in View.expanded_rule)
    pub key: String,
    pub title: String,
    pub badge: Badge,
    // the expanded rule's first line: what to pick here and that nothing
    // is ever taken automatically (rapport étudiante-cegep 2026-08-19)
    pub lead: Option<String>,
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
    // credited by an agreement with the direction: counted, never placed —
    // no +, no chips either, but the toggle that undoes it stays
    Credited,
    // in `electives`, waiting for the solver to place it (jalon 9's
    // « cours voulus »)
    Chosen,
    // takeable: the + (or the chips) applies
    Available,
    // dimmed, no action — jalon 6's filter
    PrereqUnmet,
    // no Course in the snapshot: named, never actionable
    Unknown,
    // already counted by an earlier rule of the same scope (`report.
    // elsewhere`, decision d'Antoine 2026-08-23): shown selected, no
    // choice strip — an entente is still the only way to move it
    CountedElsewhere,
}

pub fn panel_model(snapshot: &Snapshot, plan: &Plan) -> PanelModel {
    let Some(chosen) = chosen_program(snapshot, plan) else {
        return PanelModel::empty();
    };
    // `chosen_program` proved the choice exists; and_then keeps it total
    let concentration = plan
        .program
        .as_ref()
        .and_then(|choice| choice.concentration.as_deref());
    let profile = plan
        .program
        .as_ref()
        .and_then(|choice| choice.profile.as_deref());
    // the ententes ride as data: the rules gain their granted courses
    // before core counts anything
    let (granted, mut warnings) =
        granted_program(chosen, concentration, profile, &plan.rule_grants);
    let program = &granted;
    let mut selection = selection(plan);
    // « préparatoire faite » : its courses count for the coverage without
    // occupying any session
    if plan.preparatory_done {
        selection.extend(crate::solve::preparatory_codes(program));
    }
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
                chosen,
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
            // the rule as the program wrote it: a granted « tous les
            // cours » rule became a list, but its browse and its text
            // belong to the section still
            let original = find_rule(
                chosen,
                concentration,
                profile,
                rule_report.scope,
                &rule_report.title,
            );
            rule_section(
                snapshot,
                plan,
                program,
                rule_report,
                rule,
                original,
                &report.rules,
            )
        })
        .collect();
    preparatory_badge(&mut rules, plan);
    let (groups, preparatory) = grouped_sections(
        snapshot,
        plan,
        program,
        concentration,
        profile,
        &report,
        &rules,
    );
    warnings.extend(unlisted_credited(plan, Some(&mandatory), &rules));
    PanelModel {
        coverage_error: None,
        mandatory: Some(mandatory),
        rules,
        groups,
        preparatory,
        language_note: language_note(program, &report),
        notes: program.notes.clone(),
        warnings,
    }
}

// A credited course must stay visible: it shows up in the rule that lists
// it (Obligatoires, Règle 2, … — choix d'Antoine 2026-08-17). One no
// section lists — a « tous les cours » rule browses instead of listing, a
// course outside the program — would add credits from nowhere, so it is
// named rather than left silent.
fn unlisted_credited(
    plan: &Plan,
    mandatory: Option<&Section>,
    rules: &[Section],
) -> Vec<String> {
    let shown: BTreeSet<&str> = mandatory
        .into_iter()
        .chain(rules)
        .flat_map(|section| section.rows.iter())
        .map(|row| row.code.as_str())
        .collect();
    plan.credited
        .iter()
        .filter(|code| !shown.contains(code.as_str()))
        .map(|code| {
            format!(
                "{code} est crédité mais n'apparaît dans aucune règle de ce \
                 programme — rattachez-le à une règle pour l'y voir compté."
            )
        })
        .collect()
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
        CoverageError::CreditsOverMax {
            rule,
            scope,
            total,
            max,
        } => {
            let origin = scope_origin(*scope);
            format!(
                "{rule}{origin} : les cours sélectionnés y totalisent \
                 {total} crédits, au-dessus de son maximum de {max}. \
                 Retirez-en un (ou déplacez une entente); en attendant, \
                 les règles s'affichent sans comptage."
            )
        }
        CoverageError::CountOverMax {
            rule,
            scope,
            total,
            max,
        } => {
            let origin = scope_origin(*scope);
            format!(
                "{rule}{origin} : {total} cours sélectionnés y comptent, \
                 au-dessus de son maximum de {max}. Retirez-en un (ou \
                 déplacez une entente); en attendant, les règles \
                 s'affichent sans comptage."
            )
        }
        other => format!(
            "Les règles ne peuvent pas être comptées pour l'instant — \
             elles s'affichent sans comptage. Détail : {other}."
        ),
    }
}

// the whole panel, badges neutral: sections, rows and raw texts straight
// from the program, no verdict pretended — `chosen` is the program before
// the agreements, whose keyword rules keep their browse and raw text
#[allow(clippy::too_many_arguments)]
fn uncounted_panel(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    chosen: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    message: String,
    mut warnings: Vec<String>,
) -> PanelModel {
    let mandatory_rows: Vec<Row> =
        unique_rows(snapshot, plan, program.mandatory.iter());
    let mandatory = Section {
        key: "obligatoires".to_string(),
        title: "Obligatoires".to_string(),
        badge: Badge::Neutral("—".to_string()),
        lead: None,
        rows: mandatory_rows,
        raw: None,
        notes: Vec::new(),
        free: false,
        progress: None,
    };
    let original = |scope: Scope, title: &str| {
        find_rule(chosen, concentration, profile, scope, title)
    };
    let mut rules: Vec<Section> = program
        .rules
        .iter()
        .map(|rule| {
            let source = original(Scope::Program, &rule.title);
            bare_section(snapshot, plan, program, 'p', rule, source)
        })
        .collect();
    if let Some(block) = program
        .concentrations
        .iter()
        .find(|block| Some(block.title.as_str()) == concentration)
    {
        rules.extend(block.rules.iter().map(|rule| {
            let source = original(Scope::Concentration, &rule.title);
            bare_section(snapshot, plan, program, 'c', rule, source)
        }));
    }
    if let Some(block) = program
        .profiles
        .iter()
        .find(|block| Some(block.title.as_str()) == profile)
    {
        rules.extend(block.rules.iter().map(|rule| {
            let source = original(Scope::Profile, &rule.title);
            bare_section(snapshot, plan, program, 'f', rule, source)
        }));
    }
    preparatory_badge(&mut rules, plan);
    let (groups, preparatory) = uncounted_groups(
        snapshot,
        plan,
        program,
        concentration,
        profile,
        &rules,
    );
    warnings.extend(unlisted_credited(plan, Some(&mandatory), &rules));
    PanelModel {
        coverage_error: Some(message),
        mandatory: Some(mandatory),
        rules,
        groups,
        preparatory,
        language_note: language_parts(program),
        notes: program.notes.clone(),
        warnings,
    }
}

fn uncounted_groups(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    rules: &[Section],
) -> (Vec<PanelGroup>, Option<Section>) {
    let preparatory = rules
        .iter()
        .find(|section| {
            section.title == ulaval_scheduler_core::PREPARATORY_RULE_TITLE
        })
        .cloned();
    let mut groups = vec![uncounted_scope_group(
        snapshot,
        plan,
        "Programme".to_string(),
        &program.mandatory,
        "p/",
        None,
        rules,
    )];
    if let Some(block) = program
        .concentrations
        .iter()
        .find(|block| Some(block.title.as_str()) == concentration)
    {
        groups.push(uncounted_scope_group(
            snapshot,
            plan,
            format!("Concentration — {}", block.title),
            &block.mandatory,
            "c/",
            block.credits_required,
            rules,
        ));
    }
    if let Some(block) = program
        .profiles
        .iter()
        .find(|block| Some(block.title.as_str()) == profile)
    {
        groups.push(uncounted_scope_group(
            snapshot,
            plan,
            format!("Profil — {}", block.title),
            &block.mandatory,
            "f/",
            block.credits_required,
            rules,
        ));
    }
    (groups, preparatory)
}

#[allow(clippy::too_many_arguments)]
fn uncounted_scope_group(
    snapshot: &Snapshot,
    plan: &Plan,
    title: String,
    mandatory: &[String],
    prefix: &str,
    credits_required: Option<i64>,
    rules: &[Section],
) -> PanelGroup {
    let mandatory_section = if mandatory.is_empty() {
        None
    } else {
        Some(Section {
            key: format!("{prefix}obligatoires"),
            title: "Cours obligatoires".to_string(),
            badge: Badge::Neutral("—".to_string()),
            lead: None,
            rows: unique_rows(snapshot, plan, mandatory.iter()),
            raw: None,
            notes: Vec::new(),
            free: false,
            progress: None,
        })
    };
    let sections = mandatory_section
        .into_iter()
        .chain(
            rules
                .iter()
                .filter(|section| section.key.starts_with(prefix))
                .filter(|section| {
                    section.title
                        != ulaval_scheduler_core::PREPARATORY_RULE_TITLE
                })
                .cloned(),
        )
        .collect();
    PanelGroup {
        title,
        progress: credits_required.map(|required| {
            format!("—/{required} cr — progression indisponible")
        }),
        sections,
    }
}

fn grouped_sections(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    report: &ulaval_scheduler_core::CoverageReport,
    rules: &[Section],
) -> (Vec<PanelGroup>, Option<Section>) {
    let preparatory_key =
        format!("p/{}", ulaval_scheduler_core::PREPARATORY_RULE_TITLE);
    let preparatory = rules
        .iter()
        .find(|section| section.key == preparatory_key)
        .cloned();
    let mut groups = Vec::new();
    groups.push(scope_group(
        snapshot,
        plan,
        report,
        rules,
        Scope::Program,
        "Programme".to_string(),
        "p/",
        None,
    ));
    if let Some(block) = program
        .concentrations
        .iter()
        .find(|block| Some(block.title.as_str()) == concentration)
    {
        groups.push(scope_group(
            snapshot,
            plan,
            report,
            rules,
            Scope::Concentration,
            format!("Concentration — {}", block.title),
            "c/",
            block.credits_required,
        ));
    }
    if let Some(block) = program
        .profiles
        .iter()
        .find(|block| Some(block.title.as_str()) == profile)
    {
        groups.push(scope_group(
            snapshot,
            plan,
            report,
            rules,
            Scope::Profile,
            format!("Profil — {}", block.title),
            "f/",
            block.credits_required,
        ));
    }
    (groups, preparatory)
}

#[allow(clippy::too_many_arguments)]
fn scope_group(
    snapshot: &Snapshot,
    plan: &Plan,
    report: &ulaval_scheduler_core::CoverageReport,
    rules: &[Section],
    scope: Scope,
    title: String,
    prefix: &str,
    credits_required: Option<i64>,
) -> PanelGroup {
    let mandatory = report
        .mandatory
        .iter()
        .find(|mandatory| mandatory.scope == scope)
        .and_then(|mandatory| {
            scoped_mandatory_section(snapshot, plan, scope, mandatory)
        });
    let sections = mandatory
        .into_iter()
        .chain(
            rules
                .iter()
                .filter(|section| section.key.starts_with(prefix))
                .filter(|section| {
                    section.title
                        != ulaval_scheduler_core::PREPARATORY_RULE_TITLE
                })
                .cloned(),
        )
        .collect();
    PanelGroup {
        title,
        progress: credits_required
            .map(|required| scope_progress(snapshot, report, scope, required)),
        sections,
    }
}

fn scoped_mandatory_section(
    snapshot: &Snapshot,
    plan: &Plan,
    scope: Scope,
    mandatory: &ulaval_scheduler_core::MandatoryReport,
) -> Option<Section> {
    let total = mandatory.satisfied.len() + mandatory.missing.len();
    if total == 0 {
        return None;
    }
    let rows = unique_rows(
        snapshot,
        plan,
        mandatory.satisfied.iter().chain(&mandatory.missing),
    );
    let satisfied = mandatory.satisfied.len();
    let prefix = match scope {
        Scope::Program => "p",
        Scope::Concentration => "c",
        Scope::Profile => "f",
    };
    Some(Section {
        key: format!("{prefix}/obligatoires"),
        title: "Cours obligatoires".to_string(),
        badge: if satisfied == total {
            Badge::Ok(format!("{satisfied}/{total}"))
        } else if satisfied == 0 {
            Badge::Missing(format!("0/{total}"))
        } else {
            Badge::Partial(format!("{satisfied}/{total}"))
        },
        lead: None,
        rows,
        raw: None,
        notes: Vec::new(),
        free: false,
        progress: Some((satisfied, total)),
    })
}

fn scope_progress(
    snapshot: &Snapshot,
    report: &ulaval_scheduler_core::CoverageReport,
    scope: Scope,
    required: i64,
) -> String {
    let codes: BTreeSet<&str> = report
        .mandatory
        .iter()
        .filter(|mandatory| mandatory.scope == scope)
        .flat_map(|mandatory| mandatory.satisfied.iter().map(String::as_str))
        .chain(
            report
                .rules
                .iter()
                .filter(|rule| rule.scope == scope)
                .flat_map(|rule| rule.counted.iter().flatten())
                .map(String::as_str),
        )
        .collect();
    let missing = codes
        .iter()
        .find(|code| !snapshot.by_code.contains_key(**code));
    if let Some(code) = missing {
        return format!("—/{required} cr — crédits inconnus pour {code}");
    }
    let earned: i64 = codes
        .iter()
        .filter_map(|code| snapshot.by_code.get(*code))
        .map(|&index| i64::from(snapshot.courses[index].credits.planning()))
        .sum();
    format!("{}/{required} cr", earned.min(required))
}

// one rule as a section without any counting — same rows, same raw texts;
// `original` is the ungranted rule, whose browse and raw text survive a
// grant's Keyword → List transformation
fn bare_section(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    scope_prefix: char,
    rule: &Rule,
    original: Option<&Rule>,
) -> Section {
    let rows = ulaval_scheduler_core::resolved_rule_courses(program, rule)
        .ok()
        .flatten()
        .map(|courses| unique_rows(snapshot, plan, courses.iter()))
        .unwrap_or_default();
    Section {
        key: format!("{scope_prefix}/{}", rule.title),
        title: rule.title.clone(),
        // aucun comptage ici (ADR 2026-08-verdicts-honnetes-et-panneau-
        // jamais-vide) : le badge dit ce que la règle exige, sans inventer
        // de numérateur
        badge: Badge::Neutral(
            constraint_label(rule).unwrap_or_else(|| "—".to_string()),
        ),
        lead: None,
        rows,
        raw: rule_raw(rule).or_else(|| original.and_then(rule_raw)),
        notes: rule.notes.clone(),
        free: browses_catalogue(original.unwrap_or(rule)),
        progress: None,
    }
}

// a « tous les cours » rule: its rows come from a catalogue browse
fn browses_catalogue(rule: &Rule) -> bool {
    matches!(
        rule.courses,
        RuleCourses::Keyword {
            courses: ulaval_scheduler_core::Keyword::Any,
            ..
        }
    )
}

// the rule text outside the grammar, whatever shape carries it
fn rule_raw(rule: &Rule) -> Option<String> {
    match &rule.courses {
        RuleCourses::Reference { raw, .. }
        | RuleCourses::Keyword { raw, .. }
        | RuleCourses::Raw { raw } => Some(raw.clone()),
        RuleCourses::List { .. } => None,
    }
}

// The program as the direction's agreements amend it: each granted code
// joins its rule's course list — a « negotiated » rule (no fixed list)
// becomes the list of its grants. Pure data surgery; the counting stays
// core's. An inapplicable grant is named, never dropped. A `c/…`/`f/…`
// key resolves inside the *chosen* block only: every concentration of the
// B-GMC has a « Règle 1 », and an entente must never land in another
// block's rule of the same name (décision 2026-08-19).
pub fn granted_program(
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    grants: &std::collections::BTreeMap<String, String>,
) -> (Program, Vec<String>) {
    let mut granted = program.clone();
    let mut warnings = Vec::new();
    for (code, key) in grants {
        let label = grant_label(&granted, concentration, profile, key);
        let rule = grant_target(&mut granted, concentration, profile, key);
        let Some(rule) = rule else {
            warnings.push(format!(
                "Entente pour {code} : la règle « {} » est introuvable dans \
                 ce programme — le cours n'y est pas compté.",
                label
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
            // a keyword rule (« negotiated », « tous les cours ») is never
            // counted by core: the grant is the explicit attachment that
            // turns it into a countable list (ADR
            // `2026-08-entente-vers-une-regle-any`)
            RuleCourses::Keyword {
                courses:
                    ulaval_scheduler_core::Keyword::Any
                    | ulaval_scheduler_core::Keyword::Negotiated,
                ..
            } => {
                rule.courses = RuleCourses::List {
                    courses: vec![code.clone()],
                };
                true
            }
            // the remaining shapes (raw, reference) cannot host a list
            _ => {
                warnings.push(format!(
                    "Entente pour {code} : la règle « {} » n'accepte pas de \
                     liste de cours — le cours n'y est pas compté.",
                    label
                ));
                false
            }
        };
        // an entente MOVES the course within its own scope: it must stop
        // counting in the other rules of that same scope, or one course
        // credits two rules of the same scope at once (rapport étudiante
        // 2026-08-13) — scopes count independently, so a course entendu in
        // the concentration keeps counting in the profile too (ADR
        // `2026-08-un-cours-compte-dans-une-seule-regle-par-portee`)
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
    match keep_key.split_once('/').map(|(prefix, _)| prefix) {
        Some("c") => {
            for block in &mut program.concentrations {
                strip('c', &mut block.rules, code, keep_key);
            }
        }
        Some("f") => {
            for block in &mut program.profiles {
                strip('f', &mut block.rules, code, keep_key);
            }
        }
        // programme scope: `strip_from_other_lists` is only called after a
        // successful `grant_target`, so the key is well-formed here
        _ => strip('p', &mut program.rules, code, keep_key),
    }
}

// the rule a section key (« p/Règle 2 », « c/… », « f/… ») names — the
// scoped keys inside the chosen block only
fn grant_target<'a>(
    program: &'a mut Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    key: &str,
) -> Option<&'a mut Rule> {
    let (scope, title) = key.split_once('/')?;
    match scope {
        "p" => program.rules.iter_mut().find(|rule| rule.title == title),
        "c" => program
            .concentrations
            .iter_mut()
            .filter(|block| Some(block.title.as_str()) == concentration)
            .flat_map(|block| block.rules.iter_mut())
            .find(|rule| rule.title == title),
        "f" => program
            .profiles
            .iter_mut()
            .filter(|block| Some(block.title.as_str()) == profile)
            .flat_map(|block| block.rules.iter_mut())
            .find(|rule| rule.title == title),
        _ => None,
    }
}

// the coverage selection: everything the student has laid out, chosen, or
// holds by agreement — a credited course counts without a session
pub fn selection(plan: &Plan) -> BTreeSet<String> {
    plan.displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .chain(plan.electives.iter())
        .chain(plan.credited.iter())
        .cloned()
        .collect()
}

// One picker row per program code: several vintages of one program are not
// several programs (note d'Antoine 2026-08-17) — B-GMC alone ships ten, all
// titled « Baccalauréat en génie mécanique ». The row carries the vintages
// so the view can offer them in a select instead of repeating the title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramVintages {
    pub code: String,
    // the newest vintage's: two vintages may diverge, and the row announces
    // the one its select preselects
    pub title: String,
    pub credits_required: i64,
    // « A26 », « H26 », … newest first; the first one is the preselection
    pub vintages: Vec<String>,
}

// Codes in the snapshot's order (already sorted by `parse_data`), vintages
// newest first. Each group carries the newest program it has seen so far,
// so no step of this ever has an empty list to explain away. The scan is
// quadratic over a couple of dozen entries — the grouping stays a plain
// scan rather than a map, which would lose that order.
pub fn program_vintages(snapshot: &Snapshot) -> Vec<ProgramVintages> {
    // (the newest vintage of the code, every vintage of it)
    let mut groups: Vec<(&Program, Vec<&Program>)> = Vec::new();
    for program in &snapshot.programs {
        match groups
            .iter_mut()
            .find(|(newest, _)| newest.code == program.code)
        {
            None => groups.push((program, vec![program])),
            Some((newest, group)) => {
                // `parse_data` sorts on the « A26 » spelling, which puts
                // every automne before every hiver: a file that comes later
                // can still be the newer vintage
                if state::semester_rank(program.semester)
                    > state::semester_rank(newest.semester)
                {
                    *newest = program;
                }
                group.push(program);
            }
        }
    }
    groups
        .into_iter()
        .map(|(newest, mut group)| {
            // by rank for the same reason, never by the spelling
            group.sort_by_key(|program| {
                std::cmp::Reverse(state::semester_rank(program.semester))
            });
            ProgramVintages {
                code: newest.code.clone(),
                title: newest.title.clone(),
                credits_required: newest.credits_required,
                vintages: group
                    .iter()
                    .map(|program| program.semester.to_string())
                    .collect(),
            }
        })
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
    let (concentration, profile) = scope_of(plan);
    chosen_program(snapshot, plan).map(|program| {
        granted_program(program, concentration, profile, &plan.rule_grants).0
    })
}

// the chosen concentration and profile titles, read off the plan's choice
pub fn scope_of(plan: &Plan) -> (Option<&str>, Option<&str>) {
    match plan.program.as_ref() {
        None => (None, None),
        Some(choice) => {
            (choice.concentration.as_deref(), choice.profile.as_deref())
        }
    }
}

// the rules an agreement can attach a course to — a plain list, or a
// keyword rule (« negotiated », « tous les cours ») whose grant is exactly
// the attachment that makes it countable; keyed like the sections. Only
// the chosen blocks offer their rules: attaching a course to an unselected
// concentration would count it nowhere.
pub fn grantable_rules(
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> Vec<(String, String)> {
    let grantable = |rule: &Rule| {
        // never the préparatoire: attaching a course there would make it
        // « acquis » the moment the checkbox is on — no entente means that
        rule.title != ulaval_scheduler_core::PREPARATORY_RULE_TITLE
            && matches!(
                rule.courses,
                RuleCourses::List { .. }
                    | RuleCourses::Keyword {
                        courses: ulaval_scheduler_core::Keyword::Any
                            | ulaval_scheduler_core::Keyword::Negotiated,
                        ..
                    }
            )
    };
    let keyed = |prefix: char, scope: &str, rule: &Rule| {
        (
            format!("{prefix}/{}", rule.title),
            format!("{scope} — {}", rule.title),
        )
    };
    program
        .rules
        .iter()
        .filter(|rule| grantable(rule))
        .map(|rule| keyed('p', "Programme", rule))
        .chain(
            program
                .concentrations
                .iter()
                .filter(|block| Some(block.title.as_str()) == concentration)
                .flat_map(|block| {
                    block.rules.iter().filter(|rule| grantable(rule)).map(
                        |rule| {
                            keyed(
                                'c',
                                &format!("Concentration « {} »", block.title),
                                rule,
                            )
                        },
                    )
                }),
        )
        .chain(
            program
                .profiles
                .iter()
                .filter(|block| Some(block.title.as_str()) == profile)
                .flat_map(|block| {
                    block.rules.iter().filter(|rule| grantable(rule)).map(
                        |rule| {
                            keyed(
                                'f',
                                &format!("Profil « {} »", block.title),
                                rule,
                            )
                        },
                    )
                }),
        )
        .collect()
}

fn grant_label(
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    key: &str,
) -> String {
    grantable_rules(program, concentration, profile)
        .into_iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, label)| label)
        .unwrap_or_else(|| {
            key.split_once('/')
                .map(|(_, title)| title)
                .unwrap_or(key)
                .to_string()
        })
}

// The cheminement row's model: the offered titles and the current choice.
// None when no program is chosen or when the program offers neither — the
// row has nothing to say (M-GEX). A knob whose own list is empty is not
// rendered either (B-GEX has no concentrations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheminementChoices {
    pub concentrations: Vec<String>,
    pub profiles: Vec<String>,
    pub concentration: Option<String>,
    pub profile: Option<String>,
    // whether the synthetic « Aucune » option is offered at all: a program
    // whose page already carries a neutral block (« Cheminement sans
    // concentration », « Approche généraliste ») has nothing behind
    // « Aucune » but a sous-compte of that block's own rule (ADR
    // `2026-08-aucune-retiree-quand-un-bloc-neutre-existe`)
    pub offers_none: bool,
}

pub fn cheminement_choices(
    snapshot: &Snapshot,
    plan: &Plan,
) -> Option<CheminementChoices> {
    let program = chosen_program(snapshot, plan)?;
    if program.concentrations.is_empty() && program.profiles.is_empty() {
        return None;
    }
    let (concentration, profile) = scope_of(plan);
    Some(CheminementChoices {
        concentrations: program
            .concentrations
            .iter()
            .map(|block| block.title.clone())
            .collect(),
        profiles: program
            .profiles
            .iter()
            .map(|block| block.title.clone())
            .collect(),
        offers_none: !program
            .concentrations
            .iter()
            .any(|block| neutral_concentration(&block.title)),
        concentration: concentration.map(str::to_string),
        profile: profile.map(str::to_string),
    })
}

// the scraped blocks that *are* the no-concentration pathway — B-GCI and
// B-GMC name it, B-GIN calls it « Approche généraliste »
fn neutral_concentration(title: &str) -> bool {
    title == "Cheminement sans concentration"
        || title == "Approche généraliste"
}

// The electives the departing block brought along and nothing under the
// new scope lists: an auto-placed « Robotique » elective surviving under
// « Génie du développement durable » sat in the grid and the totals
// attached to nothing (contre-test étudiante-cegep 2026-08-20). Purged
// with the very act that changes the block — one « Annuler » restores
// everything. Coverage means the explicit lists (mandatory, List rules, a
// one-hop Reference resolved), never the « tous les cours » keyword: a
// course only an entente could attach is an orphan until the entente
// exists (ADR `2026-08-electifs-orphelins-purges-au-changement-de-bloc`).
pub fn scope_orphans(
    program: &Program,
    plan: &Plan,
    departing: Option<&str>,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> Vec<String> {
    let Some(departing) = departing else {
        return Vec::new();
    };
    // Concentration and Profile are distinct types with the same face —
    // flattened to (title, mandatory, rules) so one walk serves both
    let blocks = || {
        program
            .concentrations
            .iter()
            .map(|block| {
                (block.title.as_str(), &block.mandatory, &block.rules)
            })
            .chain(program.profiles.iter().map(|block| {
                (block.title.as_str(), &block.mandatory, &block.rules)
            }))
    };
    let mut from_block = BTreeSet::new();
    for (title, mandatory, rules) in blocks() {
        if title == departing {
            from_block.extend(mandatory.iter().map(String::as_str));
            listed_codes(program, rules, &mut from_block);
        }
    }
    let mut covered: BTreeSet<&str> =
        program.mandatory.iter().map(String::as_str).collect();
    listed_codes(program, &program.rules, &mut covered);
    for (title, mandatory, rules) in blocks() {
        if Some(title) == concentration || Some(title) == profile {
            covered.extend(mandatory.iter().map(String::as_str));
            listed_codes(program, rules, &mut covered);
        }
    }
    // wherever the plan holds the course: a concentration's mandatory is
    // auto-placed straight into `displayed_placement` without ever being
    // an elective — and the placement self-perpetuates through the next
    // request's seed if it survives here
    let held: BTreeSet<&str> = plan
        .electives
        .iter()
        .chain(plan.displayed_placement.keys())
        .chain(plan.pinned_sessions.keys())
        .chain(plan.manual.values().flatten())
        .map(String::as_str)
        .collect();
    held.into_iter()
        .filter(|code| from_block.contains(code))
        .filter(|code| !covered.contains(code))
        .map(str::to_string)
        .collect()
}

// the codes a rule set lists explicitly — a Reference resolves one hop to
// its target's list (references never chain, core refuses them)
fn listed_codes<'a>(
    program: &'a Program,
    rules: &'a [Rule],
    into: &mut BTreeSet<&'a str>,
) {
    for rule in rules {
        match &rule.courses {
            RuleCourses::List { courses } => {
                into.extend(courses.iter().map(String::as_str));
            }
            RuleCourses::Reference { courses, .. } => {
                for block in &program.concentrations {
                    if block.title != courses.concentration {
                        continue;
                    }
                    for target in &block.rules {
                        if target.title != courses.rule {
                            continue;
                        }
                        if let RuleCourses::List { courses } = &target.courses
                        {
                            into.extend(courses.iter().map(String::as_str));
                        }
                    }
                }
            }
            RuleCourses::Keyword { .. } | RuleCourses::Raw { .. } => {}
        }
    }
}

// The expert-safe default (AIR LAY-3, parité avec la version JS) : the
// page's first concentration when the program has any — never a profile.
// An explicit « Aucune » afterwards is the student's and persists.
pub fn default_concentration(
    snapshot: &Snapshot,
    code: &str,
    semester: &str,
) -> Option<String> {
    snapshot
        .programs
        .iter()
        .find(|program| {
            program.code == code && program.semester.to_string() == semester
        })
        .and_then(|program| program.concentrations.first())
        .map(|block| block.title.clone())
}

// the header's subtitle, the choice named whole (parité avec la version
// JS) : « Titre (CODE version A26) — Concentration — Profil »
pub fn program_subtitle(snapshot: &Snapshot, plan: &Plan) -> Option<String> {
    let program = chosen_program(snapshot, plan)?;
    let (concentration, profile) = scope_of(plan);
    let mut parts = vec![format!(
        "{} ({} version {})",
        program.title, program.code, program.semester
    )];
    parts.extend(concentration.map(str::to_string));
    parts.extend(profile.map(str::to_string));
    Some(parts.join(" — "))
}

// Some(section key) when taking `code` must record an entente as part of
// the same act: the take came from the browse of a « tous les cours » rule
// (`browse_key`), it is a first take, and no entente binds the course yet —
// an agreement already granted is never overwritten (ADR
// `2026-08-entente-vers-une-regle-any`).
pub fn grant_on_take(
    plan: &Plan,
    code: &str,
    choice: Choice,
    browse_key: Option<&str>,
) -> Option<String> {
    let key = browse_key?;
    if choice != Choice::Not || plan.rule_grants.contains_key(code) {
        return None;
    }
    Some(key.to_string())
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
    let rows = unique_rows(
        snapshot,
        plan,
        satisfied.iter().chain(missing.iter()).copied(),
    );
    Section {
        key: "obligatoires".to_string(),
        title: "Obligatoires".to_string(),
        badge: if missing.is_empty() && total > 0 {
            Badge::Ok(format!("{}/{total}", satisfied.len()))
        } else if satisfied.is_empty() {
            Badge::Missing(format!("0/{total}"))
        } else {
            Badge::Partial(format!("{}/{total}", satisfied.len()))
        },
        lead: None,
        rows,
        raw: None,
        notes: Vec::new(),
        free: false,
        progress: Some((satisfied.len(), total)),
    }
}

// `rule` is the granted rule core counted; `original` the rule as the
// program wrote it — the browse and the raw text of a « tous les cours »
// rule outlive its grants' Keyword → List transformation
fn rule_section(
    snapshot: &Snapshot,
    plan: &Plan,
    program: &Program,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
    original: Option<&Rule>,
    all: &[ulaval_scheduler_core::RuleReport],
) -> Section {
    let rows = rule
        .and_then(|rule| {
            ulaval_scheduler_core::resolved_rule_courses(program, rule)
                .ok()
                .flatten()
        })
        .map(|courses| unique_rows(snapshot, plan, courses.iter()))
        .unwrap_or_default();
    let rows = mark_counted_elsewhere(rows, report, all);
    let scope_prefix = match report.scope {
        Scope::Program => "p",
        Scope::Concentration => "c",
        Scope::Profile => "f",
    };
    let badge = rule_badge(snapshot, report, rule);
    // only an unsatisfied choice needs the explanation — a rule already
    // filled explains itself, a raw-only rule has no list to pick from
    let lead = (!rows.is_empty()
        && matches!(badge, Badge::Missing(_) | Badge::Partial(_)))
    .then(|| {
        rule_lead(report.scope, rule.and_then(|rule| rule.constraint.as_ref()))
    });
    Section {
        key: format!("{scope_prefix}/{}", report.title),
        title: report.title.clone(),
        badge,
        lead,
        rows,
        raw: report.raw.clone().or_else(|| original.and_then(rule_raw)),
        notes: rule.map(|rule| rule.notes.clone()).unwrap_or_default(),
        free: original.or(rule).is_some_and(browses_catalogue),
        progress: None,
    }
}

// A row `report.elsewhere` names is already counted by an earlier rule of
// the same scope (core's doing, decision d'Antoine 2026-08-23) — the state
// is carried by the row's own text, never a border alone (AIR INP-3), so
// this rewrites its `state` and appends to whatever `sub` text `row(...)`
// already computed instead of replacing it: a crédité or entente row must
// keep saying so, not just wherever this rule counted it.
fn mark_counted_elsewhere(
    mut rows: Vec<Row>,
    report: &ulaval_scheduler_core::RuleReport,
    all: &[ulaval_scheduler_core::RuleReport],
) -> Vec<Row> {
    for row in &mut rows {
        // a code absent from the catalogue was never actionable to begin
        // with, and an Acquired row already sits at its own rank (no +, no
        // chips) — demoting either to CountedElsewhere would either invent
        // an owner for a row that never had one, or hand an Acquired row
        // controls (the entente strip) the view refuses it
        if matches!(row.state, RowState::Unknown | RowState::Acquired)
            || !report.elsewhere.contains(&row.code)
        {
            continue;
        }
        let Some(owner) = all.iter().find(|other| {
            other.scope == report.scope
                && other
                    .counted
                    .as_deref()
                    .is_some_and(|counted| counted.contains(&row.code))
        }) else {
            // core's own invariant guarantees an owner exists; if it ever
            // doesn't, no invented text beats a plain, honest fallback
            continue;
        };
        row.state = RowState::CountedElsewhere;
        row.sub = format!("{} - compté dans la {}", row.sub, owner.title);
    }
    rows
}

// Nothing in a rule is ever taken automatically, and nothing on screen
// said so — the very gesture the comparison session repeats (« qu'est-ce
// que cette concentration change ? ») showed an unchanged grid and a rule
// at 0 without a word of explanation.
fn rule_lead(scope: Scope, constraint: Option<&Constraint>) -> String {
    let pick = match constraint {
        Some(Constraint::Course { min, max }) if min == max => {
            format!("Choisissez {min} cours dans cette liste")
        }
        Some(Constraint::Course { min, max }) => {
            format!("Choisissez de {min} à {max} cours dans cette liste")
        }
        Some(Constraint::Credits { min, max }) if min == max => {
            format!("Choisissez {min} crédits de cours dans cette liste")
        }
        Some(Constraint::Credits { min, max }) => {
            format!(
                "Choisissez de {min} à {max} crédits de cours dans cette \
                 liste"
            )
        }
        None => "Choisissez dans cette liste".to_string(),
    };
    let origin = scope_origin(scope);
    format!("{pick}{origin} — rien n'est pris automatiquement.")
}

// the French suffix naming a rule's scope, shared between the rule
// header (`rule_lead`) and the over-max error message
// (`coverage_error_message`) so the wording never drifts between the two
fn scope_origin(scope: Scope) -> &'static str {
    match scope {
        Scope::Program => "",
        Scope::Concentration => " de la concentration",
        Scope::Profile => " du profil",
    }
}

// The répertoire's lists can repeat a code (B-GMC's « Règle 1 » carries
// GEL-4799 twice); rows are keyed by code in the view, so a duplicate
// would give two siblings the same key and panic the diff — first
// occurrence wins, order kept.
fn unique_rows<'a>(
    snapshot: &Snapshot,
    plan: &Plan,
    codes: impl Iterator<Item = &'a String>,
) -> Vec<Row> {
    let mut seen = BTreeSet::new();
    codes
        .filter(|code| seen.insert(code.as_str()))
        .map(|code| row(snapshot, plan, code))
        .collect()
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
    label.map(|label| en_sus(label, rule))
}

fn rule_badge(
    snapshot: &Snapshot,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
) -> Badge {
    match report.status {
        RuleStatus::Satisfied => match constrained(rule) {
            Some((rule, constraint)) => {
                let (text, _) = constraint_fraction(
                    snapshot,
                    report.counted.as_deref().unwrap_or_default(),
                    rule,
                    constraint,
                );
                Badge::Ok(format!("✓ {text}"))
            }
            None => Badge::Ok("✓".to_string()),
        },
        // a reported rule that still carries a countable constraint
        // (« any », « negotiated ») says what remains — « 0/3 cr » — since
        // an entente can now fill it; « — » would hide a real requirement
        RuleStatus::Reported
            if rule.is_some_and(|rule| rule.constraint.is_some()) =>
        {
            incomplete_badge(snapshot, report, rule)
        }
        RuleStatus::Reported => Badge::Neutral("—".to_string()),
        RuleStatus::Incomplete => incomplete_badge(snapshot, report, rule),
    }
}

// « 0/3 » (credits counted / maximum) or « 1/2 » (courses) — the design's
// wording, computed from what the report already counted
fn incomplete_badge(
    snapshot: &Snapshot,
    report: &ulaval_scheduler_core::RuleReport,
    rule: Option<&Rule>,
) -> Badge {
    let counted = report.counted.as_deref().unwrap_or_default();
    match constrained(rule) {
        Some((rule, constraint)) => {
            let (text, any) =
                constraint_fraction(snapshot, counted, rule, constraint);
            if any {
                Badge::Partial(text)
            } else {
                Badge::Missing(text)
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

// une règle et sa contrainte quand les deux existent — le badge ne compte
// que dans ce cas
fn constrained(rule: Option<&Rule>) -> Option<(&Rule, &Constraint)> {
    let rule = rule?;
    rule.constraint
        .as_ref()
        .map(|constraint| (rule, constraint))
}

// « 6/9 cr », « 1/1 » : ce que la règle a compté sur le maximum de sa
// contrainte — le numérateur n'est jamais borné — et si elle a compté
// quoi que ce soit (Missing contre Partial)
fn constraint_fraction(
    snapshot: &Snapshot,
    counted: &[String],
    rule: &Rule,
    constraint: &Constraint,
) -> (String, bool) {
    let (label, any) = match *constraint {
        Constraint::Course { max, .. } => {
            (format!("{}/{max}", counted.len()), !counted.is_empty())
        }
        Constraint::Credits { max, .. } => {
            let sum: u32 = counted
                .iter()
                .filter_map(|code| snapshot.by_code.get(code))
                .map(|&index| snapshot.courses[index].credits.planning())
                .sum();
            (format!("{sum}/{max} cr"), sum > 0)
        }
    };
    (en_sus(label, rule), any)
}

// les crédits du stage promu sont en sus du total du diplôme : le libellé
// le dit, sur le badge comme sur l'en-tête
fn en_sus(label: String, rule: &Rule) -> String {
    if rule.credits_in_addition {
        format!("{label} - en sus")
    } else {
        label
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
    // same rank as the préparatoire hypothesis, same reason: a credited
    // course holds no session, so a leftover placement must not be shown
    if plan.credited.contains(code) {
        return Row {
            code: code.to_string(),
            title,
            credits,
            sub: "crédité - ne prend pas de session".to_string(),
            state: RowState::Credited,
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

// what the student holds: everything laid on the organigramme plus what an
// agreement credited him, with the planning credits of those courses (the
// static prerequisite question of `prerequisites_met`)
fn acquired(snapshot: &Snapshot, plan: &Plan) -> (BTreeSet<String>, u32) {
    let held: BTreeSet<String> = plan
        .displayed_placement
        .keys()
        .chain(plan.manual.values().flatten())
        .chain(plan.credited.iter())
        .cloned()
        .collect();
    let credits = held
        .iter()
        .filter_map(|code| snapshot.by_code.get(code))
        .map(|&index| snapshot.courses[index].credits.planning())
        .sum();
    (held, credits)
}

// --- the choice strip of a row --------------------------------------------

// What the student decided about a course. « Automatique » is the intuitive
// act — take the course, let the solver find it a session; a session is the
// same act plus a freeze (ADR `2026-08-choix-automatique-ou-session-gelee`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Not,
    Auto,
    Pinned(usize),
}

// Everything the row's choice strip needs, answered pure: is the course
// imposed by the program, which sessions may host it, and what is chosen
// today.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceStrip {
    // a mandatory course is always chosen: no way to un-choose it
    pub mandatory: bool,
    // the horizon sessions the course is offered in, 1-based with their
    // label — a season filter, not an admissibility verdict
    pub sessions: Vec<(usize, String)>,
    pub choice: Choice,
}

pub fn choice_strip(
    snapshot: &Snapshot,
    plan: &Plan,
    code: &str,
) -> ChoiceStrip {
    let mandatory = is_mandatory(snapshot, plan, code);
    ChoiceStrip {
        mandatory,
        sessions: candidate_sessions(snapshot, plan, code),
        choice: choice(plan, code, mandatory),
    }
}

// The horizon sessions whose season offers the course — what the ribbon
// marks while a course is dragged: offered sessions keep their face,
// the others fade and refuse the drop. The same season filter as the
// chips (INP-4 parity), never the solver probe: the probe took seconds
// and barred nearly every card (retour d'Antoine, 2026-08-19).
pub fn offered_sessions(
    snapshot: &Snapshot,
    plan: &Plan,
    code: &str,
) -> std::collections::BTreeSet<usize> {
    candidate_sessions(snapshot, plan, code)
        .into_iter()
        .map(|(index, _)| index)
        .collect()
}

// imposed by the program, its chosen concentration or its chosen profile —
// read from the program itself, so an obligatoire listed under a rule or
// met while browsing is marked there too
fn is_mandatory(snapshot: &Snapshot, plan: &Plan, code: &str) -> bool {
    let Some(program) = chosen_program(snapshot, plan) else {
        return false;
    };
    let choice = plan.program.as_ref();
    let concentration = choice.and_then(|c| c.concentration.as_deref());
    let profile = choice.and_then(|c| c.profile.as_deref());
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
        .any(|list| list.iter().any(|held| held == code))
}

// The sessions offered as freeze targets: a plain season filter over the
// horizon, read from the snapshot — no solver probe, so every visible row
// can show its strip at once (a probe per row cost one solve each). A
// session barred by the prerequisites stays clickable; `validate_new_code`
// warns then, as it always did.
fn candidate_sessions(
    snapshot: &Snapshot,
    plan: &Plan,
    code: &str,
) -> Vec<(usize, String)> {
    let Some(&index) = snapshot.by_code.get(code) else {
        return Vec::new();
    };
    let course = &snapshot.courses[index];
    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let semesters = state::session_semesters(plan.start, &seasons);
    semesters
        .iter()
        .enumerate()
        .filter(|(_, semester)| course.seasons.contains_key(&semester.season))
        .map(|(index, _)| (index + 1, state::session_label(&semesters, index)))
        .collect()
}

// A pin — explicit or inherited from a hand-added session — freezes;
// anything else the student took, plus every mandatory course, is the
// solver's to place.
fn choice(plan: &Plan, code: &str, mandatory: bool) -> Choice {
    let pinned = plan.pinned_sessions.get(code).copied().or_else(|| {
        plan.manual
            .iter()
            .find(|(_, codes)| codes.iter().any(|held| held == code))
            .map(|(&session, _)| session)
    });
    if let Some(session) = pinned {
        return Choice::Pinned(session);
    }
    let taken = mandatory
        || plan.electives.iter().any(|held| held == code)
        || plan.displayed_placement.contains_key(code);
    if taken {
        Choice::Auto
    } else {
        Choice::Not
    }
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
        "mandatory":["ANL-2020"],
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
                manual: None,
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
    fn offered_sessions_are_the_chips_season_filter_as_indices() {
        let snapshot = snapshot();
        let plan = plan();
        assert_eq!(
            offered_sessions(&snapshot, &plan, "GEX-3000"),
            std::collections::BTreeSet::from([1, 4, 7, 10]),
            "an automne course marks only the automnes"
        );
        assert_eq!(
            offered_sessions(&snapshot, &plan, "GAE-1000"),
            std::collections::BTreeSet::from([2, 5, 8, 11]),
            "a winter course marks only the hivers"
        );
        assert!(
            offered_sessions(&snapshot, &plan, "GHOST-1").is_empty(),
            "no Course at all still answers instead of blanking"
        );
    }

    #[test]
    fn the_choice_strip_offers_automatique_then_the_seasons_it_fits() {
        let snapshot = snapshot();
        let mut plan = plan();
        // eight study sessions from an automne: A H É A H É A H É A H É
        let fall = choice_strip(&snapshot, &plan, "GEX-3000");
        assert_eq!(
            fall.sessions
                .iter()
                .map(|(index, _)| *index)
                .collect::<Vec<_>>(),
            [1, 4, 7, 10],
            "an automne course only fits the automnes"
        );
        assert_eq!(fall.sessions[0].1, "A1-A26");
        let summer = choice_strip(&snapshot, &plan, "ETE-1000");
        assert_eq!(
            summer
                .sessions
                .iter()
                .map(|(index, _)| *index)
                .collect::<Vec<_>>(),
            [3, 6, 9, 12]
        );
        assert_eq!(summer.sessions[0].1, "É27");
        // no season known, and no Course at all: nothing to offer, and the
        // strip still answers instead of blanking
        assert!(choice_strip(&snapshot, &plan, "HOR-0000")
            .sessions
            .is_empty());
        assert!(choice_strip(&snapshot, &plan, "GHOST-1")
            .sessions
            .is_empty());

        // an horizon of two sessions cuts the list to what exists
        plan.study_sessions = 2;
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-3000")
                .sessions
                .iter()
                .map(|(index, _)| *index)
                .collect::<Vec<_>>(),
            [1]
        );
    }

    #[test]
    fn the_choice_reads_taken_frozen_or_untouched() {
        let snapshot = snapshot();
        let mut plan = plan();
        // untouched: offered, never taken
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-3000").choice,
            Choice::Not
        );
        // taken and left to the solver
        plan.electives.push("GEX-3000".to_string());
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-3000").choice,
            Choice::Auto
        );
        // the solver placed it: still the solver's, no freeze
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-1000").choice,
            Choice::Auto,
            "a displayed placement is a choice, not a pin"
        );
        // frozen by the student
        plan.pinned_sessions.insert("GEX-3000".to_string(), 4);
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-3000").choice,
            Choice::Pinned(4)
        );
        // a hand-added session is a freeze too — a plan saved before the
        // strip existed still reads right
        plan.manual.insert(7, vec!["GEX-2000".to_string()]);
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-2000").choice,
            Choice::Pinned(7)
        );
    }

    #[test]
    fn a_mandatory_course_is_always_taken_program_or_concentration() {
        let snapshot = snapshot();
        let mut plan = plan();
        // untouched in the plan, yet imposed: the strip says taken, and
        // the view gives it no ✕
        let strip = choice_strip(&snapshot, &plan, "GEX-2000");
        assert!(strip.mandatory);
        assert_eq!(strip.choice, Choice::Auto);
        // ANL-2020 is only mandatory once the concentration is chosen
        assert!(!choice_strip(&snapshot, &plan, "ANL-2020").mandatory);
        assert_eq!(
            choice_strip(&snapshot, &plan, "ANL-2020").choice,
            Choice::Not
        );
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
        }
        assert!(choice_strip(&snapshot, &plan, "ANL-2020").mandatory);
        // a freeze still wins over the « automatique » an obligatoire gets
        plan.pinned_sessions.insert("GEX-2000".to_string(), 1);
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-2000").choice,
            Choice::Pinned(1)
        );
        // no program, no obligation
        assert!(
            !choice_strip(&snapshot, &Plan::default(), "GEX-2000").mandatory
        );
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
            Badge::Ok("✓ 1/1".to_string()),
            "the count is shown, never the course code"
        );
        assert_eq!(model.rules[1].badge, Badge::Missing("0/9 cr".to_string()));
        assert_eq!(model.rules[1].notes, ["une note de règle"]);
        assert!(model.rules[2].free, "the any rule browses the catalogue");
        assert_eq!(
            model.rules[2].raw.as_deref(),
            Some("Tous les cours de premier cycle")
        );
        assert_eq!(
            model.rules[2].badge,
            Badge::Missing("0/3 cr".to_string()),
            "a reported rule with a constraint says what remains"
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
    fn an_unsatisfied_rule_opens_on_what_to_pick_here() {
        let model = panel_model(&snapshot(), &plan());
        // Règle 1 is satisfied by the placed GMN-1000: nothing to explain
        assert_eq!(model.rules[0].lead, None, "{:?}", model.rules[0].badge);
        // Règle 2 (6–9 cr) is empty: the lead says what to pick and that
        // nothing is automatic
        assert_eq!(
            model.rules[1].lead.as_deref(),
            Some(
                "Choisissez de 6 à 9 crédits de cours dans cette liste — \
                 rien n'est pris automatiquement."
            )
        );
        // a concentration rule names its origin — the comparison report's
        // gesture (« qu'est-ce que ça change ? ») now gets an answer
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
        }
        let model = panel_model(&snapshot(), &plan);
        let section = model
            .rules
            .iter()
            .find(|section| section.key == "c/Règle C1")
            .expect("the concentration rule is listed");
        let lead = section.lead.clone().expect("unsatisfied rule");
        assert!(lead.contains("de la concentration"), "{lead}");
        assert!(lead.starts_with("Choisissez 2 cours"), "{lead}");
    }

    #[test]
    fn changing_block_purges_the_electives_nothing_else_lists() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        let mut plan = plan();
        // ANL-2020 exists only in « Génie urbain »'s mandatory list;
        // GAE-1000 sits in the concentration too but the program's
        // Règle 2 lists it as well — covered, it stays
        plan.electives = vec!["ANL-2020".to_string(), "GAE-1000".to_string()];
        assert_eq!(
            scope_orphans(program, &plan, Some("Génie urbain"), None, None),
            ["ANL-2020"]
        );
        // an auto-placed concentration mandatory never was an elective —
        // it lives in `displayed_placement` alone and must go too
        plan.electives.clear();
        plan.displayed_placement.insert("ANL-2020".to_string(), 2);
        assert_eq!(
            scope_orphans(program, &plan, Some("Génie urbain"), None, None),
            ["ANL-2020"]
        );
        // arriving on the very same block: everything still listed
        assert!(scope_orphans(
            program,
            &plan,
            Some("Génie urbain"),
            Some("Génie urbain"),
            None
        )
        .is_empty());
        // no departing block, nothing brought along
        assert!(scope_orphans(program, &plan, None, None, None).is_empty());
    }

    #[test]
    fn a_reference_covers_its_targets_list_one_hop() {
        // Y's rule references X's Règle 1: leaving X for Y keeps ZZZ-1
        let program: Program = serde_json::from_str(
            r#"{"code":"B-T","slug":"t","semester":"A26","title":"T",
                "cycle":1,"credits_required":90,"mandatory":[],"rules":[],
                "concentrations":[
                  {"title":"X","mandatory":[],
                   "rules":[{"title":"Règle 0",
                             "constraint":{"type":"course","min":1,"max":1},
                             "courses":["AAA-1"]},
                            {"title":"Règle 1",
                             "constraint":{"type":"credits","min":3,"max":3},
                             "courses":["ZZZ-1","ZZZ-2"]}]},
                  {"title":"Y","mandatory":[],
                   "rules":[{"title":"Règle A",
                             "constraint":{"type":"credits","min":3,"max":3},
                             "courses":{"concentration":"X",
                                        "rule":"Règle 1"},
                             "raw":"tous les cours de la Règle 1 de X"}]}
                ],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = plan();
        plan.electives = vec!["ZZZ-1".to_string()];
        assert!(
            scope_orphans(&program, &plan, Some("X"), Some("Y"), None)
                .is_empty(),
            "the reference resolves to X's list"
        );
    }

    // a one-hop reference resolves only a `List` target — a target with
    // no fixed list (keyword « any ») is left uncovered rather than
    // guessed at, so the departing elective still shows as an orphan
    #[test]
    fn a_reference_to_a_non_list_target_covers_nothing() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-T","slug":"t","semester":"A26","title":"T",
                "cycle":1,"credits_required":90,"mandatory":[],"rules":[],
                "concentrations":[
                  {"title":"X","mandatory":[],
                   "rules":[{"title":"Règle 1",
                             "constraint":{"type":"credits","min":3,"max":3},
                             "courses":["ZZZ-1","ZZZ-2"]},
                            {"title":"Règle 2","courses":"any",
                             "raw":"tous les cours de premier cycle"}]},
                  {"title":"Y","mandatory":[],
                   "rules":[{"title":"Règle A",
                             "constraint":{"type":"credits","min":3,"max":3},
                             "courses":{"concentration":"X",
                                        "rule":"Règle 2"},
                             "raw":"tous les cours de la Règle 2 de X"}]}
                ],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let mut plan = plan();
        plan.electives = vec!["ZZZ-1".to_string()];
        assert_eq!(
            scope_orphans(&program, &plan, Some("X"), Some("Y"), None),
            ["ZZZ-1"],
            "Règle A references a keyword rule, not a list: nothing \
             resolves, so ZZZ-1 stays uncovered and orphans"
        );
    }

    #[test]
    fn the_rule_lead_speaks_every_constraint_shape() {
        let course = |min, max| Constraint::Course { min, max };
        let credits = |min, max| Constraint::Credits { min, max };
        assert_eq!(
            rule_lead(Scope::Program, Some(&course(1, 1))),
            "Choisissez 1 cours dans cette liste — rien n'est pris \
             automatiquement."
        );
        assert!(rule_lead(Scope::Program, Some(&course(1, 3)))
            .starts_with("Choisissez de 1 à 3 cours"));
        let profile = rule_lead(Scope::Profile, Some(&credits(12, 12)));
        assert!(
            profile.contains(
                "12 crédits de cours dans cette liste du \
                              profil"
            ),
            "{profile}"
        );
        assert!(rule_lead(Scope::Program, None)
            .starts_with("Choisissez dans cette liste"));
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
        // the constraint stays readable even without a count (no chip left
        // to carry it): the neutral badge names it instead of a bare « — »
        assert_eq!(
            model.rules[0].badge,
            Badge::Neutral("1 parmi".to_string())
        );
        assert_eq!(model.rules[1].badge, Badge::Neutral("6–9 cr".to_string()));
        assert_eq!(model.rules[2].badge, Badge::Neutral("3 cr".to_string()));
        assert_eq!(model.rules[3].badge, Badge::Neutral("—".to_string()));
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
                scope: Scope::Concentration,
                total: 12,
                max: 9,
            },
        );
        assert!(
            credits.starts_with("Règle 2 de la concentration"),
            "{credits}"
        );
        assert!(credits.contains("12 crédits"), "{credits}");
        assert!(credits.contains("maximum de 9"), "{credits}");

        let count = coverage_error_message(
            &ulaval_scheduler_core::CoverageError::CountOverMax {
                rule: "Règle 3".to_string(),
                scope: Scope::Profile,
                total: 2,
                max: 1,
            },
        );
        assert!(count.contains("du profil"), "{count}");

        let program = coverage_error_message(
            &ulaval_scheduler_core::CoverageError::CountOverMax {
                rule: "Règle 1".to_string(),
                scope: Scope::Program,
                total: 2,
                max: 1,
            },
        );
        assert!(program.starts_with("Règle 1 :"), "{program}");
    }

    #[test]
    fn a_duplicated_code_in_a_rule_yields_one_row() {
        // B-GMC's « Règle 1 » lists GEL-4799 twice; two rows would share
        // a render key and panic Dioxus's keyed diff
        let rule: Rule = serde_json::from_str(
            r#"{"title":"Règle 1",
                "constraint":{"type":"credits","min":3,"max":3},
                "courses":["GMN-1000","GAE-1000","GMN-1000"]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let snapshot = snapshot();
        let section = bare_section(
            &snapshot,
            &plan(),
            &snapshot.programs[0],
            'p',
            &rule,
            None,
        );
        let codes: Vec<&str> =
            section.rows.iter().map(|row| row.code.as_str()).collect();
        assert_eq!(codes, ["GMN-1000", "GAE-1000"], "first wins, order kept");
        assert_eq!(section.badge, Badge::Neutral("3 cr".to_string()));
    }

    #[test]
    fn a_reference_rule_keeps_its_raw_text_in_the_uncounted_fallback() {
        let rule: Rule = serde_json::from_str(
            r#"{"title":"R","courses":{"concentration":"X","rule":"Règle 1"},
                "raw":"tous les cours de la Règle 1 du cheminement X"}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let snapshot = snapshot();
        let section = bare_section(
            &snapshot,
            &plan(),
            &snapshot.programs[0],
            'p',
            &rule,
            None,
        );
        assert_eq!(
            section.raw.as_deref(),
            Some("tous les cours de la Règle 1 du cheminement X")
        );
        assert!(section.rows.is_empty());
        assert_eq!(section.badge, Badge::Neutral("—".to_string()));
    }

    #[test]
    fn a_reference_rule_renders_its_target_rows_and_keeps_its_raw() {
        let program: Program = serde_json::from_str(
            r#"{"code":"X","slug":"x","semester":"A26","title":"X",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[{"title":"Règle 2",
                  "courses":{"concentration":"Réservoir","rule":"Règle 1"},
                  "raw":"tous les cours de la Règle 1"}],
                "concentrations":[{"title":"Réservoir","mandatory":[],
                  "rules":[{"title":"Règle 1","courses":["GAE-1000",
                    "GMN-1000","GAE-1000"]}]}],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let snapshot = snapshot();
        let section = bare_section(
            &snapshot,
            &plan(),
            &program,
            'p',
            &program.rules[0],
            None,
        );
        assert_eq!(
            section
                .rows
                .iter()
                .map(|row| row.code.as_str())
                .collect::<Vec<_>>(),
            ["GAE-1000", "GMN-1000"]
        );
        assert_eq!(
            section.raw.as_deref(),
            Some("tous les cours de la Règle 1")
        );
    }

    #[test]
    fn panel_groups_are_ordered_and_keep_mandatory_scoped() {
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
            choice.profile = Some("Profil international".to_string());
        }
        let model = panel_model(&snapshot(), &plan);
        assert_eq!(
            model
                .groups
                .iter()
                .map(|group| group.title.as_str())
                .collect::<Vec<_>>(),
            [
                "Programme",
                "Concentration — Génie urbain",
                "Profil — Profil international"
            ]
        );
        assert_eq!(model.groups[0].sections[0].key, "p/obligatoires");
        assert_eq!(model.groups[1].sections[0].key, "c/obligatoires");
        assert!(model.groups[2]
            .sections
            .iter()
            .all(|section| section.key != "f/obligatoires"));
    }

    #[test]
    fn scope_progress_deduplicates_caps_and_names_missing_credits() {
        use ulaval_scheduler_core::{MandatoryReport, RuleReport};
        let snapshot = snapshot();
        let report = ulaval_scheduler_core::CoverageReport {
            mandatory: vec![MandatoryReport {
                scope: Scope::Concentration,
                satisfied: vec!["GAE-1000".to_string()],
                missing: Vec::new(),
            }],
            rules: vec![RuleReport {
                scope: Scope::Concentration,
                title: "Règle 1".to_string(),
                status: RuleStatus::Satisfied,
                counted: Some(vec![
                    "GAE-1000".to_string(),
                    "GMN-1000".to_string(),
                ]),
                elsewhere: Vec::new(),
                missing: None,
                candidates: Some(Vec::new()),
                raw: None,
            }],
            language_requirement: None,
        };
        assert_eq!(
            scope_progress(&snapshot, &report, Scope::Concentration, 5),
            "5/5 cr"
        );
        let mut missing = report.clone();
        missing.rules[0]
            .counted
            .as_mut()
            .expect("counted")
            .push("ABS-1000".to_string());
        assert_eq!(
            scope_progress(&snapshot, &missing, Scope::Concentration, 9),
            "—/9 cr — crédits inconnus pour ABS-1000"
        );

        let group = scope_group(
            &snapshot,
            &plan(),
            &report,
            &[],
            Scope::Concentration,
            "Concentration — Eau".to_string(),
            "c/",
            Some(5),
        );
        assert_eq!(group.progress.as_deref(), Some("5/5 cr"));
    }

    #[test]
    fn unavailable_scoped_progress_and_profile_mandatory_are_explicit() {
        let snapshot = snapshot();
        let plan = plan();
        let group = uncounted_scope_group(
            &snapshot,
            &plan,
            "Profil — Distinction".to_string(),
            &[],
            "f/",
            Some(12),
            &[],
        );
        assert_eq!(
            group.progress.as_deref(),
            Some("—/12 cr — progression indisponible")
        );
        let mandatory = ulaval_scheduler_core::MandatoryReport {
            scope: Scope::Profile,
            satisfied: Vec::new(),
            missing: vec!["GAE-1000".to_string()],
        };
        assert_eq!(
            scoped_mandatory_section(
                &snapshot,
                &plan,
                Scope::Profile,
                &mandatory,
            )
            .expect("one mandatory course")
            .key,
            "f/obligatoires"
        );
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
        let (granted, warnings) =
            granted_program(program, None, None, &grants);
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
    fn an_any_rule_counts_its_grants_and_keeps_its_browse() {
        let snapshot = snapshot();
        let mut plan = plan();
        // GAE-1000 (3 cr) is attached to the « tous les cours » Règle 3
        // but not taken yet: the rule counts, at zero
        plan.rule_grants = std::collections::BTreeMap::from([(
            "GAE-1000".to_string(),
            "p/Règle 3".to_string(),
        )]);
        let model = panel_model(&snapshot, &plan);
        let rule_3 = &model.rules[2];
        assert_eq!(rule_3.badge, Badge::Missing("0/3 cr".to_string()));
        assert!(rule_3.free, "the browse survives the grant");
        assert_eq!(
            rule_3.raw.as_deref(),
            Some("Tous les cours de premier cycle"),
            "the rule text survives the grant"
        );
        let attached = &rule_3.rows[0];
        assert_eq!(attached.code, "GAE-1000");
        assert!(attached.sub.ends_with("- entente"), "{}", attached.sub);
        // taken: the rule is satisfied and its badge shows the fraction
        plan.electives.push("GAE-1000".to_string());
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.rules[2].badge, Badge::Ok("✓ 3/3 cr".to_string()));
        // the entente moved it out of Règle 2's list (never two rules)
        assert!(model.rules[1].rows.iter().all(|row| row.code != "GAE-1000"));
    }

    #[test]
    fn a_second_grant_to_the_same_any_rule_joins_the_first() {
        let snapshot = snapshot();
        let grants = std::collections::BTreeMap::from([
            ("ETE-1000".to_string(), "p/Règle 3".to_string()),
            ("GAE-1000".to_string(), "p/Règle 3".to_string()),
        ]);
        let (granted, warnings) =
            granted_program(&snapshot.programs[0], None, None, &grants);
        assert!(warnings.is_empty(), "{warnings:?}");
        let rule_3 = granted
            .rules
            .iter()
            .find(|rule| rule.title == "Règle 3")
            .unwrap_or_else(|| panic!("kept"));
        assert_eq!(
            rule_3.courses,
            RuleCourses::List {
                courses: vec!["ETE-1000".to_string(), "GAE-1000".to_string()]
            },
            "the second grant appends to the first's list"
        );
    }

    #[test]
    fn a_credited_course_attached_to_an_any_rule_raises_no_warning() {
        let snapshot = snapshot();
        let mut plan = plan();
        // credited and attached: the rule lists it, so nothing is unlisted
        plan.credited.insert("ETE-1000".to_string());
        plan.rule_grants = std::collections::BTreeMap::from([(
            "ETE-1000".to_string(),
            "p/Règle 3".to_string(),
        )]);
        let model = panel_model(&snapshot, &plan);
        assert!(model.warnings.is_empty(), "{:?}", model.warnings);
        assert_eq!(
            model.rules[2].badge,
            Badge::Ok("✓ 3/3 cr".to_string()),
            "credited, attached: counted in the any rule"
        );
    }

    #[test]
    fn grant_on_take_grants_only_a_first_take_from_a_browse() {
        let mut plan = plan();
        // a first take from the Règle 3 browse records the entente
        assert_eq!(
            grant_on_take(&plan, "GAE-1000", Choice::Not, Some("p/Règle 3")),
            Some("p/Règle 3".to_string())
        );
        // outside a browse: nothing to record
        assert_eq!(grant_on_take(&plan, "GAE-1000", Choice::Not, None), None);
        // not a first take: the course was already accepted once
        assert_eq!(
            grant_on_take(&plan, "GAE-1000", Choice::Auto, Some("p/Règle 3")),
            None
        );
        // an agreement already granted is never overwritten
        plan.rule_grants
            .insert("GAE-1000".to_string(), "p/Règle 2".to_string());
        assert_eq!(
            grant_on_take(&plan, "GAE-1000", Choice::Not, Some("p/Règle 3")),
            None
        );
    }

    #[test]
    fn uncrediting_a_mandatory_course_keeps_it_counted_and_in_place() {
        let snapshot = snapshot();
        let mut plan = plan();
        // GEX-1000 is mandatory and placed; credit then uncredit it
        state::credit_code(&mut plan, "GEX-1000");
        state::uncredit_code(&mut plan, "GEX-1000");
        let model = panel_model(&snapshot, &plan);
        let mandatory = model.mandatory.unwrap_or_else(|| panic!("chosen"));
        assert_eq!(
            mandatory.badge,
            Badge::Partial("1/2".to_string()),
            "still counted among the obligatoires"
        );
        assert_eq!(
            mandatory.rows[0].code, "GEX-1000",
            "satisfied first: it never falls to the end of the list"
        );
        assert_eq!(mandatory.rows[0].state, RowState::Chosen);
        assert_eq!(
            choice_strip(&snapshot, &plan, "GEX-1000").choice,
            Choice::Auto,
            "the solver will give it a session again"
        );
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
                manual: None,
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
        assert_eq!(
            model
                .preparatory
                .as_ref()
                .map(|section| section.title.as_str()),
            Some("Scolarité préparatoire")
        );
        assert!(model.groups[0].sections.is_empty());
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
    fn a_credited_course_counts_without_a_session_and_never_hides() {
        let snapshot = snapshot();
        let mut plan = plan();
        // GAE-1000 (3 cr, Règle 2) is credited by an agreement: nowhere in
        // the organigramme, yet counted
        plan.credited.insert("GAE-1000".to_string());
        let model = panel_model(&snapshot, &plan);
        let rule_2 = model
            .rules
            .iter()
            .find(|section| section.title == "Règle 2")
            .unwrap_or_else(|| panic!("Règle 2 is in the fixture"));
        assert_eq!(rule_2.badge, Badge::Partial("3/9 cr".to_string()));
        let credited_row = rule_2
            .rows
            .iter()
            .find(|row| row.code == "GAE-1000")
            .unwrap_or_else(|| panic!("the rule lists it"));
        assert_eq!(credited_row.state, RowState::Credited);
        assert_eq!(credited_row.sub, "crédité - ne prend pas de session");
        // the state wins over a leftover placement: the healing effect is
        // about to purge it and the row must not offer it meanwhile
        plan.displayed_placement.insert("GAE-1000".to_string(), 1);
        assert_eq!(
            row(&snapshot, &plan, "GAE-1000").state,
            RowState::Credited
        );
        plan.displayed_placement.remove("GAE-1000");
        // held for the prerequisites too: GEX-9000 asks for GAE-1000
        assert!(
            acquired(&snapshot, &plan).0.contains("GAE-1000"),
            "a credited course is held"
        );
        // a credited course no section lists is named, never left to add
        // credits from nowhere
        plan.credited.insert("ETE-1000".to_string());
        let model = panel_model(&snapshot, &plan);
        assert!(
            model
                .warnings
                .iter()
                .any(|warning| warning.starts_with("ETE-1000 est crédité")),
            "{:?}",
            model.warnings
        );
        assert!(
            !model
                .warnings
                .iter()
                .any(|warning| warning.starts_with("GAE-1000")),
            "{:?}",
            model.warnings
        );
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
                manual: None,
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
        assert_eq!(c1.badge, Badge::Ok("✓ 2/2".to_string()));
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
        // Règle 2 (6–9 cr) with GAE-1000 laid out: 3/9 cr partial
        plan.displayed_placement.insert("GAE-1000".to_string(), 2);
        let model = panel_model(&snapshot, &plan);
        assert_eq!(model.rules[1].badge, Badge::Partial("3/9 cr".to_string()));
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
                elsewhere: Vec::new(),
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
    fn a_constrained_incomplete_rule_counts_against_the_maximum() {
        let rule = |constraint: &str, en_sus: bool| -> Rule {
            serde_json::from_str(&format!(
                r#"{{"title":"R","constraint":{constraint},
                     "courses":["X-1"],
                     "credits_in_addition":{en_sus}}}"#
            ))
            .unwrap_or_else(|e| panic!("rule literal: {e}"))
        };
        let report =
            |counted: Option<Vec<String>>| ulaval_scheduler_core::RuleReport {
                scope: Scope::Program,
                title: "R".to_string(),
                status: RuleStatus::Incomplete,
                counted,
                elsewhere: Vec::new(),
                missing: None,
                candidates: None,
                raw: None,
            };
        let snapshot = snapshot();

        let course_rule = rule(r#"{"type":"course","min":1,"max":3}"#, false);
        assert_eq!(
            incomplete_badge(&snapshot, &report(None), Some(&course_rule)),
            Badge::Missing("0/3".to_string())
        );
        assert_eq!(
            incomplete_badge(
                &snapshot,
                &report(Some(vec!["GMN-1000".to_string()])),
                Some(&course_rule)
            ),
            Badge::Partial("1/3".to_string())
        );

        let credits_rule =
            rule(r#"{"type":"credits","min":3,"max":9}"#, false);
        assert_eq!(
            incomplete_badge(
                &snapshot,
                &report(Some(vec!["GAE-1000".to_string()])),
                Some(&credits_rule)
            ),
            Badge::Partial("3/9 cr".to_string())
        );

        let credits_rule_en_sus =
            rule(r#"{"type":"credits","min":3,"max":9}"#, true);
        assert_eq!(
            incomplete_badge(
                &snapshot,
                &report(None),
                Some(&credits_rule_en_sus)
            ),
            Badge::Missing("0/9 cr - en sus".to_string())
        );
    }

    #[test]
    fn a_satisfied_rule_without_a_constraint_keeps_a_bare_check() {
        let snapshot = snapshot();
        let report = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Satisfied,
            counted: Some(vec!["GMN-1000".to_string()]),
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        assert_eq!(
            rule_badge(&snapshot, &report, None),
            Badge::Ok("✓".to_string())
        );
    }

    #[test]
    fn a_satisfied_constrained_rule_shows_its_fraction() {
        let snapshot = snapshot();
        let report = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Satisfied,
            counted: Some(vec!["GAE-1000".to_string()]),
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        let en_sus_rule: Rule = serde_json::from_str(
            r#"{"title":"R","constraint":{"type":"credits","min":1,"max":8},
                 "courses":["GAE-1000"],"credits_in_addition":true}"#,
        )
        .unwrap_or_else(|e| panic!("rule literal: {e}"));
        assert_eq!(
            rule_badge(&snapshot, &report, Some(&en_sus_rule)),
            Badge::Ok("✓ 3/8 cr - en sus".to_string())
        );

        // min ≠ max, numerator strictly between the two: a « 3–9 cr »
        // rule satisfied at 6 cr must not round up to the min nor clamp
        // to the max
        let open_report = ulaval_scheduler_core::RuleReport {
            counted: Some(vec![
                "GAE-1000".to_string(),
                "GMN-1000".to_string(),
            ]),
            ..report
        };
        let open_rule: Rule = serde_json::from_str(
            r#"{"title":"R","constraint":{"type":"credits","min":3,"max":9},
                 "courses":["GAE-1000","GMN-1000"]}"#,
        )
        .unwrap_or_else(|e| panic!("rule literal: {e}"));
        assert_eq!(
            rule_badge(&snapshot, &open_report, Some(&open_rule)),
            Badge::Ok("✓ 6/9 cr".to_string())
        );
    }

    // `core` refuses a count over the maximum before it ever reaches a
    // `Satisfied` report (`CoverageError::CountOverMax`/`CreditsOverMax`),
    // so this exact input never occurs today; the badge must still not
    // clamp on its own, so a future relaxation over there does not get
    // silently hidden here
    #[test]
    fn a_satisfied_rule_over_its_maximum_keeps_the_true_numerator() {
        let snapshot = snapshot();
        let rule: Rule = serde_json::from_str(
            r#"{"title":"R","constraint":{"type":"credits","min":1,"max":6},
                 "courses":["GAE-1000","GMN-1000","ANL-2020"]}"#,
        )
        .unwrap_or_else(|e| panic!("rule literal: {e}"));
        let report = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "R".to_string(),
            status: RuleStatus::Satisfied,
            counted: Some(vec![
                "GAE-1000".to_string(),
                "GMN-1000".to_string(),
                "ANL-2020".to_string(),
            ]),
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        assert_eq!(
            rule_badge(&snapshot, &report, Some(&rule)),
            Badge::Ok("✓ 9/6 cr".to_string())
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
                manual: None,
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
    fn a_second_rule_of_the_same_scope_shows_its_codes_counted_elsewhere() {
        // one catalogued course (GEX-1000), one absent from the catalogue
        // (GHOST-1) and one hand-entered (ZZZ-9000, added below) — all
        // three listed by both rules of the program scope
        let courses = r#"{"courses":[
          {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        let program = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
          "title":"Baccalauréat en génie des eaux","cycle":1,
          "credits_required":9,"mandatory":[],
          "rules":[
            {"title":"Règle 1",
             "constraint":{"type":"course","min":3,"max":3},
             "courses":["GEX-1000","GHOST-1","ZZZ-9000"]},
            {"title":"Règle 2",
             "constraint":{"type":"course","min":3,"max":3},
             "courses":["GEX-1000","GHOST-1","ZZZ-9000"]}
          ],
          "concentrations":[],"profiles":[]}"#;
        let mut snapshot = parse_data(
            &RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    program.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let manual = crate::data::build_manual_course(
            &crate::data::ManualDraft {
                code: "ZZZ-9000".to_string(),
                title: "Cours manuel".to_string(),
                credits: "3".to_string(),
                nrc: String::new(),
                slots: Vec::new(),
            },
            Season::Fall,
            2026,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        crate::data::add_manual_course(&mut snapshot, manual)
            .unwrap_or_else(|e| panic!("{e}"));
        let plan = Plan {
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            displayed_placement: std::collections::BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GHOST-1".to_string(), 1),
                ("ZZZ-9000".to_string(), 1),
            ]),
            ..Plan::default()
        };
        let model = panel_model(&snapshot, &plan);

        // Règle 1 claims all three: satisfied, shown placed normally
        assert_eq!(model.rules[0].badge, Badge::Ok("✓ 3/3".to_string()));
        let rule1_gex = model.rules[0]
            .rows
            .iter()
            .find(|row| row.code == "GEX-1000")
            .expect("Règle 1 lists it");
        assert_eq!(rule1_gex.state, RowState::Placed);

        // Règle 2: every code already claimed, nothing left to count
        assert_eq!(model.rules[1].badge, Badge::Missing("0/3".to_string()));

        // (a) a catalogued, placed course keeps its placement text and
        // gains where it counts instead, appended rather than replacing it
        let gex = model.rules[1]
            .rows
            .iter()
            .find(|row| row.code == "GEX-1000")
            .expect("Règle 2 lists it too");
        assert_eq!(gex.state, RowState::CountedElsewhere);
        assert!(gex.sub.starts_with("placé en "), "{}", gex.sub);
        assert!(
            gex.sub.ends_with(" - compté dans la Règle 1"),
            "{}",
            gex.sub
        );

        // (b) a code absent from the catalogue stays Unknown — the guard
        // never invents an owner for a row that was never actionable
        let ghost = model.rules[1]
            .rows
            .iter()
            .find(|row| row.code == "GHOST-1")
            .expect("Règle 2 lists it too");
        assert_eq!(ghost.state, RowState::Unknown);
        assert_eq!(ghost.sub, "absent du catalogue");

        // (c) a manual course keeps its flag alongside the new sub-text
        let manual_row = model.rules[1]
            .rows
            .iter()
            .find(|row| row.code == "ZZZ-9000")
            .expect("Règle 2 lists it too");
        assert_eq!(manual_row.state, RowState::CountedElsewhere);
        assert!(manual_row.sub.contains(" - manuel"), "{}", manual_row.sub);
        assert!(
            manual_row.sub.ends_with(" - compté dans la Règle 1"),
            "{}",
            manual_row.sub
        );
    }

    // core's own invariant guarantees `elsewhere` never names a code no
    // rule of the same scope claimed — this exercises the defensive
    // fallback directly, since a real report can never reach it
    #[test]
    fn an_elsewhere_code_with_no_owner_in_scope_is_left_untouched() {
        let placed = Row {
            code: "GEX-1000".to_string(),
            title: "Hydrologie".to_string(),
            credits: "3 cr".to_string(),
            sub: "placé en A1-A26".to_string(),
            state: RowState::Placed,
            assumed: Vec::new(),
        };
        let report = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "Règle 2".to_string(),
            status: RuleStatus::Incomplete,
            counted: Some(Vec::new()),
            elsewhere: vec!["GEX-1000".to_string()],
            missing: None,
            candidates: None,
            raw: None,
        };
        // no other report of this scope claims GEX-1000 in `counted`
        let all = vec![report.clone()];
        let rows = mark_counted_elsewhere(vec![placed.clone()], &report, &all);
        assert_eq!(
            rows,
            vec![placed],
            "left exactly as built, no text invented"
        );
    }

    // an Acquired row must never be promoted to CountedElsewhere: the view
    // grants Acquired no controls, and the entente strip a CountedElsewhere
    // row keeps would be a control the row never had a right to
    #[test]
    fn an_acquired_row_is_left_untouched_even_when_listed_elsewhere() {
        let acquired = Row {
            code: "MAT-0130".to_string(),
            title: "Mathématiques".to_string(),
            credits: "3 cr".to_string(),
            sub: "considéré comme déjà fait - décochez la case pour le \
                  placer"
                .to_string(),
            state: RowState::Acquired,
            assumed: Vec::new(),
        };
        let owner = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "Règle 1".to_string(),
            status: RuleStatus::Satisfied,
            counted: Some(vec!["MAT-0130".to_string()]),
            elsewhere: Vec::new(),
            missing: None,
            candidates: None,
            raw: None,
        };
        let report = ulaval_scheduler_core::RuleReport {
            scope: Scope::Program,
            title: "Règle 2".to_string(),
            status: RuleStatus::Incomplete,
            counted: Some(Vec::new()),
            elsewhere: vec!["MAT-0130".to_string()],
            missing: None,
            candidates: None,
            raw: None,
        };
        let all = vec![owner, report.clone()];
        let rows =
            mark_counted_elsewhere(vec![acquired.clone()], &report, &all);
        assert_eq!(rows, vec![acquired], "state and sub both untouched");
    }

    // GCI-4201 is listed both by the program's own "Règle 2" and by the
    // concentration's "Règle 1"/"Règle 2" — `report.rules` is ordered
    // programme → concentration → profil, so an owner search with no scope
    // filter lands on the program's "Règle 2" (the first report in that
    // order whose `counted` contains the code) instead of the
    // concentration's own "Règle 1", which is the rule that actually
    // claimed it within the concentration's scope (like B-GCI, whose
    // program rules and concentration rules both carry GCI-4201-shaped
    // overlaps independently).
    #[test]
    fn the_owner_search_stays_within_the_reporting_rules_own_scope() {
        let courses = r#"{"courses":[
          {"code":"GCI-4201","title":"Hydraulique urbaine","credits":3,
           "cycle":1,"prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}},
          {"code":"GMN-2901","title":"Non sélectionné","credits":3,
           "cycle":1,"prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        let program = r#"{"code":"B-GCI","slug":"gci","semester":"A26",
          "title":"Baccalauréat en génie civil","cycle":1,
          "credits_required":6,"mandatory":[],
          "rules":[
            {"title":"Règle 1",
             "constraint":{"type":"course","min":1,"max":1},
             "courses":["GMN-2901"]},
            {"title":"Règle 2",
             "constraint":{"type":"course","min":1,"max":1},
             "courses":["GCI-4201"]}
          ],
          "concentrations":[{"title":"Eau et environnement",
            "credits_required":null,"mandatory":[],
            "rules":[
              {"title":"Règle 1",
               "constraint":{"type":"course","min":1,"max":2},
               "courses":["GCI-4201","GMN-2901"]},
              {"title":"Règle 2",
               "constraint":{"type":"course","min":1,"max":1},
               "courses":["GCI-4201"]}
            ],"notes":[]}],
          "profiles":[]}"#;
        let snapshot = parse_data(
            &RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: vec![(
                    "B-GCI-A26.json".to_string(),
                    program.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let plan = Plan {
            program: Some(ProgramChoice {
                code: "B-GCI".to_string(),
                semester: "A26".to_string(),
                concentration: Some("Eau et environnement".to_string()),
                profile: None,
            }),
            displayed_placement: std::collections::BTreeMap::from([(
                "GCI-4201".to_string(),
                1,
            )]),
            ..Plan::default()
        };
        let model = panel_model(&snapshot, &plan);

        let concentration_rule_2 = model
            .rules
            .iter()
            .find(|section| section.key == "c/Règle 2")
            .expect("concentration Règle 2 renders");
        let row = concentration_rule_2
            .rows
            .iter()
            .find(|row| row.code == "GCI-4201")
            .expect("Règle 2 lists it");
        assert_eq!(row.state, RowState::CountedElsewhere);
        assert!(
            row.sub.contains("compté dans la Règle 1"),
            "must name the concentration's own Règle 1, not the \
             program's same-numbered rule: {}",
            row.sub
        );
        assert!(
            !row.sub.contains("Règle 2"),
            "the program's Règle 2 also counts GCI-4201 in its own scope \
             — an unscoped search would wrongly attribute it there: {}",
            row.sub
        );
    }

    // a crédité or entente course listed by two rules of the same scope
    // must keep saying so once demoted to CountedElsewhere — the sub text
    // is appended to, never replaced
    #[test]
    fn a_credited_course_keeps_its_credited_text_when_counted_elsewhere() {
        let courses = r#"{"courses":[
          {"code":"GEX-1000","title":"Hydrologie","credits":3,"cycle":1,
           "prerequisites":null,"equivalents":[],
           "seasons":{"fall":{"last_offered":2026,"options":null}}}
        ]}"#;
        let program = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
          "title":"Baccalauréat en génie des eaux","cycle":1,
          "credits_required":6,"mandatory":[],
          "rules":[
            {"title":"Règle 1",
             "constraint":{"type":"course","min":1,"max":1},
             "courses":["GEX-1000"]},
            {"title":"Règle 2",
             "constraint":{"type":"course","min":1,"max":1},
             "courses":["GEX-1000"]}
          ],
          "concentrations":[],"profiles":[]}"#;
        let snapshot = parse_data(
            &RawData {
                courses: courses.to_string(),
                meta: Some(r#"{"scraped_at":null}"#.to_string()),
                manual: None,
                programs: vec![(
                    "B-GEX-A26.json".to_string(),
                    program.to_string(),
                )],
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let plan = Plan {
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            credited: std::collections::BTreeSet::from([
                "GEX-1000".to_string()
            ]),
            ..Plan::default()
        };
        let model = panel_model(&snapshot, &plan);

        let row = model.rules[1]
            .rows
            .iter()
            .find(|row| row.code == "GEX-1000")
            .expect("Règle 2 lists it too");
        assert_eq!(row.state, RowState::CountedElsewhere);
        assert!(row.sub.contains("crédité"), "{}", row.sub);
        assert!(row.sub.contains("compté dans la Règle 1"), "{}", row.sub);
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
        let (granted, warnings) = granted_program(
            program,
            Some("Génie urbain"),
            Some("Profil international"),
            &grants,
        );
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
            Badge::Partial("3/9 cr".to_string()),
            "GEX-1000 (placé) compte maintenant dans la Règle 2"
        );
        let entente = &model.rules[1].rows[3];
        assert_eq!(entente.code, "GEX-1000");
        assert!(entente.sub.ends_with("- entente"), "{}", entente.sub);
    }

    #[test]
    fn a_grant_only_strips_the_rules_of_its_own_scope() {
        // one course listed by a program rule, two concentration rules and a
        // profile rule; a grant into one concentration rule must not touch
        // the program's or the profile's own lists (ADR
        // `2026-08-un-cours-compte-dans-une-seule-regle-par-portee`)
        let program: Program = serde_json::from_str(
            r#"{"code":"X","slug":"x","semester":"A26","title":"X",
                "cycle":1,"credits_required":6,"mandatory":[],
                "rules":[{"title":"Règle P",
                          "constraint":{"type":"course","min":1,"max":1},
                          "courses":["SHARED-1000"]}],
                "concentrations":[{"title":"Concentration X",
                  "credits_required":null,"mandatory":[],
                  "rules":[
                    {"title":"Règle C1",
                     "constraint":{"type":"course","min":1,"max":1},
                     "courses":["SHARED-1000"]},
                    {"title":"Règle C2",
                     "constraint":{"type":"course","min":1,"max":1},
                     "courses":["SHARED-1000"]}
                  ],"notes":[]}],
                "profiles":[{"title":"Profil Y","credits_required":null,
                  "mandatory":[],
                  "rules":[{"title":"Règle F1",
                            "constraint":{"type":"course","min":1,"max":1},
                            "courses":["SHARED-1000"]}],
                  "notes":[]}]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let grants = std::collections::BTreeMap::from([(
            "SHARED-1000".to_string(),
            "c/Règle C2".to_string(),
        )]);
        let (granted, warnings) = granted_program(
            &program,
            Some("Concentration X"),
            Some("Profil Y"),
            &grants,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        let c1 = &granted.concentrations[0].rules[0];
        assert_eq!(c1.title, "Règle C1");
        assert!(
            matches!(&c1.courses, RuleCourses::List { courses }
                if !courses.contains(&"SHARED-1000".to_string())),
            "the other rule of the same scope loses it: {:?}",
            c1.courses
        );
        let c2 = &granted.concentrations[0].rules[1];
        assert_eq!(c2.title, "Règle C2");
        assert!(matches!(&c2.courses, RuleCourses::List { courses }
            if courses.contains(&"SHARED-1000".to_string())));
        let f1 = &granted.profiles[0].rules[0];
        assert!(
            matches!(&f1.courses, RuleCourses::List { courses }
                if courses.contains(&"SHARED-1000".to_string())),
            "the profile's own scope is untouched by a concentration grant: {:?}",
            f1.courses
        );
        let p1 = &granted.rules[0];
        assert!(
            matches!(&p1.courses, RuleCourses::List { courses }
                if courses.contains(&"SHARED-1000".to_string())),
            "the programme's own scope is untouched by a concentration grant: {:?}",
            p1.courses
        );
    }

    #[test]
    fn an_inapplicable_grant_is_named_never_dropped() {
        let snapshot = snapshot();
        let program = &snapshot.programs[0];
        let grants = std::collections::BTreeMap::from([
            ("AAA-1000".to_string(), "p/Règle fantôme".to_string()),
            ("BBB-1000".to_string(), "sans-slash".to_string()),
            // Règle 4 is raw-only: no list can host the course
            ("CCC-1000".to_string(), "p/Règle 4".to_string()),
            ("DDD-1000".to_string(), "x/Règle 1".to_string()),
        ]);
        let (granted, warnings) =
            granted_program(program, None, None, &grants);
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings[0].contains("Règle fantôme"), "{}", warnings[0]);
        assert!(warnings[1].contains("sans-slash"), "{}", warnings[1]);
        assert!(warnings[2].contains("Règle 4"), "{}", warnings[2]);
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
    fn grantable_rules_offer_lists_and_keyword_rules_across_scopes() {
        let snapshot = snapshot();
        let rules = grantable_rules(
            &snapshot.programs[0],
            Some("Génie urbain"),
            Some("Profil international"),
        );
        let keys: Vec<&str> =
            rules.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "p/Règle 1",
                "p/Règle 2",
                "p/Règle 3",
                "c/Règle C1",
                "f/Règle P1"
            ],
            "« any » (Règle 3) is a target now; raw (Règle 4) stays out"
        );
        assert_eq!(rules[0].1, "Programme — Règle 1");
        assert_eq!(rules[3].1, "Concentration « Génie urbain » — Règle C1");
        assert_eq!(rules[4].1, "Profil « Profil international » — Règle P1");
        assert_eq!(
            rules
                .iter()
                .map(|(_, label)| label)
                .collect::<BTreeSet<_>>()
                .len(),
            rules.len(),
            "every visible agreement option is unambiguous"
        );

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
        assert!(grantable_rules(&with_preparatory, None, None).is_empty());
    }

    #[test]
    fn grantable_rules_offer_only_the_chosen_blocks() {
        let snapshot = snapshot();
        let rules = grantable_rules(&snapshot.programs[0], None, None);
        let keys: Vec<&str> =
            rules.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            ["p/Règle 1", "p/Règle 2", "p/Règle 3"],
            "no block chosen, no scoped target"
        );
    }

    #[test]
    fn a_scoped_grant_needs_its_block_chosen() {
        // the same key with no concentration chosen resolves nowhere: the
        // entente is named as inapplicable, never landed in another block
        let snapshot = snapshot();
        let grants = std::collections::BTreeMap::from([(
            "XYZ-3000".to_string(),
            "c/Règle C1".to_string(),
        )]);
        let (granted, warnings) =
            granted_program(&snapshot.programs[0], None, None, &grants);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("Règle C1"), "{}", warnings[0]);
        assert_eq!(
            granted.concentrations, snapshot.programs[0].concentrations,
            "untouched"
        );
    }

    #[test]
    fn cheminement_choices_offer_the_blocks_and_carry_the_choice() {
        let snapshot = snapshot();
        let mut plan = plan();
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
        }
        let choices = cheminement_choices(&snapshot, &plan)
            .expect("the program offers blocks");
        assert_eq!(choices.concentrations, ["Génie urbain"]);
        assert_eq!(choices.profiles, ["Profil international"]);
        assert_eq!(choices.concentration.as_deref(), Some("Génie urbain"));
        assert_eq!(choices.profile, None);
        assert!(
            choices.offers_none,
            "no neutral block scraped: « Aucune » stays offered"
        );
        // a scraped neutral block replaces the synthetic « Aucune » — the
        // option would sous-compter its own rule (décision 2026-08-19)
        let mut neutral = snapshot.clone();
        neutral.programs[0].concentrations[0].title =
            "Cheminement sans concentration".to_string();
        let choices =
            cheminement_choices(&neutral, &plan).expect("still offers blocks");
        assert!(!choices.offers_none);
        neutral.programs[0].concentrations[0].title =
            "Approche généraliste".to_string();
        let choices =
            cheminement_choices(&neutral, &plan).expect("still offers blocks");
        assert!(!choices.offers_none, "B-GIN's wording counts too");
        assert!(
            cheminement_choices(&snapshot, &Plan::default()).is_none(),
            "no program, no row"
        );
        // a program offering neither block has no row either (M-GEX)
        let mut bare = snapshot;
        bare.programs[0].concentrations.clear();
        bare.programs[0].profiles.clear();
        assert!(cheminement_choices(&bare, &plan).is_none());
    }

    #[test]
    fn the_default_concentration_is_the_pages_first() {
        let snapshot = snapshot();
        assert_eq!(
            default_concentration(&snapshot, "B-GEX", "A26").as_deref(),
            Some("Génie urbain")
        );
        assert!(
            default_concentration(&snapshot, "B-GEX", "H99").is_none(),
            "an unknown vintage imposes nothing"
        );
        let mut bare = snapshot;
        bare.programs[0].concentrations.clear();
        assert!(
            default_concentration(&bare, "B-GEX", "A26").is_none(),
            "no concentration on the page, none imposed (B-GEX)"
        );
    }

    #[test]
    fn the_subtitle_names_the_chosen_program_concentration_and_profile() {
        let snapshot = snapshot();
        let mut plan = plan();
        assert_eq!(
            program_subtitle(&snapshot, &plan).as_deref(),
            Some("Baccalauréat en génie des eaux (B-GEX version A26)")
        );
        if let Some(choice) = plan.program.as_mut() {
            choice.concentration = Some("Génie urbain".to_string());
            choice.profile = Some("Profil international".to_string());
        }
        assert_eq!(
            program_subtitle(&snapshot, &plan).as_deref(),
            Some(
                "Baccalauréat en génie des eaux (B-GEX version A26) — \
                 Génie urbain — Profil international"
            )
        );
        assert!(program_subtitle(&snapshot, &Plan::default()).is_none());
    }

    #[test]
    fn subjects_count_the_catalogue_by_prefix() {
        let subjects = subjects(&snapshot());
        assert!(subjects.contains(&("GEX".to_string(), 4)));
        assert!(subjects.contains(&("ANL".to_string(), 1)));
        assert_eq!(subject_of("SANS-TIRET"), "SANS");
        assert_eq!(subject_of("BRUT"), "BRUT");
    }

    // a program snapshot reduced to what a picker row shows
    fn bare_program(
        code: &str,
        semester: &str,
        title: &str,
        credits: i64,
    ) -> (String, String) {
        (
            format!("{code}-{semester}.json"),
            format!(
                r#"{{"code":"{code}","slug":"x","semester":"{semester}",
                    "title":"{title}","cycle":1,
                    "credits_required":{credits},"mandatory":[],
                    "rules":[],"concentrations":[],"profiles":[]}}"#
            ),
        )
    }

    fn snapshot_of(programs: Vec<(String, String)>) -> Snapshot {
        parse_data(
            &RawData {
                courses: COURSES.to_string(),
                meta: None,
                manual: None,
                programs,
            },
            Vec::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_catalogue_without_a_program_offers_no_picker_row() {
        assert!(program_vintages(&snapshot_of(Vec::new())).is_empty());
    }

    #[test]
    fn one_vintage_is_one_row_carrying_it_alone() {
        let rows = program_vintages(&snapshot());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, "B-GEX");
        assert_eq!(rows[0].title, "Baccalauréat en génie des eaux");
        assert_eq!(rows[0].credits_required, 120);
        assert_eq!(rows[0].vintages, vec!["A26".to_string()]);
    }

    // the whole point: ten B-GMC files are one row, and the select they
    // feed runs newest first — « A26, H26, A25 », never the « A25, A26,
    // H26 » the spelling would give
    #[test]
    fn vintages_of_one_code_group_into_one_row_newest_first() {
        let rows = program_vintages(&snapshot_of(vec![
            bare_program("B-GMC", "A25", "Génie mécanique", 120),
            bare_program("B-GMC", "H26", "Génie mécanique", 120),
            bare_program("B-GMC", "A26", "Génie mécanique", 120),
        ]));
        assert_eq!(rows.len(), 1, "one row, not three");
        assert_eq!(rows[0].vintages, vec!["A26", "H26", "A25"]);
    }

    // a divergence between vintages is announced by the one preselected
    #[test]
    fn the_row_announces_the_newest_vintages_title_and_credits() {
        let rows = program_vintages(&snapshot_of(vec![
            bare_program("B-GIN", "A24", "Ancien titre", 117),
            bare_program("B-GIN", "H27", "Titre courant", 120),
        ]));
        assert_eq!(rows[0].title, "Titre courant");
        assert_eq!(rows[0].credits_required, 120);
        assert_eq!(rows[0].vintages, vec!["H27", "A24"]);
    }

    #[test]
    fn several_codes_keep_the_snapshots_order() {
        let rows = program_vintages(&snapshot_of(vec![
            bare_program("M-GEX", "A26", "Maîtrise", 45),
            bare_program("B-ANT", "A26", "Anthropologie", 90),
            bare_program("B-GMC", "H27", "Génie mécanique", 120),
        ]));
        let codes: Vec<&str> =
            rows.iter().map(|row| row.code.as_str()).collect();
        assert_eq!(codes, vec!["B-ANT", "B-GMC", "M-GEX"]);
    }
}
