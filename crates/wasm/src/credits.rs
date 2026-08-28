use std::collections::BTreeSet;

use ulaval_scheduler_core::{
    resolved_rule_courses, Constraint, Course, CourseCycle, Keyword, Program,
    Rule, RuleCourses,
};

// The header's « 96/120 cr » : credits toward the diploma, with the
// families that never count as new credits — « en sus » (the promoted
// Stages rule, ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`),
// the préuniversité (scolarité préparatoire), and a chosen profile's own
// courses beyond what a free "any" rule can absorb (ADR
// `2026-08-le-profil-napporte-jamais-de-credits-neufs`) — tallied apart so
// the UI can show them instead of silently miscounting (`docs/next_steps.
// md` : `credits_in_addition` must be subtracted before comparing to
// `credits_required`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditSummary {
    pub counted: u32,
    pub in_addition: u32,
    pub preparatory: u32,
    // a profile course neither listed elsewhere nor absorbable into a free
    // credits rule — the profile substitutes, it never grows the total, so
    // this stays out of `counted` (ADR
    // `2026-08-le-profil-napporte-jamais-de-credits-neufs`)
    pub profile_only: u32,
    // selected codes with no Course to read credits from — surfaced,
    // never dropped
    pub unknown: Vec<String>,
}

// A `Credits::Range` counts at its planning value (the lower bound), the
// same convention the solver uses; showing the interval is the row's job.
pub fn credit_summary(
    program: Option<&Program>,
    concentration: Option<&str>,
    profile: Option<&str>,
    selection: &BTreeSet<String>,
    courses: &[Course],
) -> CreditSummary {
    let en_sus = program
        .map(|program| en_sus_codes(program, concentration, profile))
        .unwrap_or_default();
    // a profile course also pooled elsewhere counts normally — the
    // substitution only bites courses the profile alone would have
    // sheltered (ADR `2026-08-le-profil-napporte-jamais-de-credits-neufs`)
    let profile_only_codes = program
        .map(|program| {
            profile_codes(program, profile)
                .difference(&elsewhere_codes(program, concentration))
                .cloned()
                .collect::<BTreeSet<String>>()
        })
        .unwrap_or_default();
    let mut allowance = program
        .map(|program| free_credits_allowance(program, concentration))
        .unwrap_or(0);
    let mut summary = CreditSummary {
        counted: 0,
        in_addition: 0,
        preparatory: 0,
        profile_only: 0,
        unknown: Vec::new(),
    };
    for code in selection {
        let Some(course) = courses.iter().find(|course| &course.code == code)
        else {
            summary.unknown.push(code.clone());
            continue;
        };
        let credits = course.credits.planning();
        if course.cycle == CourseCycle::Preuniversity {
            summary.preparatory += credits;
        } else if en_sus.contains(code) {
            summary.in_addition += credits;
        } else if profile_only_codes.contains(code) {
            // whole-course decrement only — a course only "fits" the
            // remaining allowance if it fits entirely, never split
            if credits <= allowance {
                summary.counted += credits;
                allowance -= credits;
            } else {
                summary.profile_only += credits;
            }
        } else {
            summary.counted += credits;
        }
    }
    summary
}

// Codes the chosen profile pools : its own mandatory list, plus every rule
// naming a list of courses — a reference resolved through core the same
// way the coverage report does. A rule that resolves to no list (Keyword,
// Raw, or a broken reference) contributes nothing, never invented (ADR
// `2026-08-le-profil-napporte-jamais-de-credits-neufs`).
fn profile_codes(
    program: &Program,
    profile: Option<&str>,
) -> BTreeSet<String> {
    let Some(block) = profile.and_then(|title| program.profile(title)) else {
        return BTreeSet::new();
    };
    let mut codes: BTreeSet<String> =
        block.mandatory.iter().cloned().collect();
    codes.extend(rule_list_codes(program, &block.rules));
    codes
}

