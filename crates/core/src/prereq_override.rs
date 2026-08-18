// A course's prerequisites as they stood under an older program vintage.
// The répertoire publishes one version — today's — but a student stays
// governed by the version he was admitted under, and the snapshot has no
// room to say so (ADR `2026-08-correction-des-prealables-par-millesime`).
//
// A correction is applied to the `Course` values themselves, before the
// solver ever sees them: `flat_tree`, `parsed_tree` and `preparatory_rule`
// are the only readers of `Course::prerequisites`, so rewriting the field
// covers the whole engine without a single solver signature moving.
use std::collections::BTreeMap;

use crate::course::{Course, Prerequisites};
use crate::prereq_parse::{parse_prereq_tree, PrereqParseError};

#[derive(
    Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct PrereqOverride {
    // the rewritten expression, in the grammar the répertoire itself uses.
    // Empty means « this course has no prerequisites » — a real answer, not
    // a missing one.
    pub text: String,
    // the official raw at the moment the correction was written, kept only
    // to notice that the répertoire has moved since. `None` for a
    // correction that comes from a vintage file: that one is *meant* to
    // differ from today's snapshot, so a warning would be noise.
    pub official: Option<String>,
}

// What the application could not do, or did while something looked wrong.
// Nothing here is ever swallowed: an override the catalogue cannot honour
// is surfaced, never dropped.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OverrideNote {
    // the rewritten text is not an expression the grammar reads; the course
    // keeps its official prerequisites. Unlike the scraper, a correction
    // never falls back to `Prerequisites::Raw`: that variant preserves what
    // the university wrote, it is not a place to park a typo.
    Unparsed {
        code: String,
        error: String,
    },
    // a correction naming a course no catalogue holds
    UnknownCode {
        code: String,
    },
    // the official prerequisites moved since the correction was written —
    // applying it silently would mask a real change in the répertoire
    OfficialChanged {
        code: String,
        was: String,
        now: String,
    },
}

pub fn apply_prereq_overrides(
    courses: &mut [Course],
    overrides: &BTreeMap<String, PrereqOverride>,
) -> Vec<OverrideNote> {
    let position: BTreeMap<&str, usize> = courses
        .iter()
        .enumerate()
        .map(|(index, course)| (course.code.as_str(), index))
        .collect();
    // resolved first so the index borrow ends before the courses are
    // written through
    let targets: Vec<(&String, &PrereqOverride, Option<usize>)> = overrides
        .iter()
        .map(|(code, correction)| {
            (code, correction, position.get(code.as_str()).copied())
        })
        .collect();

    let mut notes = Vec::new();
    for (code, correction, index) in targets {
        let Some(index) = index else {
            notes.push(OverrideNote::UnknownCode { code: code.clone() });
            continue;
        };
        let course = &mut courses[index];

        if let Some(was) = &correction.official {
            let now = official_raw(course);
            if was != now {
                notes.push(OverrideNote::OfficialChanged {
                    code: code.clone(),
                    was: was.clone(),
                    now: now.to_string(),
                });
            }
        }

        let text = correction.text.trim();
        if text.is_empty() {
            course.prerequisites = None;
            continue;
        }
        match parse_prereq_tree(text) {
            Ok(tree) => {
                course.prerequisites = Some(Prerequisites::Parsed {
                    raw: text.to_string(),
                    tree,
                });
            }
            Err(PrereqParseError { error, .. }) => {
                notes.push(OverrideNote::Unparsed {
                    code: code.clone(),
                    error,
                });
            }
        }
    }
    notes
}

