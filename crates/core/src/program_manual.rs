use crate::program::Semester;

// The hand-maintained companion of one program snapshot —
// `data/programmes/{code}-{semester}.manuel.json`, the scraped file's own
// name plus the suffix, never written by the scraper (ADRs
// `2026-07-cheminement-type-en-fichier-manuel`,
// `2026-08-fichier-manuel-de-programme-millesime`). It carries the
// cheminements types: no machine-readable source exists, so each grid is
// encoded by hand — one file per admission vintage, one entry per variant
// inside it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct ProgramManual {
    pub cheminements_types: Vec<CheminementType>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct CheminementType {
    // what tells the variants of one admission apart — profile, technique
    // DEC, five-year pace — exactly as shown to the student. Empty when the
    // vintage holds a single variant with nothing to tell apart: the file
    // name already says which admission it is.
    pub label: String,
    // courses granted on admission (a technique DEC's recognitions) — the
    // grid's « cours complétés » column
    pub completed: Vec<String>,
    pub sessions: Vec<CheminementSession>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct CheminementSession {
    pub semester: Semester,
    // empty = the timeline holds the slot (an off summer, an alignment row
    // before the admission) with nothing scheduled
    pub courses: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manual_file_round_trips_whole() {
        let raw = r#"{
  "cheminements_types": [
    {
      "label": "Technique de génie mécanique",
      "completed": ["GMC-1024", "OPT-ION3"],
      "sessions": [
        { "semester": "A26", "courses": [] },
        { "semester": "H27", "courses": ["GMC-1001", "MAT-1900"] }
      ]
    }
  ]
}"#;
        let manual: ProgramManual =
            serde_json::from_str(raw).expect("the sample parses");
        let cheminement = manual.cheminements_types[0].clone();
        assert_eq!(cheminement.label, "Technique de génie mécanique");
        assert_eq!(cheminement.completed, ["GMC-1024", "OPT-ION3"]);
        assert_eq!(cheminement.sessions[0].courses, Vec::<String>::new());
        assert_eq!(cheminement.sessions[1].semester.to_string(), "H27");
        let reserialized = serde_json::to_string_pretty(&manual)
            .expect("the manual serializes");
        assert_eq!(
            serde_json::from_str::<ProgramManual>(&reserialized)
                .expect("the reserialized form parses"),
            manual
        );
    }

    #[test]
    fn a_bad_semester_is_a_parse_error_not_a_silent_default() {
        let raw = r#"{
  "cheminements_types": [
    { "label": "", "completed": [],
      "sessions": [{ "semester": "X99", "courses": [] }] }
  ]
}"#;
        let error = serde_json::from_str::<ProgramManual>(raw)
            .expect_err("an unknown semester must refuse to parse");
        assert!(error.to_string().contains("unknown semester"), "{error}");
    }
}
