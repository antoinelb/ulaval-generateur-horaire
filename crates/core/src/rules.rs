use std::collections::{BTreeMap, BTreeSet};

use crate::course::Course;
use crate::program::{
    Constraint, LanguageRequirement, Program, Rule, RuleCourses, RuleReference,
};

// The rules-coverage report the UI renders (jalon 8 product API, ADR
// `2026-07-schema-du-rapport-de-couverture-en-fixtures`): per scope —
// program, plus the chosen concentration and profile — mandatory courses
// split satisfied/missing and every rule reported. `candidates` are the
// rule's list minus the selection, deliberately *not* filtered by weekly
// feasibility: this is the accounting layer, the composition with A comes
// with its own input shape later.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct CoverageReport {
    pub mandatory: Vec<MandatoryReport>,
    pub rules: Vec<RuleReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_requirement: Option<LanguageReport>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct MandatoryReport {
    pub scope: Scope,
    pub satisfied: Vec<String>,
    pub missing: Vec<String>,
}

// One rule's verdict. The evaluated fields (`counted`, `candidates`) exist
// exactly when the rule could be counted; a `reported` rule carries its
// `raw` text instead — surfaced to the student, never invented (ADRs
// `2026-07-contrainte-de-regle-optionnelle`,
// `2026-07-regles-negociees-reconnues`). `elsewhere` lists codes this rule
// also lists but that a *previous* evaluated rule of the same scope already
// counted — shown to the student so the course doesn't look forgotten, but
// excluded from `counted`/`candidates` because it must not count twice
// (decision d'Antoine 2026-08-23, ADR
// `2026-08-un-cours-compte-dans-une-seule-regle-par-portee`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct RuleReport {
    pub scope: Scope,
    pub title: String,
    pub status: RuleStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counted: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elsewhere: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Missing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    // why this rule alone could not be counted — `Uncounted` carries one,
    // every other status none (ADR `2026-08-depassement-de-regle-en-statut-rouge`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defect: Option<RuleDefect>,
}

// A defect that stops *one* rule from being counted, and stops nothing
// else. It rides in the report rather than aborting it: a rule the data
// broke must not blank the nineteen rules beside it (AIR ERR-5).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum RuleDefect {
    // a credits rule counted a code no `Course` carries: the sum is
    // unknowable, and inventing one would misreport a graduation
    MissingCourse {
        code: String,
    },
    // « tous les cours de la Règle N du cheminement X » where the chase
    // fails — unknown concentration, unknown rule, or a target that is not
    // a plain list. All three read the same to a student, so they are one.
    BrokenReference {
        concentration: String,
        target: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Program,
    Concentration,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
// snake_case, not lowercase: the three original one-word variants
// serialize identically either way, and `OverMax` needs the underscore
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Satisfied,
    Incomplete,
    Reported,
    // more courses than the rule's maximum admits. A violation, not a
    // surplus: the arbitration the ADR
    // `2026-07-somme-au-dessus-du-max-en-erreur-typee` was waiting for came
    // down on 2026-08-30 (ADR
    // `2026-08-depassement-de-regle-en-statut-rouge`). `counted` keeps every
    // code, so the view can say « 15/12 cr » — the excess is the point.
    OverMax,
    // the rule carries a `RuleDefect`: no verdict is possible for it alone
    Uncounted,
}

// mirror of `Constraint`: what remains to reach the count or the minimum
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(untagged)]
pub enum Missing {
    Count { count: i64 },
    Credits { credits: i64 },
}

// The language requirement is never « missing » : a placement-test score
// can dispense from the course and core cannot see it, so the only verdicts
// are satisfied (a branch's course is selected) or reported.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct LanguageReport {
    pub status: LanguageStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "lowercase")]
pub enum LanguageStatus {
    Satisfied,
    Reported,
}

// one scope's block: its mandatory list and its rules
type ScopeBlock<'a> = (Scope, &'a [String], &'a [Rule]);

// The only inputs that leave *no* scope to report on. Everything a single
// rule can get wrong is a `RuleDefect` inside the report instead, so one
// broken rule never costs the student the other nineteen (AIR ERR-5, ADR
// `2026-08-depassement-de-regle-en-statut-rouge`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoverageError {
    #[error("no concentration titled « {title} » in the program")]
    UnknownConcentration { title: String },
    #[error("no profile titled « {title} » in the program")]
    UnknownProfile { title: String },
}