// Codes a course could already belong to *outside* the profile : the
// program's own mandatory and rule lists, plus the *chosen* concentration's
// — an unchosen concentration shelters nothing here either (mirrors
// `en_sus_codes`, décision d'Antoine 2026-08-19).
fn elsewhere_codes(
    program: &Program,
    concentration: Option<&str>,
) -> BTreeSet<String> {
    let mut codes: BTreeSet<String> =
        program.mandatory.iter().cloned().collect();
    codes.extend(rule_list_codes(program, &program.rules));
    if let Some(block) =
        concentration.and_then(|title| program.concentration(title))
    {
        codes.extend(block.mandatory.iter().cloned());
        codes.extend(rule_list_codes(program, &block.rules));
    }
    codes
}

// Every code named by a rule whose courses resolve to a list — `List`
// as-is, `Reference` chased through core; `Keyword`/`Raw`/a broken
// reference resolve to nothing and are skipped, never guessed at.
fn rule_list_codes(program: &Program, rules: &[Rule]) -> BTreeSet<String> {
    rules
        .iter()
        .filter_map(|rule| resolved_rule_courses(program, rule).ok().flatten())
        .flatten()
        .cloned()
        .collect()
}

// The only room a profile-only course can be absorbed into : the sum of
// the trunk's and chosen concentration's free "tous les cours" credit
// rules (`Keyword::Any` under a `Constraint::Credits`) — the official
// pages' « le cheminement de 12 crédits s'intègre aux cours complémentaires
// » (ADR `2026-08-le-profil-napporte-jamais-de-credits-neufs`). No `Course`
// constraint of a free rule is honoured : none exists in the scraped data.
fn free_credits_allowance(
    program: &Program,
    concentration: Option<&str>,
) -> u32 {
    let concentration_rules = concentration
        .and_then(|title| program.concentration(title))
        .map(|block| block.rules.as_slice())
        .unwrap_or_default();
    program
        .rules
        .iter()
        .chain(concentration_rules)
        .filter_map(|rule| match (&rule.courses, &rule.constraint) {
            (
                RuleCourses::Keyword {
                    courses: Keyword::Any,
                    ..
                },
                Some(Constraint::Credits { max, .. }),
            ) => Some(u32::try_from(*max).unwrap_or(0)),
            _ => None,
        })
        .sum()
}

