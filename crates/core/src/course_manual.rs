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
    // course code → the prerequisites expression that holds under *every*
    // vintage, in the same grammar the répertoire writes. This is the
    // répertoire being wrong, not merely out of date for one admission
    // year, so it applies before a program is even chosen. Empty means the
    // course has none.
    pub prerequisites: BTreeMap<String, String>,
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
    // Every correction that governs a student admitted in `semester`,
    // ready for `apply_prereq_overrides`: the vintage-less ones and, over
    // them, the ones written for that vintage. An unknown or empty
    // `semester` still gets the vintage-less layer — the répertoire being
    // wrong does not wait for a program to be chosen.
    pub fn overrides_for(
        &self,
        semester: &str,
    ) -> BTreeMap<String, PrereqOverride> {
        // the vintage-less corrections first, the vintage's own on top: a
        // correction written for one admission year is the more specific
        // answer, and wins where both name a course
        let mut overrides = as_overrides(&self.prerequisites);
        if let Some(overlay) = self.vintages.get(semester) {
            overrides.extend(as_overrides(&overlay.prerequisites));
        }
        overrides
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

// No `official` is recorded on either layer: a manual correction is *meant*
// to differ from the snapshot it corrects, so the staleness warning would
// fire on every entry.
fn as_overrides(
    prerequisites: &BTreeMap<String, String>,
) -> BTreeMap<String, PrereqOverride> {
    prerequisites
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
  "prerequisites": {
    "GCI-2000": "GCI-1000",
    "GMC-4000": "GMC-1000 ET  Crédits exigés : 72"
  },
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
        assert_eq!(corrections.len(), 3, "both layers, merged");
        assert_eq!(
            corrections["GCI-2000"].text, "GCI-1000 ET MAT-1902",
            "the vintage is the more specific answer and wins"
        );
        assert_eq!(
            corrections["GMC-4000"].text, "GMC-1000 ET  Crédits exigés : 72",
            "a vintage-less correction holds under every vintage"
        );
        assert_eq!(
            corrections["GCI-2000"].official, None,
            "a vintage overlay is meant to differ from today's répertoire"
        );
        assert_eq!(
            corrections["GCI-3000"].text, "",
            "an empty expression is an answer: this course had none"
        );
    }

    // « the répertoire is wrong » does not wait for a program to be chosen
    #[test]
    fn a_vintage_nobody_picked_keeps_the_vintage_less_corrections() {
        let manual: CourseManual =
            serde_json::from_str(SAMPLE).expect("the sample parses");
        for vintage in ["A26", ""] {
            let corrections = manual.overrides_for(vintage);
            assert_eq!(
                corrections.keys().collect::<Vec<_>>(),
                ["GCI-2000", "GMC-4000"],
                "{vintage}: the vintage-less layer alone"
            );
            assert_eq!(corrections["GCI-2000"].text, "GCI-1000");
        }
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