// The pure function the UI calls on every selection change. `selection`
// holds the chosen codes; `courses` carries the Course of every selected
// course a credits rule must count (a Range counts its lower bound, ADR
// `2026-07-credits-range-borne-basse-en-planification`).
pub fn coverage_report(
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
    selection: &BTreeSet<String>,
    courses: &[Course],
) -> Result<CoverageReport, CoverageError> {
    let credits: BTreeMap<&str, u32> = courses
        .iter()
        .map(|course| (course.code.as_str(), course.credits.planning()))
        .collect();
    let scopes = resolve_scopes(program, concentration, profile)?;
    Ok(CoverageReport {
        mandatory: scopes
            .iter()
            .map(|(scope, mandatory, _)| {
                mandatory_report(*scope, mandatory, selection)
            })
            .collect(),
        rules: scopes
            .iter()
            .flat_map(|(scope, _, rules)| {
                scope_reports(*scope, rules, program, selection, &credits)
            })
            .collect(),
        language_requirement: program
            .language_requirement
            .as_ref()
            .map(|requirement| language_report(requirement, selection)),
    })
}

// one scope's rules, in order, each attributed against what earlier rules
// of *this same scope* already claimed — the accumulator starts empty per
// scope so a course counts once in the concentration and once in the
// profile (decision d'Antoine 2026-08-23). Only a constrained rule's
// `counted` claims codes: an unconstrained list (« Scolarité préparatoire »)
// never removes a course from a later rule's count.
fn scope_reports(
    scope: Scope,
    rules: &[Rule],
    program: &Program,
    selection: &BTreeSet<String>,
    credits: &BTreeMap<&str, u32>,
) -> Vec<RuleReport> {
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    rules
        .iter()
        .map(|rule| {
            let report = rule_report(
                scope, rule, program, selection, credits, &claimed,
            );
            if rule.constraint.is_some() {
                claimed.extend(report.counted.iter().flatten().cloned());
            }
            report
        })
        .collect()
}

// the program scope always, plus each chosen block found by its title — an
// unknown title is the student's typo, surfaced
fn resolve_scopes<'a>(
    program: &'a Program,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> Result<Vec<ScopeBlock<'a>>, CoverageError> {
    let mut scopes: Vec<ScopeBlock> =
        vec![(Scope::Program, &program.mandatory, &program.rules)];
    if let Some(title) = concentration {
        let block = program.concentration(title).ok_or_else(|| {
            CoverageError::UnknownConcentration {
                title: title.to_string(),
            }
        })?;
        scopes.push((Scope::Concentration, &block.mandatory, &block.rules));
    }
    if let Some(title) = profile {
        let block = program.profile(title).ok_or_else(|| {
            CoverageError::UnknownProfile {
                title: title.to_string(),
            }
        })?;
        scopes.push((Scope::Profile, &block.mandatory, &block.rules));
    }
    Ok(scopes)
}

fn mandatory_report(
    scope: Scope,
    mandatory: &[String],
    selection: &BTreeSet<String>,
) -> MandatoryReport {
    // set semantics: a code listed twice is one course to pass, and the
    // BTreeSet iterates sorted, matching the frozen fixtures
    let unique: BTreeSet<&String> = mandatory.iter().collect();
    MandatoryReport {
        scope,
        satisfied: unique
            .iter()
            .filter(|code| selection.contains(**code))
            .map(|code| (*code).clone())
            .collect(),
        missing: unique
            .iter()
            .filter(|code| !selection.contains(**code))
            .map(|code| (*code).clone())
            .collect(),
    }
}

fn rule_report(
    scope: Scope,
    rule: &Rule,
    program: &Program,
    selection: &BTreeSet<String>,
    credits: &BTreeMap<&str, u32>,
    claimed: &BTreeSet<String>,
) -> RuleReport {
    // a reference is resolved before the constraint check: a broken
    // reference marks this rule uncounted even when it would only be
    // reported — but it marks nothing beyond it
    let listed = match resolved_rule_courses(program, rule) {
        Ok(listed) => listed,
        Err(defect) => return uncounted(scope, rule, defect),
    };
    match (listed, &rule.constraint) {
        (Some(listed), Some(constraint)) => evaluated(
            scope, rule, listed, constraint, selection, credits, claimed,
        ),
        // a list naming no number — « Scolarité préparatoire » : nothing to
        // verdict, but the split is still shown (ADR
        // `2026-08-regle-sans-contrainte-comptee-mais-reportee`)
        (Some(listed), None) => {
            listed_reported(scope, rule, listed, selection)
        }
        // Keyword (any/negotiated) or raw-only
        (None, _) => reported(scope, rule),
    }
}