// En-sus codes of the program and the *chosen* blocks only — an en-sus
// rule of an unselected concentration must not shelter credits (décision
// 2026-08-19). An unknown title contributes nothing here: the coverage
// report is the layer that surfaces it as an error.
fn en_sus_codes(
    program: &Program,
    concentration: Option<&str>,
    profile: Option<&str>,
) -> BTreeSet<String> {
    let concentration_rules = concentration
        .and_then(|title| program.concentration(title))
        .map(|block| block.rules.as_slice())
        .unwrap_or_default();
    let profile_rules = profile
        .and_then(|title| program.profile(title))
        .map(|block| block.rules.as_slice())
        .unwrap_or_default();
    program
        .rules
        .iter()
        .chain(concentration_rules)
        .chain(profile_rules)
        .filter(|rule| rule.credits_in_addition)
        .filter_map(|rule| match &rule.courses {
            RuleCourses::List { courses } => Some(courses),
            _ => None,
        })
        .flatten()
        .cloned()
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn course(code: &str, credits: &str, cycle: u8) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":{credits},
                 "cycle":{cycle},"prerequisites":null,"equivalents":[],
                 "seasons":{{}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn program_with_stage_rule() -> Program {
        serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],
                "rules":[{"title":"Stages",
                          "constraint":{"type":"course","min":1,"max":8},
                          "courses":["GEX-1580"],
                          "credits_in_addition":true}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"))
    }

    fn selection(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    #[test]
    fn counted_en_sus_and_preparatory_credits_are_tallied_apart() {
        let courses = vec![
            course("GEX-1000", "3", 1),
            course("GEX-1580", "6", 1),
            course("MAT-0130", "3", 0),
        ];
        let summary = credit_summary(
            Some(&program_with_stage_rule()),
            None,
            None,
            &selection(&["GEX-1000", "GEX-1580", "MAT-0130"]),
            &courses,
        );
        assert_eq!(summary.counted, 3, "only the regular course counts");
        assert_eq!(summary.in_addition, 6, "the stage is en sus");
        assert_eq!(summary.preparatory, 3, "préuniversitaire kept apart");
        assert!(summary.unknown.is_empty());
    }

    #[test]
    fn without_a_program_every_university_credit_counts() {
        let courses = vec![course("GEX-1580", "6", 1)];
        let summary = credit_summary(
            None,
            None,
            None,
            &selection(&["GEX-1580"]),
            &courses,
        );
        assert_eq!(summary.counted, 6, "no rule declares it en sus");
    }

    #[test]
    fn a_selected_code_without_a_course_is_surfaced_not_dropped() {
        let summary =
            credit_summary(None, None, None, &selection(&["GHOST-999"]), &[]);
        assert_eq!(summary.counted, 0);
        assert_eq!(summary.unknown, ["GHOST-999"]);
    }

    #[test]
    fn an_en_sus_rule_without_a_course_list_contributes_no_code() {
        // a « negotiated » en-sus rule names no fixed list: nothing to pool
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"gex","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],
                "rules":[{"title":"R","courses":"negotiated",
                          "raw":"convenus avec la direction",
                          "credits_in_addition":true}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("GEX-1580", "6", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            None,
            &selection(&["GEX-1580"]),
            &courses,
        );
        assert_eq!(summary.counted, 6, "no list, nothing en sus");
    }

    #[test]
    fn a_blocks_en_sus_rule_counts_only_when_the_block_is_chosen() {
        // the en-sus shelter follows the choice: unchosen, the course
        // counts toward the diploma like any other
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GMC","slug":"gmc","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[
                  {"title":"Robotique","mandatory":[],
                   "rules":[{"title":"R","courses":["GMC-3351"],
                             "credits_in_addition":true}]}],
                "profiles":[
                  {"title":"Profil international","mandatory":[],
                   "rules":[{"title":"R","courses":["GPL-1000"],
                             "credits_in_addition":true}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses =
            vec![course("GMC-3351", "3", 1), course("GPL-1000", "3", 1)];
        let picked = selection(&["GMC-3351", "GPL-1000"]);

        let unchosen =
            credit_summary(Some(&program), None, None, &picked, &courses);
        assert_eq!(unchosen.counted, 6, "no block chosen, nothing en sus");

        let chosen = credit_summary(
            Some(&program),
            Some("Robotique"),
            Some("Profil international"),
            &picked,
            &courses,
        );
        assert_eq!(chosen.counted, 0);
        assert_eq!(chosen.in_addition, 6, "both chosen blocks shelter theirs");
    }

    #[test]
    fn a_credits_range_counts_at_its_planning_value() {
        let courses = vec![course("GEX-2500", r#"{"min":6,"max":12}"#, 1)];
        let summary = credit_summary(
            None,
            None,
            None,
            &selection(&["GEX-2500"]),
            &courses,
        );
        assert_eq!(summary.counted, 6, "the lower bound, never a guess");
    }

    // --- the profile substitutes, it never adds (ADR
    // `2026-08-le-profil-napporte-jamais-de-credits-neufs`) ---

    #[test]
    fn a_course_listed_only_by_the_profile_does_not_count() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R","courses":["PRF-1000"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("PRF-1000", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            Some("Profil DD"),
            &selection(&["PRF-1000"]),
            &courses,
        );
        assert_eq!(summary.counted, 0, "the profile alone shelters nothing");
        assert_eq!(summary.profile_only, 3);
    }

    #[test]
    fn a_profile_course_also_listed_by_the_chosen_concentration_counts() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[
                  {"title":"Structures","mandatory":[],
                   "rules":[{"title":"Règle 1","courses":["GCI-4201"]}]}],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R","courses":["GCI-4201"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("GCI-4201", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            Some("Structures"),
            Some("Profil DD"),
            &selection(&["GCI-4201"]),
            &courses,
        );
        assert_eq!(
            summary.counted, 3,
            "listed by the chosen concentration too"
        );
        assert_eq!(summary.profile_only, 0);
    }

    #[test]
    fn a_profile_only_course_absorbs_into_a_free_any_rule_up_to_its_max() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],
                "rules":[{"title":"Complémentaires",
                          "constraint":{"type":"credits","min":0,"max":3},
                          "courses":"any",
                          "raw":"tous les cours de premier cycle"}],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R",
                                       "courses":["DDU-1000","ENT-2000"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses =
            vec![course("DDU-1000", "3", 1), course("ENT-2000", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            Some("Profil DD"),
            &selection(&["DDU-1000", "ENT-2000"]),
            &courses,
        );
        // the selection is a BTreeSet, walked alphabetically: DDU-1000
        // absorbs the whole 3-cr allowance first, leaving none for
        // ENT-2000 — a course is never split across the two buckets
        assert_eq!(summary.counted, 3, "DDU-1000 absorbed the free allowance");
        assert_eq!(summary.profile_only, 3, "ENT-2000 found no room left");
    }

    #[test]
    fn an_unlisted_course_keeps_counting_with_a_profile_chosen() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R","courses":["PRF-1000"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("HORS-9999", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            Some("Profil DD"),
            &selection(&["HORS-9999"]),
            &courses,
        );
        assert_eq!(summary.counted, 3, "unlisted anywhere, counts as always");
        assert_eq!(summary.profile_only, 0);
    }

    #[test]
    fn profile_mandatory_and_rule_lists_pool_together() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":["PRF-9999"],
                             "rules":[{"title":"R","courses":["PRF-1000"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses =
            vec![course("PRF-9999", "3", 1), course("PRF-1000", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            Some("Profil DD"),
            &selection(&["PRF-9999", "PRF-1000"]),
            &courses,
        );
        assert_eq!(summary.counted, 0, "both pooled, neither elsewhere");
        assert_eq!(summary.profile_only, 6, "mandatory and rule list, pooled");
    }

    #[test]
    fn an_unchosen_profile_shelters_nothing() {
        let program: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R","courses":["PRF-1000"]}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("PRF-1000", "3", 1)];
        let summary = credit_summary(
            Some(&program),
            None,
            None,
            &selection(&["PRF-1000"]),
            &courses,
        );
        assert_eq!(summary.counted, 3, "no profile chosen, nothing sheltered");
        assert_eq!(summary.profile_only, 0);
    }

    #[test]
    fn a_reference_shaped_profile_rule_resolves_or_contributes_nothing() {
        let valid: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[
                  {"title":"Structures","mandatory":[],
                   "rules":[{"title":"Règle 1","courses":["GCI-9001"]}]}],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R",
                                       "courses":{"concentration":"Structures",
                                                  "rule":"Règle 1"},
                                       "raw":"tous les cours de la Règle 1"}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let courses = vec![course("GCI-9001", "3", 1)];
        let resolved = credit_summary(
            Some(&valid),
            None,
            Some("Profil DD"),
            &selection(&["GCI-9001"]),
            &courses,
        );
        assert_eq!(
            resolved.profile_only, 3,
            "the reference resolved to a list"
        );
        assert_eq!(resolved.counted, 0);

        let broken: Program = serde_json::from_str(
            r#"{"code":"B-GCI","slug":"gci","semester":"A26","title":"P",
                "cycle":1,"credits_required":120,"mandatory":[],"rules":[],
                "concentrations":[],
                "profiles":[{"title":"Profil DD","mandatory":[],
                             "rules":[{"title":"R",
                                       "courses":{"concentration":"Inconnue",
                                                  "rule":"Règle 1"},
                                       "raw":"tous les cours de la Règle 1"}]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let unresolved = credit_summary(
            Some(&broken),
            None,
            Some("Profil DD"),
            &selection(&["GCI-9001"]),
            &courses,
        );
        assert_eq!(
            unresolved.counted, 3,
            "an unresolved reference contributes nothing, never invented"
        );
        assert_eq!(unresolved.profile_only, 0);
    }
}
