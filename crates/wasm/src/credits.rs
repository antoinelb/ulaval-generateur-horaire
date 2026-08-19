use std::collections::BTreeSet;

use ulaval_scheduler_core::{Course, CourseCycle, Program, RuleCourses};

// The header's « 96/120 cr » : credits toward the diploma, with the two
// families that never count — « en sus » (the promoted Stages rule, ADR
// `2026-08-stage-obligatoire-en-prose-promu-en-regle`) and the préuniversité
// (scolarité préparatoire) — tallied apart so the UI can show them instead
// of silently miscounting (`docs/next_steps.md` : `credits_in_addition`
// must be subtracted before comparing to `credits_required`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditSummary {
    pub counted: u32,
    pub in_addition: u32,
    pub preparatory: u32,
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
    let mut summary = CreditSummary {
        counted: 0,
        in_addition: 0,
        preparatory: 0,
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
        } else {
            summary.counted += credits;
        }
    }
    summary
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
}