// a rule whose own data defeated the count: named, never silently dropped
fn uncounted(scope: Scope, rule: &Rule, defect: RuleDefect) -> RuleReport {
    RuleReport {
        scope,
        title: rule.title.clone(),
        status: RuleStatus::Uncounted,
        counted: None,
        elsewhere: Vec::new(),
        missing: None,
        candidates: None,
        raw: rule_raw(rule).map(str::to_string),
        defect: Some(defect),
    }
}

pub fn resolved_rule_courses<'a>(
    program: &'a Program,
    rule: &'a Rule,
) -> Result<Option<&'a [String]>, RuleDefect> {
    match &rule.courses {
        RuleCourses::List { courses } => Ok(Some(courses)),
        RuleCourses::Reference { courses, .. } => {
            resolve_reference(courses, program).map(Some)
        }
        RuleCourses::Keyword { .. } | RuleCourses::Raw { .. } => Ok(None),
    }
}

// « tous les cours de la Règle N du cheminement X » : both titles come from
// the same scraped page. The three ways the chase can fail — no such
// concentration, no such rule, a target that is not a plain list — are one
// `BrokenReference` because they read alike to whoever sees the rule.
fn resolve_reference<'a>(
    reference: &RuleReference,
    program: &'a Program,
) -> Result<&'a [String], RuleDefect> {
    let broken = || RuleDefect::BrokenReference {
        concentration: reference.concentration.clone(),
        target: reference.rule.clone(),
    };
    let concentration = program
        .concentrations
        .iter()
        .find(|block| block.title == reference.concentration)
        .ok_or_else(broken)?;
    let target = concentration
        .rules
        .iter()
        .find(|target| target.title == reference.rule)
        .ok_or_else(broken)?;
    match &target.courses {
        RuleCourses::List { courses } => Ok(courses),
        _ => Err(broken()),
    }
}

fn evaluated(
    scope: Scope,
    rule: &Rule,
    listed: &[String],
    constraint: &Constraint,
    selection: &BTreeSet<String>,
    credits: &BTreeMap<&str, u32>,
    claimed: &BTreeSet<String>,
) -> RuleReport {
    let (counted, candidates) = split_selection(listed, selection);
    // a code an earlier rule of this scope already claimed no longer counts
    // here — shown as `elsewhere` instead so the student sees it, but the
    // verdict is computed on the reduced set (that is the whole point)
    let (elsewhere, counted): (Vec<String>, Vec<String>) =
        counted.into_iter().partition(|code| claimed.contains(code));
    let (status, missing, defect) = verdict(constraint, &counted, credits);
    RuleReport {
        scope,
        title: rule.title.clone(),
        status,
        counted: Some(counted),
        elsewhere,
        missing,
        candidates: Some(candidates),
        raw: None,
        defect,
    }
}

// the same set split as an evaluated rule, but no verdict: whether a listed
// course applies depends on facts core cannot see (the student's collegial
// record for the cours d'appoint), so the status stays reported — counted
// and candidates give the UI the remaining courses to surface
fn listed_reported(
    scope: Scope,
    rule: &Rule,
    listed: &[String],
    selection: &BTreeSet<String>,
) -> RuleReport {
    let (counted, candidates) = split_selection(listed, selection);
    RuleReport {
        scope,
        title: rule.title.clone(),
        status: RuleStatus::Reported,
        counted: Some(counted),
        elsewhere: Vec::new(),
        missing: None,
        candidates: Some(candidates),
        raw: rule_raw(rule).map(str::to_string),
        defect: None,
    }
}

// set semantics: the page duplicates codes across thematic subgroups
// (règle 4 GEX), and the sorted iteration matches the frozen fixtures
fn split_selection(
    listed: &[String],
    selection: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let unique: BTreeSet<&str> = listed.iter().map(String::as_str).collect();
    let counted = unique
        .iter()
        .filter(|code| selection.contains(**code))
        .map(|code| code.to_string())
        .collect();
    let candidates = unique
        .iter()
        .filter(|code| !selection.contains(**code))
        .map(|code| code.to_string())
        .collect();
    (counted, candidates)
}

