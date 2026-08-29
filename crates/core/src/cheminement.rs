use crate::program::Semester;

// One cheminement type: the session-by-session grid a program publishes for
// one admission. No machine-readable source exists, so each grid is encoded
// by hand — one file per cheminement under `data/cheminements/`, named
// `{code}-{semester}[-{concentration}].json`. The name carries the program,
// the vintage and the variant; the document itself is only the grid, so a
// file the student is handed and a file the app exports have the very same
// shape (ADR `2026-08-un-cheminement-par-fichier`).
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Cheminement {
    // courses granted on admission (a technique DEC's recognitions) — the
    // grid's « cours complétés » column, credited without a session
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
    fn a_cheminement_file_round_trips_whole() {
        let raw = r#"{
  "completed": ["GMC-1024", "OPT-ION3"],
  "sessions": [
    { "semester": "A26", "courses": [] },
    { "semester": "H27", "courses": ["GMC-1001", "MAT-1900"] }
  ]
}"#;
        let cheminement: Cheminement =
            serde_json::from_str(raw).expect("the sample parses");
        assert_eq!(cheminement.completed, ["GMC-1024", "OPT-ION3"]);
        assert_eq!(cheminement.sessions[0].courses, Vec::<String>::new());
        assert_eq!(cheminement.sessions[1].semester.to_string(), "H27");
        let reserialized = serde_json::to_string_pretty(&cheminement)
            .expect("the cheminement serializes");
        assert_eq!(
            serde_json::from_str::<Cheminement>(&reserialized)
                .expect("the reserialized form parses"),
            cheminement
        );
    }

    #[test]
    fn a_bad_semester_is_a_parse_error_not_a_silent_default() {
        let raw = r#"{
  "completed": [],
  "sessions": [{ "semester": "X99", "courses": [] }]
}"#;
        let error = serde_json::from_str::<Cheminement>(raw)
            .expect_err("an unknown semester must refuse to parse");
        assert!(error.to_string().contains("unknown semester"), "{error}");
    }

    // an exported file carries provenance the reader has no field for; it
    // must ride along unread rather than refuse the whole document (ADR
    // `2026-08-un-cheminement-par-fichier`)
    #[test]
    fn an_unknown_field_is_ignored_not_refused() {
        let raw = r#"{
  "completed": [],
  "sessions": [],
  "provenance": { "exported_at": "2026-08-29T14:32:00-04:00" }
}"#;
        let cheminement: Cheminement = serde_json::from_str(raw)
            .expect("a provenance block does not break the read");
        assert_eq!(cheminement, Cheminement::default());
    }
}