// A course with no prerequisites reads as the empty expression — the same
// thing an emptied correction writes, so the two compare without a special
// case.
fn official_raw(course: &Course) -> &str {
    match &course.prerequisites {
        Some(
            Prerequisites::Parsed { raw, .. } | Prerequisites::Raw { raw },
        ) => raw,
        None => "",
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use crate::course::PrereqTree;

    use super::*;

    fn course(code: &str, prerequisites: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":{prerequisites},"equivalents":[],
                 "seasons":{{}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn parsed(raw: &str) -> String {
        format!(r#"{{"raw":"{raw}","tree":"{raw}"}}"#)
    }

    fn correction(text: &str) -> PrereqOverride {
        PrereqOverride {
            text: text.to_string(),
            official: None,
        }
    }

    fn overrides(
        entries: [(&str, PrereqOverride); 1],
    ) -> BTreeMap<String, PrereqOverride> {
        entries
            .into_iter()
            .map(|(code, value)| (code.to_string(), value))
            .collect()
    }

    #[test]
    fn a_rewritten_expression_replaces_the_tree() {
        let mut courses = vec![course("GCI-2000", &parsed("GCI-1005"))];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([(
                "GCI-2000",
                correction("GCI-1000 ET (MAT-1900 OU MAT-1902)"),
            )]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        let Some(Prerequisites::Parsed { raw, tree }) =
            &courses[0].prerequisites
        else {
            panic!(
                "expected a parsed tree, got {:?}",
                courses[0].prerequisites
            )
        };
        assert_eq!(raw, "GCI-1000 ET (MAT-1900 OU MAT-1902)");
        assert_eq!(
            tree,
            &PrereqTree::All {
                all: vec![
                    PrereqTree::Course("GCI-1000".to_string()),
                    PrereqTree::Any {
                        any: vec![
                            PrereqTree::Course("MAT-1900".to_string()),
                            PrereqTree::Course("MAT-1902".to_string()),
                        ]
                    },
                ]
            }
        );
    }

    #[test]
    fn an_emptied_correction_means_no_prerequisites_at_all() {
        let mut courses = vec![course("GCI-2000", &parsed("GCI-1005"))];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([("GCI-2000", correction("   "))]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(courses[0].prerequisites, None);
    }

    #[test]
    fn an_unreadable_correction_leaves_the_course_untouched_and_is_reported() {
        let official = parsed("GCI-1005");
        let mut courses = vec![course("GCI-2000", &official)];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([("GCI-2000", correction("GCI-1000 ET"))]),
        );
        assert_eq!(
            notes,
            [OverrideNote::Unparsed {
                code: "GCI-2000".to_string(),
                error: "expression ends on an operator".to_string(),
            }]
        );
        // never `Prerequisites::Raw`: the official tree survives intact
        assert_eq!(courses[0], course("GCI-2000", &official));
    }

    #[test]
    fn a_correction_naming_a_course_no_catalogue_holds_is_reported() {
        let mut courses = vec![course("GCI-2000", "null")];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([("XXX-9999", correction("GCI-1000"))]),
        );
        assert_eq!(
            notes,
            [OverrideNote::UnknownCode {
                code: "XXX-9999".to_string(),
            }]
        );
        assert_eq!(courses[0].prerequisites, None);
    }

    #[test]
    fn an_official_that_moved_since_the_correction_is_reported_and_applied() {
        let mut courses = vec![course("GCI-2000", &parsed("GCI-1099"))];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([(
                "GCI-2000",
                PrereqOverride {
                    text: "GCI-1000".to_string(),
                    official: Some("GCI-1005".to_string()),
                },
            )]),
        );
        assert_eq!(
            notes,
            [OverrideNote::OfficialChanged {
                code: "GCI-2000".to_string(),
                was: "GCI-1005".to_string(),
                now: "GCI-1099".to_string(),
            }],
            "the student is warned, but his correction still stands"
        );
        assert!(matches!(
            &courses[0].prerequisites,
            Some(Prerequisites::Parsed { raw, .. }) if raw == "GCI-1000"
        ));
    }

    #[test]
    fn an_official_the_repertoire_left_as_text_compares_all_the_same() {
        // `Prerequisites::Raw` — an expression outside the grammar. It is
        // still what the correction replaced, so it still has to compare.
        let mut courses =
            vec![course("GCI-2000", r#"{"raw":"Autorisation"}"#)];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([(
                "GCI-2000",
                PrereqOverride {
                    text: "GCI-1000".to_string(),
                    official: Some("Autorisation".to_string()),
                },
            )]),
        );
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn an_official_still_matching_says_nothing() {
        let mut courses = vec![course("GCI-2000", &parsed("GCI-1005"))];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([(
                "GCI-2000",
                PrereqOverride {
                    text: "GCI-1000".to_string(),
                    official: Some("GCI-1005".to_string()),
                },
            )]),
        );
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn a_course_with_no_prerequisites_reads_as_the_empty_expression() {
        let mut courses = vec![course("GCI-2000", "null")];
        let notes = apply_prereq_overrides(
            &mut courses,
            &overrides([(
                "GCI-2000",
                PrereqOverride {
                    text: "GCI-1000".to_string(),
                    official: Some(String::new()),
                },
            )]),
        );
        assert!(notes.is_empty(), "{notes:?}");
    }
}
