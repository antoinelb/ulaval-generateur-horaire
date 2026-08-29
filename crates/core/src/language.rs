// The language rule written in prose — « Réussir le cours ANL-2020
// Intermediate English II. L'étudiant qui démontre qu'il a acquis ce niveau
// (VEPT : 53) … peut choisir un cours d'anglais de niveau supérieur ou …
// un cours d'une autre langue moderne » — names one course but grants a
// choice. The parser sees a page, never the catalogue, so it can only list
// the sigles the sentence itself writes (ADR
// `2026-08-regle-linguistique-conservee-comblable`); widening the rule to
// every course the sentence actually permits needs the course snapshot,
// and happens here — pure, no IO — exactly as `preparatory::preparatory_rule`
// does (ADR `2026-08-regle-linguistique-elargie-au-catalogue`).
//
// Deliberately outside the `parser` feature: no HTML is involved, and the
// wasm build compiles `core` without it.

use std::collections::BTreeSet;

use crate::common::course_codes;
use crate::course::{Course, CourseCycle};
use crate::program::{Program, Rule, RuleCourses};

// The sigles ULaval enumerates itself, on the bac en anthropologie's page:
// « et les cours de langue moderne portant les sigles ALL, ARA, CHN, ESG,
// ITL, JAP, POR et RUS ». Copied, never invented.
//
// `ESP` (études hispaniques) is deliberately absent — `ESG` is the École de
// langues' Spanish, the one that page names. `FLS` and `FRN` are the
// non-francophone branch, carried by `Program.language_requirement`, not an
// « autre langue moderne » for the student this rule addresses.
pub const MODERN_LANGUAGE_SUBJECTS: &[&str] =
    &["ALL", "ARA", "CHN", "ESG", "ITL", "JAP", "POR", "RUS"];

const ENGLISH_SUBJECT: &str = "ANL";

// « LAN-GUES — Cours de langue selon le résultat VEPT », 3 crédits: a
// template sigle of the hand-maintained catalogue (`data/cours.manuel.json`),
// same family as `OPT-ION1` and `AUC-HOIX` (ADR
// `2026-08-cours-manuels-offerts-en-toute-saison`). Two things depend on it:
//
// - every `cheminement_type` of the B-GMC and B-GIN places it in a session
//   as the language slot, so a language rule that does not list it leaves
//   that placement counted by no rule at all;
// - the vintages scraped before 2026-08-28 carry it as the *whole* content
//   of their language rule — the prose was stripped out to
//   `language_requirement` back then and the placeholder stood in for the
//   choice it named. It is the one marker telling such a rule apart from any
//   other single-course rule: the B-GMC's « Profil entrepreneurial – Règle 3 »
//   holds `OPT-GMC1` under the very same `credits 3..3` and no note.
pub const LANGUAGE_PLACEHOLDER_CODE: &str = "LAN-GUES";

// Nine ANL courses and eight modern languages come to 77 today. The cap is
// there so a catalogue that grows a whole faculty of language courses fails
// loudly instead of dropping a rule nobody can read into the panel.
const MAX_LANGUAGE_COURSES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LanguageError {
    #[error(
        "the language rule « {title} » would list {found} courses, over the \
         {MAX_LANGUAGE_COURSES} a student can be asked to choose from"
    )]
    TooManyCourses { title: String, found: usize },
}

// The English requirement always names the École de langues test that
// dispenses from the course. Génie géologique writes « École **des**
// langues », so the article is not matched.
//
// Shared with the parser, which parks such a rule as `Raw` without an
// anomaly precisely so this pass can find it back: the two must stay in
// lockstep.
pub fn is_language_prose(raw: &str) -> bool {
    raw.contains("VEPT")
        || raw.contains("École de langues")
        || raw.contains("École des langues")
}

// Rewrites every prose language rule of the program — program scope,
// concentrations and profiles alike — as the list of courses its own
// sentence permits. The prose stays in `rule.notes`, the constraint read
// from the page's heading stays untouched, and `language_requirement` is
// not read at all: this only ever widens a choice, never restates a
// graduation gate.
pub fn widen_language_rules(
    program: &mut Program,
    courses: &[Course],
) -> Result<(), LanguageError> {
    for rule in all_rules(program) {
        let Some((listed, prose)) = widenable(rule) else {
            continue;
        };
        let found = admissible_courses(&listed, &prose, courses);
        if found.len() > MAX_LANGUAGE_COURSES {
            return Err(LanguageError::TooManyCourses {
                title: rule.title.clone(),
                found: found.len(),
            });
        }
        // the set is seeded with what the rule already listed, so this can
        // only grow it — never a course lost, whatever the catalogue holds
        rule.courses = RuleCourses::List {
            courses: found.into_iter().collect(),
        };
    }
    Ok(())
}