// One rule's verdict, which never fails: over the maximum is a status the
// student can see and act on, not a refusal that costs him the report (ADR
// `2026-08-depassement-de-regle-en-statut-rouge`). Only a credits rule can
// come back uncounted, and only because a code carries no `Course`.
fn verdict(
    constraint: &Constraint,
    counted: &[String],
    credits: &BTreeMap<&str, u32>,
) -> (RuleStatus, Option<Missing>, Option<RuleDefect>) {
    match *constraint {
        Constraint::Course { min, max } => {
            let total = counted.len() as i64;
            let (status, missing) = over_or(
                total,
                min,
                max,
                Missing::Count { count: min - total },
            );
            (status, missing, None)
        }
        Constraint::Credits { min, max } => {
            let summed = counted.iter().try_fold(0i64, |acc, code| {
                credits
                    .get(code.as_str())
                    .map(|&value| acc + i64::from(value))
                    .ok_or_else(|| RuleDefect::MissingCourse {
                        code: code.clone(),
                    })
            });
            match summed {
                Err(defect) => (RuleStatus::Uncounted, None, Some(defect)),
                Ok(total) => {
                    let (status, missing) = over_or(
                        total,
                        min,
                        max,
                        Missing::Credits {
                            credits: min - total,
                        },
                    );
                    (status, missing, None)
                }
            }
        }
    }
}

// the shared three-way verdict, `shortfall` being what the caller's unit
// names as still missing when the total sits below the minimum
fn over_or(
    total: i64,
    min: i64,
    max: i64,
    shortfall: Missing,
) -> (RuleStatus, Option<Missing>) {
    if total > max {
        (RuleStatus::OverMax, None)
    } else if total >= min {
        (RuleStatus::Satisfied, None)
    } else {
        (RuleStatus::Incomplete, Some(shortfall))
    }
}

fn reported(scope: Scope, rule: &Rule) -> RuleReport {
    RuleReport {
        scope,
        title: rule.title.clone(),
        status: RuleStatus::Reported,
        counted: None,
        elsewhere: Vec::new(),
        missing: None,
        candidates: None,
        raw: rule_raw(rule).map(str::to_string),
        defect: None,
    }
}

// a plain list carries no source text; every other shape keeps its raw for
// the student to read
fn rule_raw(rule: &Rule) -> Option<&str> {
    match &rule.courses {
        RuleCourses::List { .. } => None,
        RuleCourses::Reference { raw, .. }
        | RuleCourses::Keyword { raw, .. }
        | RuleCourses::Raw { raw } => Some(raw),
    }
}

