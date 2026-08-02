// The « Scolarité préparatoire » rule: every préuniversitaire course (0xxx)
// transitively reachable from a program's mandatory courses' prerequisite
// trees. No page lists them — they hide inside the trees — so the scraper
// computes the rule here (pure, no IO) and appends it to each scraped
// program (ADR `2026-08-regle-scolarite-preparatoire`).

use std::collections::{BTreeMap, BTreeSet};

use crate::course::{is_preuniversity, Course, PrereqTree, Prerequisites};
use crate::program::{Rule, RuleCourses};

// same budget as `organigramme::MAX_TREE_NODES`: bounds the worklist over
// course codes and tree nodes alike
const MAX_VISITED: usize = 10_000;

const RULE_TITLE: &str = "Scolarité préparatoire";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreparatoryError {
    #[error(
        "the prerequisite graph reachable from the mandatory courses \
         exceeds {MAX_VISITED} nodes"
    )]
    GraphTooLarge,
}

// a worklist entry: a course code to resolve against the snapshot, or a
// prerequisite subtree to walk
#[derive(Clone, Copy)]
enum Node<'a> {
    Code(&'a str),
    Tree(&'a PrereqTree),
}

// No constraint on the returned rule: which cours d'appoint apply depends
// on each student's collegial record — the rule exists so the right ones
// get taken, not to count anything. `None` when nothing is reachable: the
// rule is omitted, not emitted empty.
pub fn preparatory_rule(
    mandatory: &[String],
    courses: &[Course],
) -> Result<Option<Rule>, PreparatoryError> {
    let by_code: BTreeMap<&str, &Course> = courses
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let mandatory_set: BTreeSet<&str> =
        mandatory.iter().map(String::as_str).collect();

    let mut pending: Vec<Node<'_>> =
        mandatory.iter().map(|code| Node::Code(code)).collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut found: BTreeSet<String> = BTreeSet::new();

    for cursor in 0..MAX_VISITED {
        if cursor >= pending.len() {
            return Ok(preparatory_from(found));
        }
        match pending[cursor] {
            Node::Code(code) => {
                // cycles among prerequisites terminate here
                if !seen.insert(code) {
                    continue;
                }
                // a mandatory 0xxx is already in the program, not
                // préparatoire; a 0xxx absent from the snapshot is still
                // listed — the 0 prefix is the only cycle signal we have
                // (ADR `2026-07-presomption-limitee-au-preuniversitaire`)
                if is_preuniversity(code) && !mandatory_set.contains(code) {
                    found.insert(code.to_string());
                }
                // absent courses and whole-`Raw` prerequisites walk nothing
                if let Some(Prerequisites::Parsed { tree, .. }) = by_code
                    .get(code)
                    .and_then(|course| course.prerequisites.as_ref())
                {
                    pending.push(Node::Tree(tree));
                }
            }
            Node::Tree(tree) => match tree {
                PrereqTree::Course(code) => pending.push(Node::Code(code)),
                // cégep sigles (« BIO-NYA ») and prose are not course
                // codes; a credit threshold names no course either
                PrereqTree::Raw { .. } | PrereqTree::ProgramCredits { .. } => {
                }
                // OR branches deliberately collapsed: every named course
                // enters the rule, whichever branch holds it
                PrereqTree::All { all: children }
                | PrereqTree::Any { any: children } => {
                    pending.extend(children.iter().map(Node::Tree));
                }
            },
        }
    }
    Err(PreparatoryError::GraphTooLarge)
}

fn preparatory_from(found: BTreeSet<String>) -> Option<Rule> {
    if found.is_empty() {
        return None;
    }
    Some(Rule {
        title: RULE_TITLE.to_string(),
        constraint: None,
        // the BTreeSet already sorted and deduplicated the codes
        courses: RuleCourses::List {
            courses: found.into_iter().collect(),
        },
        notes: Vec::new(),
        credits_in_addition: false,
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::course::{CourseCycle, Credits};

    fn course(code: &str, prerequisites: Option<Prerequisites>) -> Course {
        Course {
            code: code.to_string(),
            title: "x".to_string(),
            credits: Credits::Fixed(3),
            cycle: CourseCycle::First,
            prerequisites,
            equivalents: Vec::new(),
            seasons: BTreeMap::new(),
        }
    }

    fn parsed(tree: PrereqTree) -> Option<Prerequisites> {
        Some(Prerequisites::Parsed {
            raw: "x".to_string(),
            tree,
        })
    }

    fn listed(rule: &Rule) -> &[String] {
        match &rule.courses {
            RuleCourses::List { courses } => courses,
            other => panic!("expected a course list, got {other:?}"),
        }
    }

    #[test]
    fn a_chain_through_a_preuniversity_course_is_followed_raw_leaves_not() {
        let courses = vec![
            course(
                "GEX-1000",
                parsed(PrereqTree::Any {
                    any: vec![
                        PrereqTree::Course("PHY-0250".to_string()),
                        PrereqTree::Raw {
                            raw: "BIO-NYA".to_string(),
                        },
                    ],
                }),
            ),
            course(
                "PHY-0250",
                parsed(PrereqTree::Course("PHY-0150".to_string())),
            ),
        ];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("within budget")
            .expect("préuniversitaire courses found");
        assert_eq!(rule.title, "Scolarité préparatoire");
        assert_eq!(rule.constraint, None);
        assert_eq!(listed(&rule), ["PHY-0150", "PHY-0250"]);
        assert!(!rule.credits_in_addition);
    }

    #[test]
    fn branches_are_collapsed_and_duplicates_deduplicated_sorted() {
        let courses = vec![course(
            "GEX-1000",
            parsed(PrereqTree::All {
                all: vec![
                    PrereqTree::Any {
                        any: vec![
                            PrereqTree::Course("MAT-0150".to_string()),
                            PrereqTree::Course("MAT-0130".to_string()),
                        ],
                    },
                    PrereqTree::Course("MAT-0150".to_string()),
                ],
            }),
        )];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("within budget")
            .expect("préuniversitaire courses found");
        assert_eq!(listed(&rule), ["MAT-0130", "MAT-0150"]);
    }

    #[test]
    fn a_university_intermediate_is_walked_through() {
        let courses = vec![
            course(
                "GEX-1000",
                parsed(PrereqTree::Course("MAT-1900".to_string())),
            ),
            course(
                "MAT-1900",
                parsed(PrereqTree::Course("MAT-0260".to_string())),
            ),
        ];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("within budget")
            .expect("préuniversitaire courses found");
        assert_eq!(listed(&rule), ["MAT-0260"]);
    }

    #[test]
    fn raw_prerequisites_and_credit_thresholds_walk_nothing() {
        let courses = vec![
            course(
                "GEX-1000",
                Some(Prerequisites::Raw {
                    raw: "MAT-0130 hidden in prose".to_string(),
                }),
            ),
            course(
                "GEX-2000",
                parsed(PrereqTree::ProgramCredits {
                    program_credits: crate::course::ProgramCredits {
                        program: None,
                        credits: 72,
                    },
                }),
            ),
        ];
        let mandatory = ["GEX-1000".to_string(), "GEX-2000".to_string()];
        let rule =
            preparatory_rule(&mandatory, &courses).expect("within budget");
        assert_eq!(rule, None);
    }

    #[test]
    fn a_preuniversity_code_absent_from_the_snapshot_is_still_listed() {
        let courses = vec![course(
            "GEX-1000",
            parsed(PrereqTree::Course("CHM-0150".to_string())),
        )];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("within budget")
            .expect("préuniversitaire courses found");
        assert_eq!(listed(&rule), ["CHM-0150"]);
    }

    #[test]
    fn a_mandatory_preuniversity_course_is_not_listed() {
        let courses = vec![course(
            "MAT-0150",
            parsed(PrereqTree::Course("MAT-0130".to_string())),
        )];
        let rule = preparatory_rule(&["MAT-0150".to_string()], &courses)
            .expect("within budget")
            .expect("préuniversitaire courses found");
        assert_eq!(listed(&rule), ["MAT-0130"]);
    }

    #[test]
    fn nothing_reachable_yields_no_rule_at_all() {
        let courses = vec![course("GEX-1000", None)];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("within budget");
        assert_eq!(rule, None);
    }

    #[test]
    fn a_prerequisite_cycle_terminates() {
        let courses = vec![
            course(
                "GEX-1000",
                parsed(PrereqTree::Course("GEX-2000".to_string())),
            ),
            course(
                "GEX-2000",
                parsed(PrereqTree::Course("GEX-1000".to_string())),
            ),
        ];
        let rule = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect("a cycle stays within budget");
        assert_eq!(rule, None);
    }

    #[test]
    fn a_graph_over_budget_is_a_typed_error() {
        let leaves = (0..MAX_VISITED)
            .map(|i| PrereqTree::Course(format!("GEX-{i:04}")))
            .collect();
        let courses =
            vec![course("GEX-1000", parsed(PrereqTree::All { all: leaves }))];
        let error = preparatory_rule(&["GEX-1000".to_string()], &courses)
            .expect_err("over budget");
        assert_eq!(error, PreparatoryError::GraphTooLarge);
        assert!(error.to_string().contains("exceeds 10000 nodes"));
    }
}