// What a rule this pass may widen already lists, and the prose it widens
// from — or `None`.
//
// A rule already listing several courses is one ULaval published itself
// (`B-GEX`, `B-GCI`, `B-ANT`): its list is the page's, and no inference
// replaces a published list. Only the rule the parser wrote out of a
// sentence — a single course — is widened. Widening is idempotent by the
// same token: a rule this pass already expanded is a multi-course list on
// the next run.
fn widenable(rule: &Rule) -> Option<(Vec<String>, String)> {
    let RuleCourses::List { courses } = &rule.courses else {
        return None;
    };
    if courses.len() > 1 {
        return None;
    }
    let prose = rule.notes.iter().find(|note| is_language_prose(note))?;
    Some((courses.clone(), prose.clone()))
}

// Every course the sentence admits, as three unions — none of them
// guessed, each one anchored in words the page wrote:
//
// 1. every sigle the sentence names itself, whether as the course to pass
//    (« Réussir le cours ANL-2020 »), as one of several entry courses
//    (génie chimique: ANL-2020 ou ANL-3010 ou ANL-3020) or as a closed
//    list appended to it (génie agroenvironnemental: EDC-1001…);
// 2. « un cours d'anglais de niveau supérieur » → every ANL of the
//    catalogue at or above the lowest the sentence names. The floor is
//    read, never assumed: le génie écrit ANL-2020, l'anthropologie
//    ANL-3010;
// 3. « un cours d'une autre langue moderne » → every course of the sigles
//    ULaval enumerates.
//
// Seeded with whatever the rule already listed, so nothing a previous pass
// or a hand correction put there is ever dropped — `LAN-GUES` above all,
// which the cheminements types place. That placeholder joins the list even
// when the rule did not carry it, for the same reason.
//
// The open branches — « tout autre cours jugé pertinent par la direction »,
// « tout autre cours de 3 crédits », le catalogue négatif du bac en
// sciences et technologie des aliments — are not enumerable and stay in the
// note, where the entente avec la direction covers them. A prose that
// refuses a list is not given one.
fn admissible_courses(
    listed: &[String],
    prose: &str,
    courses: &[Course],
) -> BTreeSet<String> {
    let named = course_codes(prose);
    // a sigle the snapshot does not carry is still what the page says: kept
    let mut found: BTreeSet<String> = named.iter().cloned().collect();
    found.extend(listed.iter().cloned());
    found.insert(LANGUAGE_PLACEHOLDER_CODE.to_string());

    if let Some(floor) = english_floor(&named) {
        found.extend(
            subject_courses(courses, ENGLISH_SUBJECT).filter(|code| {
                course_number(code).is_some_and(|n| n >= floor)
            }),
        );
    }

    if prose.contains("langue moderne") {
        for subject in MODERN_LANGUAGE_SUBJECTS {
            found.extend(subject_courses(courses, subject));
        }
    }

    found
}

// the lowest ANL the sentence names — the level it sets as its own floor
fn english_floor(named: &[String]) -> Option<u32> {
    named
        .iter()
        .filter(|code| code.starts_with(ENGLISH_SUBJECT))
        .filter_map(|code| course_number(code))
        .min()
}

// first-cycle only: a rule of a bac never admits a graduate course, and the
// snapshot carries both (LOA-7900 sits beside LOA-1000)
fn subject_courses<'a>(
    courses: &'a [Course],
    subject: &'a str,
) -> impl Iterator<Item = String> + 'a {
    courses
        .iter()
        .filter(move |course| {
            course.cycle == CourseCycle::First
                && course
                    .code
                    .split_once('-')
                    .is_some_and(|(prefix, _)| prefix == subject)
        })
        .map(|course| course.code.clone())
}

fn course_number(code: &str) -> Option<u32> {
    code.split_once('-')?.1.parse().ok()
}

