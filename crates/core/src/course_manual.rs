// The hand-maintained companion of `data/cours.json` — never written by the
// scraper. It carries two things the répertoire cannot give: courses no
// catalogue page describes (the `OPT-*` placeholders, the `EHE-*`
// exchanges), and, per admission vintage, the prerequisites a course had
// under an older version of the program (ADR
// `2026-08-correction-des-prealables-par-millesime`).
//
// It stays a single file, unlike the program snapshots that split one per
// vintage: a manual file follows the shape of its scraped counterpart, and
// `cours.json` is one vintage-less snapshot.
use std::collections::BTreeMap;

use crate::course::Course;
use crate::prereq_override::PrereqOverride;
use crate::program::Semester;

#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(default, deny_unknown_fields)]
pub struct CourseManual {
    pub courses: Vec<Course>,
    // keyed by admission vintage — « A24 ». A plain `String` because
    // `Semester` is deliberately unordered (an `Ord` derive would rank A26
    // before H25); `malformed_vintages` is what keeps the keys honest.
    pub vintages: BTreeMap<String, VintageOverlay>,
}

#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(default, deny_unknown_fields)]
pub struct VintageOverlay {
    // course code → the prerequisites expression in force under this
    // vintage, in the same grammar the répertoire writes. Empty means the
    // course had none.
    pub prerequisites: BTreeMap<String, String>,
}

impl CourseManual {
    // The corrections one admission vintage carries, ready for
    // `apply_prereq_overrides`. No `official` is recorded: this overlay is
    // *meant* to differ from today's répertoire, so the staleness warning
    // would be noise on every entry.
    pub fn overrides_for(
        &self,
        semester: &str,
    ) -> BTreeMap<String, PrereqOverride> {
        self.vintages
            .get(semester)
            .map(|overlay| {
                overlay
                    .prerequisites
                    .iter()
                    .map(|(code, text)| {
                        (
                            code.clone(),
                            PrereqOverride {
                                text: text.clone(),
                                official: None,
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    // A vintage key that names no real semester applies to nobody — and
    // would do so in silence, which is the one thing this project never
    // does. A key naming a vintage no student picked is *not* an error:
    // the file may well describe A24 while everyone here is in A26.
    pub fn malformed_vintages(&self) -> Vec<String> {
        self.vintages
            .keys()
            .filter(|key| key.parse::<Semester>().is_err())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "courses": [
    {
      "code": "AUC-HOIX", "title": "Cours de 1er cycle au choix",
      "credits": 3, "cycle": 1, "prerequisites": null, "equivalents": [],
      "seasons": {
        "fall":   {"last_offered": null, "options": null},
        "winter": {"last_offered": null, "options": null},
        "summer": {"last_offered": null, "options": null}
      }
    }
  ],
  "vintages": {
    "A24": {
      "prerequisites": {
        "GCI-2000": "GCI-1000 ET MAT-1902",
        "GCI-3000": ""
      }
    }
  }
}"#;

    #[test]
    fn the_file_carries_courses_and_vintage_corrections_side_by_side() {
        let manual: CourseManual =
            serde_json::from_str(SAMPLE).expect("the sample parses");
        assert_eq!(manual.courses[0].code, "AUC-HOIX");
        let corrections = manual.overrides_for("A24");
        assert_eq!(corrections.len(), 2);
        assert_eq!(corrections["GCI-2000"].text, "GCI-1000 ET MAT-1902");
        assert_eq!(
            corrections["GCI-2000"].official, None,
            "a vintage overlay is meant to differ from today's répertoire"
        );
        assert_eq!(
            corrections["GCI-3000"].text, "",
            "an empty expression is an answer: this course had none"
        );
    }

    #[test]
    fn a_vintage_nobody_picked_yields_no_correction() {
        let manual: CourseManual =
            serde_json::from_str(SAMPLE).expect("the sample parses");
        assert!(manual.overrides_for("A26").is_empty());
        assert!(manual.malformed_vintages().is_empty());
    }

    #[test]
    fn an_absent_file_is_an_empty_one() {
        let manual: CourseManual =
            serde_json::from_str("{}").expect("an empty object parses");
        assert_eq!(manual, CourseManual::default());
        assert!(manual.overrides_for("A24").is_empty());
    }

    #[test]
    fn a_vintage_key_naming_no_semester_is_surfaced() {
        let raw = r#"{"vintages": {"A24": {"prerequisites": {}},
                                   "2024": {"prerequisites": {}}}}"#;
        let manual: CourseManual =
            serde_json::from_str(raw).expect("the sample parses");
        assert_eq!(manual.malformed_vintages(), ["2024"]);
    }

    #[test]
    fn an_unknown_key_refuses_to_parse_rather_than_doing_nothing() {
        let error = serde_json::from_str::<CourseManual>(
            r#"{"vintages": {"A24": {"prealables": {}}}}"#,
        )
        .expect_err("a misspelled key must refuse to parse");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn the_file_round_trips_whole() {
        let manual: CourseManual =
            serde_json::from_str(SAMPLE).expect("the sample parses");
        let written =
            serde_json::to_string(&manual).expect("the manual serializes");
        assert_eq!(
            serde_json::from_str::<CourseManual>(&written)
                .expect("the rewritten form parses"),
            manual
        );
    }
}