fn language_report(
    requirement: &LanguageRequirement,
    selection: &BTreeSet<String>,
) -> LanguageReport {
    let satisfied = std::iter::once(&requirement.francophone)
        .chain(requirement.non_francophone.as_ref())
        .any(|qualification| selection.contains(&qualification.course));
    LanguageReport {
        status: if satisfied {
            LanguageStatus::Satisfied
        } else {
            LanguageStatus::Reported
        },
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn program(body: &str) -> Program {
        serde_json::from_str(&format!(
            r#"{{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                 "credits_required":120,{body}}}"#
        ))
        .unwrap_or_else(|e| panic!("program literal: {e}"))
    }

    fn bare(rules: &str) -> Program {
        program(&format!(
            r#""mandatory":[],"rules":{rules},
               "concentrations":[],"profiles":[]"#
        ))
    }

    fn course(code: &str, credits: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":{credits},"cycle":1,
                 "prerequisites":null,"equivalents":[],"seasons":{{}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn selection(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    fn report(
        program: &Program,
        selected: &[&str],
        courses: &[Course],
    ) -> CoverageReport {
        coverage_report(program, None, None, &selection(selected), courses)
            .unwrap_or_else(|e| panic!("{e}"))
    }

    // --- mandatory: set semantics, sorted split ---

    #[test]
    fn mandatory_splits_satisfied_and_missing_sorted_and_deduped() {
        let program = program(
            r#""mandatory":["B-2","A-1","B-2","C-3"],"rules":[],
               "concentrations":[],"profiles":[]"#,
        );
        let coverage = report(&program, &["B-2"], &[]);
        assert_eq!(coverage.mandatory.len(), 1);
        assert_eq!(coverage.mandatory[0].scope, Scope::Program);
        assert_eq!(coverage.mandatory[0].satisfied, ["B-2"]);
        assert_eq!(coverage.mandatory[0].missing, ["A-1", "C-3"]);
    }

    // --- count rules ---

    #[test]
    fn a_count_rule_is_satisfied_at_the_count_and_lists_candidates() {
        let program = bare(
            r#"[{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},
                 "courses":["B-2","A-1","C-3"]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        let rule = &coverage.rules[0];
        assert_eq!(rule.status, RuleStatus::Satisfied);
        assert_eq!(rule.counted.as_deref(), Some(&["A-1".to_string()][..]));
        assert_eq!(rule.missing, None);
        assert_eq!(
            rule.candidates.as_deref(),
            Some(&["B-2".to_string(), "C-3".to_string()][..])
        );
    }

    #[test]
    fn a_count_rule_short_of_the_count_says_how_many_remain() {
        let program = bare(
            r#"[{"title":"Règle 1","constraint":{"type":"course","min":2,"max":2},
                 "courses":["A-1","B-2","C-3"]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        let rule = &coverage.rules[0];
        assert_eq!(rule.status, RuleStatus::Incomplete);
        assert_eq!(rule.missing, Some(Missing::Count { count: 1 }));
    }

    #[test]
    fn a_duplicated_listed_code_counts_once() {
        // règle 4 GEX lists DDU-2000 twice (thematic subgroups): one course
        let program = bare(
            r#"[{"title":"Règle 4","constraint":{"type":"course","min":2,"max":2},
                 "courses":["A-1","A-1","B-2"]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(
            coverage.rules[0].counted.as_deref(),
            Some(&["A-1".to_string()][..])
        );
        assert_eq!(coverage.rules[0].status, RuleStatus::Incomplete);
    }

    #[test]
    fn a_selection_above_the_course_max_is_the_rule_s_own_verdict() {
        // the course-count twin of the credits ceiling: a violation the
        // student can see and undo, not a refusal that costs him the report
        let program = bare(
            r#"[{"title":"Règle 1",
                 "constraint":{"type":"course","min":1,"max":1},
                 "courses":["A-1","B-2"]}]"#,
        );
        let coverage = report(&program, &["A-1", "B-2"], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::OverMax);
        assert_eq!(coverage.rules[0].defect, None);
        // every code stays counted: the excess is what the view shows
        assert_eq!(
            coverage.rules[0].counted.as_deref(),
            Some(&["A-1".to_string(), "B-2".to_string()][..])
        );
        assert_eq!(coverage.rules[0].missing, None);
    }

    // --- one course, one rule per scope ---

    #[test]
    fn a_course_listed_by_two_program_rules_counts_only_in_the_first() {
        // decision d'Antoine 2026-08-23: strictly the first evaluated rule
        // of the scope that lists the course, no overflow to a later one
        let program = bare(
            r#"[{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},
                 "courses":["X-1"]},
                {"title":"Règle 2","constraint":{"type":"course","min":1,"max":1},
                 "courses":["X-1","X-2"]}]"#,
        );
        let coverage = report(&program, &["X-1"], &[]);
        let r1 = &coverage.rules[0];
        assert_eq!(r1.counted.as_deref(), Some(&["X-1".to_string()][..]));
        assert!(r1.elsewhere.is_empty());
        assert_eq!(r1.status, RuleStatus::Satisfied);
        let r2 = &coverage.rules[1];
        assert_eq!(r2.counted.as_deref(), Some(&[][..]));
        assert_eq!(r2.elsewhere, ["X-1".to_string()]);
        assert_eq!(r2.candidates.as_deref(), Some(&["X-2".to_string()][..]));
        assert_eq!(r2.status, RuleStatus::Incomplete);
    }

    #[test]
    fn scopes_attribute_a_shared_course_independently() {
        // a course counts in the concentration and in the profile at once:
        // each scope's accumulator starts empty
        let program = program(
            r#""mandatory":[],"rules":[],
               "concentrations":[{"title":"Géotechnique","mandatory":[],
                 "rules":[{"title":"Règle C","constraint":{"type":"course","min":1,"max":1},
                           "courses":["X-1"]}]}],
               "profiles":[{"title":"Profil international","mandatory":[],
                 "rules":[{"title":"Règle P","constraint":{"type":"course","min":1,"max":1},
                           "courses":["X-1"]}]}]"#,
        );
        let coverage = coverage_report(
            &program,
            Some("Géotechnique"),
            Some("Profil international"),
            &selection(&["X-1"]),
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));
        for rule in &coverage.rules {
            assert_eq!(
                rule.counted.as_deref(),
                Some(&["X-1".to_string()][..]),
                "{:?}",
                rule.scope
            );
            assert!(rule.elsewhere.is_empty(), "{:?}", rule.scope);
        }
    }

    #[test]
    fn an_unconstrained_rule_before_a_constrained_one_does_not_claim_its_course(
    ) {
        // « Scolarité préparatoire » never claims a code (no constraint to
        // enforce): it comes first here, so if it fed `claimed` the later
        // constrained rule would see X-1 as `elsewhere` instead of counting
        // it — the `if rule.constraint.is_some()` guard in `scope_reports`
        // is what prevents that
        let program = bare(
            r#"[{"title":"Règle 1","courses":["X-1"]},
                {"title":"Règle 2","constraint":{"type":"course","min":1,"max":1},
                 "courses":["X-1"]}]"#,
        );
        let coverage = report(&program, &["X-1"], &[]);
        let r2 = &coverage.rules[1];
        assert_eq!(r2.counted.as_deref(), Some(&["X-1".to_string()][..]));
        assert!(r2.elsewhere.is_empty());
        assert_eq!(r2.status, RuleStatus::Satisfied);
    }

    // --- credits rules ---

    fn credits_rule(min: i64, max: i64) -> Program {
        bare(&format!(
            r#"[{{"title":"Règle 2","constraint":{{"type":"credits","min":{min},"max":{max}}},
                 "courses":["A-1","B-2","C-3"]}}]"#
        ))
    }

    #[test]
    fn a_credits_rule_sums_the_selected_courses() {
        let coverage = report(
            &credits_rule(3, 9),
            &["A-1", "B-2"],
            &[course("A-1", "3"), course("B-2", "3")],
        );
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
    }

    #[test]
    fn a_credits_rule_below_the_min_says_how_many_credits_remain() {
        let coverage =
            report(&credits_rule(6, 9), &["A-1"], &[course("A-1", "3")]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Incomplete);
        assert_eq!(
            coverage.rules[0].missing,
            Some(Missing::Credits { credits: 3 })
        );
    }

    #[test]
    fn a_zero_min_credits_rule_is_satisfied_by_an_empty_selection() {
        // génie industriel has a real {min: 0} rule: 0 credits ≥ 0
        let coverage = report(&credits_rule(0, 9), &[], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
    }

    #[test]
    fn a_range_credit_course_counts_its_lower_bound() {
        // ADR `2026-07-credits-range-borne-basse-en-planification`
        let coverage = report(
            &credits_rule(3, 9),
            &["A-1"],
            &[course("A-1", r#"{"min":3,"max":12}"#)],
        );
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
    }

    #[test]
    fn a_sum_above_the_max_is_the_rule_s_own_verdict() {
        // the arbitration the ADR
        // `2026-07-somme-au-dessus-du-max-en-erreur-typee` awaited, taken
        // on 2026-08-30: a violation on this rule, not a refusal of the
        // report (ADR `2026-08-depassement-de-regle-en-statut-rouge`)
        let program = credits_rule(3, 3);
        let coverage = report(
            &program,
            &["A-1", "B-2"],
            &[course("A-1", "3"), course("B-2", "3")],
        );
        assert_eq!(coverage.rules[0].status, RuleStatus::OverMax);
        assert_eq!(coverage.rules[0].defect, None);
        assert_eq!(coverage.rules[0].missing, None);
    }

    #[test]
    fn a_counted_course_without_its_credits_marks_its_own_rule() {
        let program = credits_rule(3, 9);
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Uncounted);
        assert_eq!(
            coverage.rules[0].defect,
            Some(RuleDefect::MissingCourse {
                code: "A-1".to_string(),
            })
        );
    }

    // --- reported rules: surfaced, never invented ---

    #[test]
    fn keyword_and_raw_rules_are_reported_with_their_raw_text() {
        let program = bare(
            r#"[{"title":"Règle 5","constraint":{"type":"credits","min":3,"max":3},
                 "courses":"any","raw":"tous les cours de premier cycle"},
                {"title":"Règle 6","constraint":{"type":"credits","min":3,"max":3},
                 "courses":"negotiated","raw":"convenus avec la direction"},
                {"title":"Règle 7","raw":"du texte hors grammaire"}]"#,
        );
        let coverage = report(&program, &[], &[]);
        for (rule, raw) in coverage.rules.iter().zip([
            "tous les cours de premier cycle",
            "convenus avec la direction",
            "du texte hors grammaire",
        ]) {
            assert_eq!(rule.status, RuleStatus::Reported, "{}", rule.title);
            assert_eq!(rule.raw.as_deref(), Some(raw), "{}", rule.title);
            assert_eq!(rule.counted, None, "{}", rule.title);
            assert_eq!(rule.candidates, None, "{}", rule.title);
        }
    }

    #[test]
    fn a_list_rule_without_a_constraint_reports_its_split_without_raw() {
        // « Scolarité préparatoire » : no verdict — which listed course
        // applies depends on the student's collegial record — but the split
        // is still shown for the UI to surface (ADR
        // `2026-08-regle-sans-contrainte-comptee-mais-reportee`)
        let program = bare(r#"[{"title":"Règle 1","courses":["A-1","B-2"]}]"#);
        let coverage = report(&program, &["A-1"], &[]);
        let rule = &coverage.rules[0];
        assert_eq!(rule.status, RuleStatus::Reported);
        assert_eq!(rule.counted.as_deref(), Some(&["A-1".to_string()][..]));
        assert_eq!(rule.missing, None);
        assert_eq!(rule.candidates.as_deref(), Some(&["B-2".to_string()][..]));
        assert_eq!(rule.raw, None);
    }

    // --- references: resolved, never chased ---

    fn referencing(concentrations: &str) -> Program {
        program(&format!(
            r#""mandatory":[],
               "rules":[{{"title":"Règle 2","constraint":{{"type":"course","min":1,"max":1}},
                          "courses":{{"concentration":"Géotechnique",
                                      "rule":"Règle 1"}},
                          "raw":"tous les cours de la Règle 1"}}],
               "concentrations":{concentrations},"profiles":[]"#
        ))
    }

    #[test]
    fn a_reference_resolves_to_the_target_list_and_evaluates() {
        let program = referencing(
            r#"[{"title":"Géotechnique","mandatory":[],
                 "rules":[{"title":"Règle 1","constraint":{"type":"course","min":2,"max":2},
                           "courses":["A-1","B-2"]}]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
        assert_eq!(
            coverage.rules[0].counted.as_deref(),
            Some(&["A-1".to_string()][..])
        );
        assert_eq!(
            resolved_rule_courses(&program, &program.rules[0])
                .unwrap_or_else(|e| panic!("{e:?}")),
            Some(&["A-1".to_string(), "B-2".to_string()][..])
        );
    }

    #[test]
    fn a_reference_without_a_constraint_is_reported_with_its_raw() {
        // the resolution still runs (a broken reference stays an error),
        // but a rule naming no number is only ever reported
        let mut program = referencing(
            r#"[{"title":"Géotechnique","mandatory":[],
                 "rules":[{"title":"Règle 1","constraint":{"type":"course","min":2,"max":2},
                           "courses":["A-1","B-2"]}]}]"#,
        );
        program.rules[0].constraint = None;
        let coverage = report(&program, &[], &[]);
        let rule = &coverage.rules[0];
        assert_eq!(rule.status, RuleStatus::Reported);
        assert_eq!(rule.raw.as_deref(), Some("tous les cours de la Règle 1"));
        // the resolved list still splits — counted empty, all candidates
        assert_eq!(rule.counted.as_deref(), Some(&[][..]));
        assert_eq!(
            rule.candidates.as_deref(),
            Some(&["A-1".to_string(), "B-2".to_string()][..])
        );
    }

    #[test]
    fn every_broken_reference_marks_its_rule_and_nothing_else() {
        // unknown concentration, unknown rule, target that is not a list:
        // three ways to fail the same chase, one verdict on one rule
        let broken = RuleDefect::BrokenReference {
            concentration: "Géotechnique".to_string(),
            target: "Règle 1".to_string(),
        };
        for program in [
            referencing("[]"),
            referencing(
                r#"[{"title":"Géotechnique","mandatory":[],"rules":[]}]"#,
            ),
            referencing(
                r#"[{"title":"Géotechnique","mandatory":[],
                     "rules":[{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},
                               "courses":"any","raw":"tous les cours"}]}]"#,
            ),
        ] {
            let coverage = report(&program, &[], &[]);
            let rule = &coverage.rules[0];
            assert_eq!(rule.status, RuleStatus::Uncounted);
            assert_eq!(rule.defect.as_ref(), Some(&broken));
            // the raw text survives, as it does for every uncounted rule
            assert_eq!(
                rule.raw.as_deref(),
                Some("tous les cours de la Règle 1")
            );
        }
    }

    // --- scopes: program plus the chosen concentration and profile ---

    fn scoped() -> Program {
        program(
            r#""mandatory":["M-1"],"rules":[],
               "concentrations":[{"title":"Géotechnique",
                 "mandatory":["C-1"],
                 "rules":[{"title":"Règle C","constraint":{"type":"course","min":1,"max":1},
                           "courses":["C-2"]}]}],
               "profiles":[{"title":"Profil international",
                 "mandatory":["P-1"],"rules":[]}]"#,
        )
    }

    #[test]
    fn chosen_scopes_append_their_mandatory_and_rules_in_order() {
        let coverage = coverage_report(
            &scoped(),
            Some("Géotechnique"),
            Some("Profil international"),
            &selection(&["C-2"]),
            &[],
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let scopes: Vec<Scope> =
            coverage.mandatory.iter().map(|block| block.scope).collect();
        assert_eq!(
            scopes,
            [Scope::Program, Scope::Concentration, Scope::Profile]
        );
        assert_eq!(coverage.rules.len(), 1);
        assert_eq!(coverage.rules[0].scope, Scope::Concentration);
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
    }

    #[test]
    fn an_unknown_concentration_title_is_an_error() {
        let error = coverage_report(
            &scoped(),
            Some("Hydraulique"),
            None,
            &selection(&[]),
            &[],
        )
        .expect_err("no such concentration");
        assert_eq!(
            error,
            CoverageError::UnknownConcentration {
                title: "Hydraulique".to_string(),
            }
        );
    }

    #[test]
    fn an_unknown_profile_title_is_an_error() {
        let error = coverage_report(
            &scoped(),
            None,
            Some("Profil entrepreneurial"),
            &selection(&[]),
            &[],
        )
        .expect_err("no such profile");
        assert_eq!(
            error,
            CoverageError::UnknownProfile {
                title: "Profil entrepreneurial".to_string(),
            }
        );
    }

    // --- language requirement ---

    fn with_language() -> Program {
        program(
            r#""mandatory":[],"rules":[],"concentrations":[],"profiles":[],
               "language_requirement":{
                 "francophone":{"course":"ANL-2020","raw":"ANL-2020"},
                 "non_francophone":{"course":"FLS-2093","raw":"FLS-2093"}}"#,
        )
    }

    #[test]
    fn either_language_branch_in_the_selection_satisfies() {
        for code in ["ANL-2020", "FLS-2093"] {
            let coverage = report(&with_language(), &[code], &[]);
            assert_eq!(
                coverage.language_requirement,
                Some(LanguageReport {
                    status: LanguageStatus::Satisfied
                }),
                "for {code}"
            );
        }
    }

    #[test]
    fn an_unselected_language_requirement_is_reported_never_missing() {
        // a test score can dispense from the course and core cannot see it
        let coverage = report(&with_language(), &[], &[]);
        assert_eq!(
            coverage.language_requirement,
            Some(LanguageReport {
                status: LanguageStatus::Reported
            })
        );
    }

    #[test]
    fn a_program_without_the_requirement_omits_the_key() {
        let coverage = report(&bare("[]"), &[], &[]);
        assert_eq!(coverage.language_requirement, None);
        let json = serde_json::to_value(&coverage)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        assert!(json.get("language_requirement").is_none(), "{json}");
    }

    // --- serialization: absent keys stay absent, mirror of the fixtures ---

    #[test]
    fn evaluated_and_reported_entries_serialize_their_own_keys_only() {
        let program = bare(
            r#"[{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},
                 "courses":["A-1"]},
                {"title":"Règle 5","constraint":{"type":"credits","min":3,"max":3},
                 "courses":"any","raw":"tous les cours"}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        let json = serde_json::to_value(&coverage)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        let evaluated = &json["rules"][0];
        assert_eq!(evaluated["status"], "satisfied");
        assert!(evaluated.get("missing").is_none(), "{evaluated}");
        assert!(evaluated.get("raw").is_none(), "{evaluated}");
        let reported = &json["rules"][1];
        assert_eq!(reported["status"], "reported");
        assert!(reported.get("counted").is_none(), "{reported}");
        assert!(reported.get("candidates").is_none(), "{reported}");
        assert_eq!(reported["raw"], "tous les cours");
    }

    #[test]
    fn every_coverage_error_names_its_subject() {
        let text = |error: CoverageError| error.to_string();
        assert!(text(CoverageError::UnknownConcentration {
            title: "X".to_string()
        })
        .contains("X"));
        assert!(text(CoverageError::UnknownProfile {
            title: "X".to_string()
        })
        .contains("X"));
    }

    // a defect rides in the report, so it must survive serialization —
    // an unnamed one would be a rule silently uncounted
    #[test]
    fn a_defect_serializes_beside_its_rule() {
        let program = credits_rule(3, 9);
        let coverage = report(&program, &["A-1"], &[]);
        let json =
            serde_json::to_value(&coverage).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(json["rules"][0]["status"], "uncounted");
        assert_eq!(
            json["rules"][0]["defect"]["missing_course"]["code"],
            "A-1"
        );
        // every healthy rule stays free of the field
        let clean = report(
            &bare(
                r#"[{"title":"Règle 1",
                 "constraint":{"type":"course","min":1,"max":1},
                 "courses":["A-1"]}]"#,
            ),
            &["A-1"],
            &[],
        );
        let json =
            serde_json::to_value(&clean).unwrap_or_else(|e| panic!("{e}"));
        assert!(json["rules"][0].get("defect").is_none(), "{json}");
    }
}
