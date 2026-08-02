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
pub struct CoverageReport {
    pub mandatory: Vec<MandatoryReport>,
    pub rules: Vec<RuleReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_requirement: Option<LanguageReport>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MandatoryReport {
    pub scope: Scope,
    pub satisfied: Vec<String>,
    pub missing: Vec<String>,
}

// One rule's verdict. The evaluated fields (`counted`, `candidates`) exist
// exactly when the rule could be counted; a `reported` rule carries its
// `raw` text instead — surfaced to the student, never invented (ADRs
// `2026-07-contrainte-de-regle-optionnelle`,
// `2026-07-regles-negociees-reconnues`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RuleReport {
    pub scope: Scope,
    pub title: String,
    pub status: RuleStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counted: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Missing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Program,
    Concentration,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    Satisfied,
    Incomplete,
    Reported,
}

// mirror of `Constraint`: what remains to reach the count or the minimum
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum Missing {
    Count { count: i64 },
    Credits { credits: i64 },
}

// The language requirement is never « missing » : a placement-test score
// can dispense from the course and core cannot see it, so the only verdicts
// are satisfied (a branch's course is selected) or reported.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LanguageReport {
    pub status: LanguageStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageStatus {
    Satisfied,
    Reported,
}

// one scope's block: its mandatory list and its rules
type ScopeBlock<'a> = (Scope, &'a [String], &'a [Rule]);

// Inputs the report refuses to guess about — surfaced, never patched over.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoverageError {
    #[error("no concentration titled « {title} » in the program")]
    UnknownConcentration { title: String },
    #[error("no profile titled « {title} » in the program")]
    UnknownProfile { title: String },
    #[error(
        "{rule} references concentration « {concentration} », \
         which the program does not have"
    )]
    ReferenceUnknownConcentration { rule: String, concentration: String },
    #[error(
        "{rule} references « {target} » of « {concentration} », \
         which has no such rule"
    )]
    ReferenceUnknownRule {
        rule: String,
        concentration: String,
        target: String,
    },
    #[error(
        "{rule} references « {target} » of « {concentration} », \
         which is not a course list — a reference chase is an error"
    )]
    ReferenceNotAList {
        rule: String,
        concentration: String,
        target: String,
    },
    #[error("{rule} : {code} counts credits but no Course carries them")]
    MissingCourse { rule: String, code: String },
    // semantics undecided — violation or uncounted surplus — so no verdict
    // is invented (ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee`)
    #[error(
        "{rule} : the selection sums {total} credits, above the max {max} \
         — semantics await the director's ruling"
    )]
    CreditsOverMax { rule: String, total: i64, max: i64 },
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
                rules.iter().map(|rule| {
                    rule_report(*scope, rule, program, selection, &credits)
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        language_requirement: program
            .language_requirement
            .as_ref()
            .map(|requirement| language_report(requirement, selection)),
    })
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
        let block = program
            .concentrations
            .iter()
            .find(|block| block.title == title)
            .ok_or_else(|| CoverageError::UnknownConcentration {
                title: title.to_string(),
            })?;
        scopes.push((Scope::Concentration, &block.mandatory, &block.rules));
    }
    if let Some(title) = profile {
        let block = program
            .profiles
            .iter()
            .find(|block| block.title == title)
            .ok_or_else(|| CoverageError::UnknownProfile {
                title: title.to_string(),
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
) -> Result<RuleReport, CoverageError> {
    // a reference is resolved before the constraint check: a broken
    // reference is an error even on a rule that then only gets reported
    match (resolved_courses(rule, program)?, &rule.constraint) {
        (Some(listed), Some(constraint)) => {
            evaluated(scope, rule, listed, constraint, selection, credits)
        }
        // Keyword (any/negotiated), raw-only, or a rule naming no number
        _ => Ok(reported(scope, rule)),
    }
}

fn resolved_courses<'a>(
    rule: &'a Rule,
    program: &'a Program,
) -> Result<Option<&'a [String]>, CoverageError> {
    match &rule.courses {
        RuleCourses::List { courses } => Ok(Some(courses)),
        RuleCourses::Reference { courses, .. } => {
            resolve_reference(&rule.title, courses, program).map(Some)
        }
        RuleCourses::Keyword { .. } | RuleCourses::Raw { .. } => Ok(None),
    }
}

// « tous les cours de la Règle N du cheminement X » : both titles come from
// the same scraped page; a target that is itself not a plain list is an
// error, not a chase
fn resolve_reference<'a>(
    rule_title: &str,
    reference: &RuleReference,
    program: &'a Program,
) -> Result<&'a [String], CoverageError> {
    let concentration = program
        .concentrations
        .iter()
        .find(|block| block.title == reference.concentration)
        .ok_or_else(|| CoverageError::ReferenceUnknownConcentration {
            rule: rule_title.to_string(),
            concentration: reference.concentration.clone(),
        })?;
    let target = concentration
        .rules
        .iter()
        .find(|target| target.title == reference.rule)
        .ok_or_else(|| CoverageError::ReferenceUnknownRule {
            rule: rule_title.to_string(),
            concentration: reference.concentration.clone(),
            target: reference.rule.clone(),
        })?;
    match &target.courses {
        RuleCourses::List { courses } => Ok(courses),
        _ => Err(CoverageError::ReferenceNotAList {
            rule: rule_title.to_string(),
            concentration: reference.concentration.clone(),
            target: reference.rule.clone(),
        }),
    }
}