// The extraction in force before 2026-08-28 moved the whole prose out of
// the rule into `language_requirement`, leaving the rule holding only
// `LANGUAGE_PLACEHOLDER_CODE` — the language slot, with nothing on screen
// saying what could fill it.
//
// Those vintages cannot be re-scraped: their pages are gone from ulaval.ca,
// and refetching would overwrite a frozen snapshot with today's programme.
// The prose survived in `language_requirement.francophone.raw`, so the note
// goes back where the current parser would have written it — no network,
// exactly as `reparse` re-derives prerequisite trees from the `raw` already
// stored. The rule's own list is left untouched; the widening that follows
// grows it.
//
// Returns the rules it repaired, by title, for the caller to report.
pub fn restore_stripped_language_prose(program: &mut Program) -> Vec<String> {
    let Some(prose) = program
        .language_requirement
        .as_ref()
        .map(|requirement| requirement.francophone.raw.clone())
    else {
        return Vec::new();
    };
    if !is_language_prose(&prose) {
        return Vec::new();
    }

    let mut repaired = Vec::new();
    for rule in all_rules(program) {
        if !is_a_stripped_language_rule(rule) {
            continue;
        }
        repaired.push(rule.title.clone());
        rule.notes.push(prose.clone());
    }
    repaired
}

// A rule holding the language placeholder alone, and no prose saying so: the
// exact shape the pre-2026-08-28 extraction left behind. A rule that still
// carries its prose is already whole, and one holding another template sigle
// (`OPT-GMC1`) is not a language rule at all.
fn is_a_stripped_language_rule(rule: &Rule) -> bool {
    let RuleCourses::List { courses } = &rule.courses else {
        return false;
    };
    courses.as_slice() == [LANGUAGE_PLACEHOLDER_CODE]
        && !rule.notes.iter().any(|note| is_language_prose(note))
}