fn evaluated(
    scope: Scope,
    rule: &Rule,
    listed: &[String],
    constraint: &Constraint,
    selection: &BTreeSet<String>,
    credits: &BTreeMap<&str, u32>,
) -> Result<RuleReport, CoverageError> {
    // set semantics: the page duplicates codes across thematic subgroups
    // (règle 4 GEX), and the sorted iteration matches the frozen fixtures
    let listed: BTreeSet<&str> = listed.iter().map(String::as_str).collect();
    let counted: Vec<String> = listed
        .iter()
        .filter(|code| selection.contains(**code))
        .map(|code| code.to_string())
        .collect();
    let candidates: Vec<String> = listed
        .iter()
        .filter(|code| !selection.contains(**code))
        .map(|code| code.to_string())
        .collect();
    let (status, missing) =
        verdict(&rule.title, constraint, &counted, credits)?;
    Ok(RuleReport {
        scope,
        title: rule.title.clone(),
        status,
        counted: Some(counted),
        missing,
        candidates: Some(candidates),
        raw: None,
    })
}

fn verdict(
    title: &str,
    constraint: &Constraint,
    counted: &[String],
    credits: &BTreeMap<&str, u32>,
) -> Result<(RuleStatus, Option<Missing>), CoverageError> {
    match *constraint {
        Constraint::Count { count } => {
            let chosen = counted.len() as i64;
            if chosen >= count {
                Ok((RuleStatus::Satisfied, None))
            } else {
                Ok((
                    RuleStatus::Incomplete,
                    Some(Missing::Count {
                        count: count - chosen,
                    }),
                ))
            }
        }
        Constraint::Credits { min, max } => {
            let total = counted.iter().try_fold(0i64, |acc, code| {
                credits
                    .get(code.as_str())
                    .map(|&value| acc + i64::from(value))
                    .ok_or_else(|| CoverageError::MissingCourse {
                        rule: title.to_string(),
                        code: code.clone(),
                    })
            })?;
            if total > max {
                Err(CoverageError::CreditsOverMax {
                    rule: title.to_string(),
                    total,
                    max,
                })
            } else if total >= min {
                Ok((RuleStatus::Satisfied, None))
            } else {
                Ok((
                    RuleStatus::Incomplete,
                    Some(Missing::Credits {
                        credits: min - total,
                    }),
                ))
            }
        }
    }
}

fn reported(scope: Scope, rule: &Rule) -> RuleReport {
    RuleReport {
        scope,
        title: rule.title.clone(),
        status: RuleStatus::Reported,
        counted: None,
        missing: None,
        candidates: None,
        raw: rule_raw(rule).map(str::to_string),
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
            r#"[{"title":"Règle 1","constraint":{"count":1},
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
            r#"[{"title":"Règle 1","constraint":{"count":2},
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
            r#"[{"title":"Règle 4","constraint":{"count":2},
                 "courses":["A-1","A-1","B-2"]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(
            coverage.rules[0].counted.as_deref(),
            Some(&["A-1".to_string()][..])
        );
        assert_eq!(coverage.rules[0].status, RuleStatus::Incomplete);
    }

    // --- credits rules ---

    fn credits_rule(min: i64, max: i64) -> Program {
        bare(&format!(
            r#"[{{"title":"Règle 2","constraint":{{"min":{min},"max":{max}}},
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
    fn a_sum_above_the_max_is_a_typed_error() {
        // ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee`
        let program = credits_rule(3, 3);
        let error = coverage_report(
            &program,
            None,
            None,
            &selection(&["A-1", "B-2"]),
            &[course("A-1", "3"), course("B-2", "3")],
        )
        .expect_err("6 credits above max 3");
        assert_eq!(
            error,
            CoverageError::CreditsOverMax {
                rule: "Règle 2".to_string(),
                total: 6,
                max: 3,
            }
        );
    }

    #[test]
    fn a_counted_course_without_its_credits_is_an_error() {
        let program = credits_rule(3, 9);
        let error =
            coverage_report(&program, None, None, &selection(&["A-1"]), &[])
                .expect_err("no Course carries A-1's credits");
        assert_eq!(
            error,
            CoverageError::MissingCourse {
                rule: "Règle 2".to_string(),
                code: "A-1".to_string(),
            }
        );
    }

    // --- reported rules: surfaced, never invented ---

    #[test]
    fn keyword_and_raw_rules_are_reported_with_their_raw_text() {
        let program = bare(
            r#"[{"title":"Règle 5","constraint":{"min":3,"max":3},
                 "courses":"any","raw":"tous les cours de premier cycle"},
                {"title":"Règle 6","constraint":{"min":3,"max":3},
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
    fn a_list_rule_without_a_constraint_is_reported_without_raw() {
        // génie mécanique's real « Règle 1 – Réussir la scolarité de » is
        // cut off mid-sentence: shown, never counted
        let program = bare(r#"[{"title":"Règle 1","courses":["A-1","B-2"]}]"#);
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Reported);
        assert_eq!(coverage.rules[0].raw, None);
    }

    // --- references: resolved, never chased ---

    fn referencing(concentrations: &str) -> Program {
        program(&format!(
            r#""mandatory":[],
               "rules":[{{"title":"Règle 2","constraint":{{"count":1}},
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
                 "rules":[{"title":"Règle 1","constraint":{"count":2},
                           "courses":["A-1","B-2"]}]}]"#,
        );
        let coverage = report(&program, &["A-1"], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Satisfied);
        assert_eq!(
            coverage.rules[0].counted.as_deref(),
            Some(&["A-1".to_string()][..])
        );
    }

    #[test]
    fn a_reference_without_a_constraint_is_reported_with_its_raw() {
        // the resolution still runs (a broken reference stays an error),
        // but a rule naming no number is only ever reported
        let mut program = referencing(
            r#"[{"title":"Géotechnique","mandatory":[],
                 "rules":[{"title":"Règle 1","constraint":{"count":2},
                           "courses":["A-1","B-2"]}]}]"#,
        );
        program.rules[0].constraint = None;
        let coverage = report(&program, &[], &[]);
        assert_eq!(coverage.rules[0].status, RuleStatus::Reported);
        assert_eq!(
            coverage.rules[0].raw.as_deref(),
            Some("tous les cours de la Règle 1")
        );
    }

    #[test]
    fn a_reference_to_an_unknown_concentration_is_an_error() {
        let program = referencing("[]");
        let error =
            coverage_report(&program, None, None, &selection(&[]), &[])
                .expect_err("no Géotechnique block");
        assert_eq!(
            error,
            CoverageError::ReferenceUnknownConcentration {
                rule: "Règle 2".to_string(),
                concentration: "Géotechnique".to_string(),
            }
        );
    }

    #[test]
    fn a_reference_to_an_unknown_rule_is_an_error() {
        let program = referencing(
            r#"[{"title":"Géotechnique","mandatory":[],"rules":[]}]"#,
        );
        let error =
            coverage_report(&program, None, None, &selection(&[]), &[])
                .expect_err("no Règle 1 in the block");
        assert_eq!(
            error,
            CoverageError::ReferenceUnknownRule {
                rule: "Règle 2".to_string(),
                concentration: "Géotechnique".to_string(),
                target: "Règle 1".to_string(),
            }
        );
    }

    #[test]
    fn a_reference_whose_target_is_not_a_list_is_an_error_not_a_chase() {
        let program = referencing(
            r#"[{"title":"Géotechnique","mandatory":[],
                 "rules":[{"title":"Règle 1","constraint":{"count":1},
                           "courses":"any","raw":"tous les cours"}]}]"#,
        );
        let error =
            coverage_report(&program, None, None, &selection(&[]), &[])
                .expect_err("the target is a keyword, not a list");
        assert_eq!(
            error,
            CoverageError::ReferenceNotAList {
                rule: "Règle 2".to_string(),
                concentration: "Géotechnique".to_string(),
                target: "Règle 1".to_string(),
            }
        );
    }

    // --- scopes: program plus the chosen concentration and profile ---

    fn scoped() -> Program {
        program(
            r#""mandatory":["M-1"],"rules":[],
               "concentrations":[{"title":"Géotechnique",
                 "mandatory":["C-1"],
                 "rules":[{"title":"Règle C","constraint":{"count":1},
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
            r#"[{"title":"Règle 1","constraint":{"count":1},
                 "courses":["A-1"]},
                {"title":"Règle 5","constraint":{"min":3,"max":3},
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
        assert!(text(CoverageError::ReferenceUnknownConcentration {
            rule: "R".to_string(),
            concentration: "X".to_string()
        })
        .contains("X"));
        assert!(text(CoverageError::ReferenceUnknownRule {
            rule: "R".to_string(),
            concentration: "X".to_string(),
            target: "T".to_string()
        })
        .contains("T"));
        assert!(text(CoverageError::ReferenceNotAList {
            rule: "R".to_string(),
            concentration: "X".to_string(),
            target: "T".to_string()
        })
        .contains("chase"));
        assert!(text(CoverageError::MissingCourse {
            rule: "R".to_string(),
            code: "A-1".to_string()
        })
        .contains("A-1"));
        assert!(text(CoverageError::CreditsOverMax {
            rule: "R".to_string(),
            total: 6,
            max: 3
        })
        .contains("6"));
    }
}