// every rule of the program, in every scope, as one mutable walk — the
// prose form repeats once per concentration on the B-GMC
fn all_rules(program: &mut Program) -> Vec<&mut Rule> {
    let mut rules: Vec<&mut Rule> = program.rules.iter_mut().collect();
    for concentration in &mut program.concentrations {
        rules.extend(concentration.rules.iter_mut());
    }
    for profile in &mut program.profiles {
        rules.extend(profile.rules.iter_mut());
    }
    rules
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::course::Credits;
    use crate::program::{Constraint, Keyword};

    const GMC_PROSE: &str = "Réussir le cours ANL-2020 Intermediate English \
        II. L'étudiant qui démontre qu'il a acquis ce niveau (VEPT : 53) lors \
        du test administré par l'École de langues peut choisir un cours \
        d'anglais de niveau supérieur ou, s'il a acquis le niveau Advanced \
        English II (VEPT : 63), un cours d'une autre langue moderne.";

    fn course(code: &str) -> Course {
        Course {
            code: code.to_string(),
            title: code.to_string(),
            credits: Credits::Fixed(3),
            cycle: CourseCycle::First,
            prerequisites: None,
            equivalents: Vec::new(),
            seasons: Default::default(),
        }
    }

    fn catalogue() -> Vec<Course> {
        let mut courses: Vec<Course> = [
            "ANL-1010", "ANL-2010", "ANL-2020", "ANL-3010", "ANL-3020",
            "ANL-3900", "ALL-1010", "ARA-1010", "CHN-1010", "ESG-1010",
            "ITL-1010", "JAP-1010", "POR-1010", "RUS-1010", "ESP-1000",
            "FLS-2093", "GMC-1000",
        ]
        .iter()
        .map(|code| course(code))
        .collect();
        // a second-cycle course of a language subject must never be listed
        let mut graduate = course("ANL-6000");
        graduate.cycle = CourseCycle::Second;
        courses.push(graduate);
        // external data: a code whose number is not a number at all must
        // not crash the floor comparison, nor sneak into the rule
        courses.push(course("ANL-XXXX"));
        courses
    }

    // what `parser::program::take_language_rules` writes out of a prose
    // rule: the sentence as a note, its first sigle as the rule's course
    fn prose_rule(title: &str, prose: &str) -> Rule {
        let code = crate::common::first_course_code(prose)
            .expect("the fixture prose names a sigle");
        Rule {
            title: title.to_string(),
            constraint: Some(Constraint::Credits { min: 3, max: 3 }),
            courses: RuleCourses::List {
                courses: vec![code],
            },
            notes: vec![prose.to_string()],
            credits_in_addition: false,
        }
    }

    fn program_with(rules: Vec<Rule>) -> Program {
        let mut program: Program = serde_json::from_str(
            r#"{"code":"B-GMC","slug":"x","title":"x","semester":"A26",
                "cycle":1,"credits_required":120,"possible_semester_start":["A"],
                "mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .expect("fixture program");
        program.rules = rules;
        program
    }

    fn widened(program: &Program) -> Vec<String> {
        match &program.rules[0].courses {
            RuleCourses::List { courses } => courses.clone(),
            other => panic!("expected a list, got {other:?}"),
        }
    }

    #[test]
    fn the_prose_rule_lists_english_above_its_floor_and_every_language() {
        let mut program = program_with(vec![prose_rule("Règle 2", GMC_PROSE)]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        let courses = widened(&program);
        // ANL-2020 and above, never the levels below it
        assert!(courses.contains(&"ANL-2020".to_string()));
        assert!(courses.contains(&"ANL-3900".to_string()));
        assert!(!courses.contains(&"ANL-2010".to_string()));
        assert!(!courses.contains(&"ANL-1010".to_string()));
        // « une autre langue moderne »: the eight sigles the B-ANT page names
        assert!(courses.contains(&"RUS-1010".to_string()));
        assert!(courses.contains(&"ESG-1010".to_string()));
        // ESP is études hispaniques, not the École de langues' Spanish
        assert!(!courses.contains(&"ESP-1000".to_string()));
        // FLS is the non-francophone branch, not an « autre langue moderne »
        assert!(!courses.contains(&"FLS-2093".to_string()));
        // a graduate course of a listed subject never joins a bac's rule
        assert!(!courses.contains(&"ANL-6000".to_string()));
        assert!(!courses.contains(&"ANL-XXXX".to_string()));
        assert!(!courses.contains(&"GMC-1000".to_string()));
        // the language slot the cheminements types place stays listed, so
        // that placement keeps counting toward this rule
        assert!(courses.contains(&LANGUAGE_PLACEHOLDER_CODE.to_string()));
        // the prose stays displayable, the page's constraint untouched
        assert_eq!(program.rules[0].notes, vec![GMC_PROSE.to_string()]);
        assert_eq!(
            program.rules[0].constraint,
            Some(Constraint::Credits { min: 3, max: 3 })
        );
    }

    #[test]
    fn the_english_floor_is_read_from_the_prose_not_assumed() {
        // psychologie and anthropologie set their floor at ANL-3010
        let prose = "Réussir le cours ANL-3010 Advanced English I. La \
            personne étudiante qui démontre qu'elle a acquis ce niveau \
            (VEPT : 58) lors du test administré par l'École de langues a \
            satisfait l'exigence de la langue anglaise.";
        let mut program = program_with(vec![prose_rule("Règle 2", prose)]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        let courses = widened(&program);
        assert!(courses.contains(&"ANL-3010".to_string()));
        assert!(courses.contains(&"ANL-3020".to_string()));
        assert!(!courses.contains(&"ANL-2020".to_string()));
        assert!(!courses.contains(&"ANL-2010".to_string()));
        // no « langue moderne » in this sentence: none is offered
        assert!(!courses.contains(&"RUS-1010".to_string()));
    }

    #[test]
    fn every_sigle_the_sentence_names_is_kept_whatever_its_subject() {
        // génie chimique names three entry courses; agroenvironnemental
        // appends a closed list of its own
        let prose = "Réussir le cours ANL-2020 Intermediate English II ou le \
            cours ANL-3010 Advanced English I ou le cours ANL-3020 Advanced \
            English II. L'étudiant qui démontre qu'il a acquis le niveau \
            Advanced English II (VEPT : 63) lors du test administré par \
            l'École de langues peut choisir : un cours d'anglais de niveau \
            supérieur; ou un cours d'une autre langue moderne; ou un cours \
            parmi : EDC-1001.";
        let mut program = program_with(vec![prose_rule("Règle 3", prose)]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        let courses = widened(&program);
        // named but absent from the snapshot: kept, never dropped
        assert!(courses.contains(&"EDC-1001".to_string()));
        assert!(courses.contains(&"ANL-2020".to_string()));
        assert!(courses.contains(&"ITL-1010".to_string()));
    }

    #[test]
    fn ecole_des_langues_is_recognized_like_ecole_de_langues() {
        // génie géologique writes the article; the sentence names no VEPT
        // score in this fragment
        assert!(is_language_prose(
            "lors du test administré par l'École des langues"
        ));
        assert!(is_language_prose(
            "lors du test administré par l'École de langues"
        ));
        assert!(is_language_prose("(VEPT : 53)"));
        // the elective rule of génie informatique excludes low English
        // levels and is not a language rule
        assert!(!is_language_prose(
            "les cours d'anglais de niveau inférieur à ANL-2020 sont exclus"
        ));
    }

    #[test]
    fn a_prose_naming_no_english_course_adds_no_english_tier() {
        // droit states the level and no sigle: « Le niveau intermédiaire II
        // en anglais (VEPT : 53) doit être atteint pour compléter le
        // programme. » Nothing sets a floor, so no ANL is inferred.
        let prose = "Le niveau intermédiaire II en anglais (VEPT : 53) doit \
            être atteint, ou un cours d'une autre langue moderne.";
        let mut rule = prose_rule("Règle 5", GMC_PROSE);
        rule.notes = vec![prose.to_string()];
        rule.courses = RuleCourses::List {
            courses: vec!["RUS-1010".to_string()],
        };
        let mut program = program_with(vec![rule]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        let courses = widened(&program);
        assert!(courses.contains(&"RUS-1010".to_string()));
        assert!(courses.contains(&"ITL-1010".to_string()));
        assert!(
            !courses.iter().any(|code| code.starts_with("ANL-")),
            "no sigle sets an English floor: {courses:?}"
        );
    }

    #[test]
    fn a_published_list_is_never_replaced() {
        let mut rule = prose_rule("Règle 6", GMC_PROSE);
        rule.courses = RuleCourses::List {
            courses: vec!["ANL-2020".to_string(), "FLS-2093".to_string()],
        };
        let mut program = program_with(vec![rule]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        assert_eq!(
            widened(&program),
            vec!["ANL-2020".to_string(), "FLS-2093".to_string()]
        );
    }

    #[test]
    fn widening_twice_changes_nothing_the_second_time() {
        let mut program = program_with(vec![prose_rule("Règle 2", GMC_PROSE)]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");
        let once = widened(&program);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        assert_eq!(widened(&program), once);
    }

    #[test]
    fn a_rule_that_is_not_a_list_is_left_alone() {
        let mut rule = prose_rule("Règle 1", GMC_PROSE);
        rule.courses = RuleCourses::Keyword {
            courses: Keyword::Negotiated,
            raw: GMC_PROSE.to_string(),
        };
        let mut program = program_with(vec![rule]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        assert!(matches!(
            program.rules[0].courses,
            RuleCourses::Keyword { .. }
        ));
    }

    #[test]
    fn a_rule_without_language_prose_is_left_alone() {
        let mut rule = prose_rule("Règle 1", "Un cours GMC-1000, 3 crédits.");
        rule.courses = RuleCourses::List {
            courses: vec!["GMC-1000".to_string()],
        };
        let mut program = program_with(vec![rule]);
        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        assert_eq!(widened(&program), vec!["GMC-1000".to_string()]);
    }

    #[test]
    fn an_empty_catalogue_still_keeps_what_the_rule_and_the_prose_named() {
        // the sentence qualifies, but nothing in this snapshot matches: the
        // rule keeps its own course and gains only the language slot —
        // nothing the page never named is invented
        let mut program = program_with(vec![prose_rule("Règle 2", GMC_PROSE)]);
        widen_language_rules(&mut program, &[course("GMC-1000")])
            .expect("within cap");

        assert_eq!(
            widened(&program),
            vec![
                "ANL-2020".to_string(),
                LANGUAGE_PLACEHOLDER_CODE.to_string()
            ]
        );
    }

    #[test]
    fn every_scope_is_widened_not_only_the_program() {
        let mut program = program_with(Vec::new());
        program.concentrations = serde_json::from_str(
            r#"[{"title":"Robotique","credits_required":18,"mandatory":[],
                 "rules":[]}]"#,
        )
        .expect("fixture concentration");
        program.profiles = serde_json::from_str(
            r#"[{"title":"Entrepreneurial","credits_required":12,
                 "mandatory":[],"rules":[]}]"#,
        )
        .expect("fixture profile");
        program.concentrations[0].rules =
            vec![prose_rule("Règle 2", GMC_PROSE)];
        program.profiles[0].rules = vec![prose_rule("Règle 2", GMC_PROSE)];

        widen_language_rules(&mut program, &catalogue()).expect("within cap");

        for rules in
            [&program.concentrations[0].rules, &program.profiles[0].rules]
        {
            let RuleCourses::List { courses } = &rules[0].courses else {
                panic!("expected a list");
            };
            assert!(courses.len() > 1, "every scope is widened");
        }
    }

    #[test]
    fn a_catalogue_over_the_cap_is_an_error_naming_the_rule() {
        let flood: Vec<Course> = (0..=MAX_LANGUAGE_COURSES)
            .map(|n| course(&format!("RUS-{:04}", 1000 + n)))
            .collect();
        let mut program = program_with(vec![prose_rule("Règle 2", GMC_PROSE)]);

        let error = widen_language_rules(&mut program, &flood)
            .expect_err("over the cap");
        assert!(matches!(
            error,
            LanguageError::TooManyCourses { ref title, .. } if title == "Règle 2"
        ));
        assert!(error.to_string().contains("Règle 2"));
    }

    // --- the offline repair of the pre-2026-08-28 vintages ---------------

    fn stripped_program() -> Program {
        let mut rule = prose_rule("Règle 2", GMC_PROSE);
        rule.courses = RuleCourses::List {
            courses: vec![LANGUAGE_PLACEHOLDER_CODE.to_string()],
        };
        rule.notes.clear();
        let mut program = program_with(vec![rule]);
        program.language_requirement = Some(
            serde_json::from_str(&format!(
                r#"{{"francophone":{{"course":"ANL-2020","tests":[],
                     "raw":{}}}}}"#,
                serde_json::to_string(GMC_PROSE).expect("json string")
            ))
            .expect("fixture requirement"),
        );
        program
    }

    #[test]
    fn a_stripped_rule_gets_its_prose_back_and_widens() {
        let mut program = stripped_program();

        let repaired = restore_stripped_language_prose(&mut program);
        assert_eq!(repaired, vec!["Règle 2".to_string()]);
        assert_eq!(program.rules[0].notes, vec![GMC_PROSE.to_string()]);
        // the repair restores the note, never the list
        assert_eq!(
            widened(&program),
            vec![LANGUAGE_PLACEHOLDER_CODE.to_string()]
        );

        widen_language_rules(&mut program, &catalogue()).expect("within cap");
        let courses = widened(&program);
        assert!(courses.contains(&"ANL-2020".to_string()));
        assert!(courses.contains(&"RUS-1010".to_string()));
        // the placeholder survives: the cheminements types place it
        assert!(courses.contains(&LANGUAGE_PLACEHOLDER_CODE.to_string()));
    }

    #[test]
    fn another_template_sigle_is_never_taken_for_a_language_rule() {
        // « Profil entrepreneurial – Règle 3 » of the B-GMC holds OPT-GMC1
        // under the very same credits 3..3 and no note
        let mut program = stripped_program();
        program.rules[0].courses = RuleCourses::List {
            courses: vec!["OPT-GMC1".to_string()],
        };

        assert!(restore_stripped_language_prose(&mut program).is_empty());
        assert!(program.rules[0].notes.is_empty());
        widen_language_rules(&mut program, &catalogue()).expect("within cap");
        assert_eq!(widened(&program), vec!["OPT-GMC1".to_string()]);
    }

    #[test]
    fn a_rule_that_still_carries_its_prose_is_not_repaired_twice() {
        let mut program = stripped_program();
        program.rules[0].notes.push(GMC_PROSE.to_string());

        assert!(restore_stripped_language_prose(&mut program).is_empty());
        assert_eq!(program.rules[0].notes, vec![GMC_PROSE.to_string()]);
    }

    #[test]
    fn a_program_without_a_language_requirement_is_never_repaired() {
        let mut program = stripped_program();
        program.language_requirement = None;

        assert!(restore_stripped_language_prose(&mut program).is_empty());
    }

    #[test]
    fn a_requirement_whose_raw_is_not_language_prose_repairs_nothing() {
        let mut program = stripped_program();
        program.language_requirement = Some(
            serde_json::from_str(
                r#"{"francophone":{"course":"","tests":[],"raw":"rien"}}"#,
            )
            .expect("fixture requirement"),
        );

        assert!(restore_stripped_language_prose(&mut program).is_empty());
    }

    #[test]
    fn a_rule_that_is_not_a_list_is_never_repaired() {
        let mut program = stripped_program();
        program.rules[0].courses = RuleCourses::Raw {
            raw: GMC_PROSE.to_string(),
        };

        assert!(restore_stripped_language_prose(&mut program).is_empty());
    }
}
