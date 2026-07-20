use std::collections::BTreeMap;

use crate::common::Cycle;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Course {
    pub code: String,
    pub title: String,
    pub credits: u32,
    pub cycle: Cycle,
    #[serde(default)]
    pub prerequisites: Option<Prerequisites>,
    #[serde(default)]
    pub equivalents: Vec<String>,
    pub seasons: BTreeMap<Season, SeasonOffering>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Prerequisites {
    Parsed { raw: String, tree: PrereqTree },
    Raw { raw: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PrereqTree {
    Course(String),
    // an operand no rule can check automatically — an examination
    // (« Examen Test français … avec résultat de 060.0 à 100.0 », FRN-1904)
    // or a range of course numbers (« ESG-2020 à 3799 », ESP-1000) — kept
    // verbatim for the student to judge, never dropped
    Raw { raw: String },
    All { all: Vec<PrereqTree> },
    Any { any: Vec<PrereqTree> },
    ProgramCredits { program_credits: ProgramCredits },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgramCredits {
    // some pages state a credit requirement with no programme at all —
    // GEX-3333 reads « … ET  Crédits exigés : 72 », the requirement then
    // bearing on the student's own programme
    #[serde(default)]
    pub program: Option<String>,
    pub credits: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeasonOffering {
    pub groups: Vec<Vec<Section>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub nrc: String,
    pub section: Option<String>,
    pub mode: Mode,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub day: Day,
    pub start: Time,
    pub end: Time,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Season {
    Fall,
    Winter,
    Summer,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    InPerson,
    Remote,
    // part in class, part online: only the in-class meetings carry a day
    // and a time, so a hybrid section yields the in-person slots alone
    Hybrid,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

impl TryFrom<String> for Time {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let (h, m) = s
            .split_once(':')
            .ok_or_else(|| format!("invalid time (expected HH:MM) : {s}"))?;
        let hour =
            h.parse::<u8>().map_err(|_| format!("invalid hour : {s}"))?;
        let minute = m
            .parse::<u8>()
            .map_err(|_| format!("invalid minute : {s}"))?;
        if hour > 23 || minute > 59 {
            return Err(format!("time out of range : {s}"));
        }
        Ok(Time { hour, minute })
    }
}

impl From<Time> for String {
    fn from(t: Time) -> String {
        format!("{:02}:{:02}", t.hour, t.minute)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::common::Cycle;

    // Parse a JSON literal into a comparable value, for asserting that an
    // untagged enum serializes back to the exact shape it was read from.
    fn as_value(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("test JSON should parse")
    }

    // --- Time: "HH:MM" string <-> {hour, minute}, validated on the way in ---

    #[test]
    fn time_parses_and_reserializes_hh_mm() {
        let t: Time = serde_json::from_str(r#""08:30""#).expect("valid time");
        assert_eq!(
            t,
            Time {
                hour: 8,
                minute: 30
            }
        );
        assert_eq!(serde_json::to_string(&t).expect("ser"), r#""08:30""#);
    }

    #[test]
    fn time_orders_chronologically() {
        let earlier: Time = serde_json::from_str(r#""08:30""#).expect("time");
        let later: Time = serde_json::from_str(r#""12:30""#).expect("time");
        assert!(earlier < later);
    }

    #[test]
    fn time_accepts_boundary_values() {
        let midnight: Time =
            serde_json::from_str(r#""00:00""#).expect("00:00");
        let last: Time = serde_json::from_str(r#""23:59""#).expect("23:59");
        assert_eq!(midnight, Time { hour: 0, minute: 0 });
        assert_eq!(
            last,
            Time {
                hour: 23,
                minute: 59
            }
        );
    }

    #[test]
    fn time_rejects_out_of_range_and_malformed() {
        // Each input exercises a distinct rejection path:
        //   24:00, 12:60 -> parse ok, fail the hour/minute range check
        //   1230, noon   -> no ':' separator, fail at split_once
        //   ab:30, 08:cd -> colon present, but a component is not a number
        for bad in [
            r#""24:00""#,
            r#""12:60""#,
            r#""1230""#,
            r#""noon""#,
            r#""ab:30""#,
            r#""08:cd""#,
        ] {
            assert!(
                serde_json::from_str::<Time>(bad).is_err(),
                "should reject {bad}"
            );
        }
    }

    // --- Season: lowercase strings, ordered by academic year ---

    #[test]
    fn season_deserializes_lowercase() {
        let fall: Season = serde_json::from_str(r#""fall""#).expect("fall");
        let winter: Season =
            serde_json::from_str(r#""winter""#).expect("winter");
        let summer: Season =
            serde_json::from_str(r#""summer""#).expect("summer");
        assert_eq!(fall, Season::Fall);
        assert_eq!(winter, Season::Winter);
        assert_eq!(summer, Season::Summer);
    }

    #[test]
    fn season_orders_by_academic_year() {
        assert!(Season::Fall < Season::Winter);
        assert!(Season::Winter < Season::Summer);
    }

    // --- Mode / Day: serde rename conventions ---

    #[test]
    fn mode_uses_kebab_case() {
        let in_person: Mode =
            serde_json::from_str(r#""in-person""#).expect("in-person");
        assert_eq!(in_person, Mode::InPerson);
        assert_eq!(
            serde_json::to_string(&Mode::InPerson).expect("ser"),
            r#""in-person""#
        );
        assert_eq!(
            serde_json::to_string(&Mode::Remote).expect("ser"),
            r#""remote""#
        );
    }

    #[test]
    fn day_round_trips_lowercase() {
        let cases = [
            (r#""monday""#, Day::Monday),
            (r#""tuesday""#, Day::Tuesday),
            (r#""wednesday""#, Day::Wednesday),
            (r#""thursday""#, Day::Thursday),
            (r#""friday""#, Day::Friday),
            (r#""saturday""#, Day::Saturday),
            (r#""sunday""#, Day::Sunday),
        ];
        for (json, expected) in cases {
            let day: Day = serde_json::from_str(json).expect("valid day");
            assert_eq!(day, expected);
            assert_eq!(serde_json::to_string(&expected).expect("ser"), json);
        }
    }

    #[test]
    fn season_offering_holds_choice_groups() {
        // Each inner array is one choice: pick exactly one section per group
        // and union the slots. GCI-1007's shape — the lecture is forced (a
        // one-element group), one of two labs is chosen.
        let json = r#"{"groups":[[{"nrc":"84664","section":null,"mode":"in-person","slots":[]}],[{"nrc":"84665","section":"A","mode":"in-person","slots":[]},{"nrc":"84666","section":"B","mode":"in-person","slots":[]}]]}"#;
        let offering: SeasonOffering =
            serde_json::from_str(json).expect("offering");
        assert_eq!(offering.groups.len(), 2);
        assert_eq!(offering.groups[0].len(), 1);
        assert_eq!(offering.groups[1].len(), 2);
        assert_eq!(
            serde_json::to_value(&offering).expect("ser"),
            as_value(json)
        );
    }

    // --- PrereqTree: untagged ET/OU tree, each variant round-trips exactly ---

    #[test]
    fn prereq_bare_string_is_course() {
        let tree: PrereqTree =
            serde_json::from_str(r#""GLG-1000""#).expect("course leaf");
        assert_eq!(tree, PrereqTree::Course("GLG-1000".to_string()));
    }

    #[test]
    fn prereq_any_variant_round_trips() {
        let json = r#"{"any":["GGL-2600","GLG-1900"]}"#;
        let tree: PrereqTree = serde_json::from_str(json).expect("any");
        assert_eq!(
            tree,
            PrereqTree::Any {
                any: vec![
                    PrereqTree::Course("GGL-2600".to_string()),
                    PrereqTree::Course("GLG-1900".to_string()),
                ]
            }
        );
        assert_eq!(serde_json::to_value(&tree).expect("ser"), as_value(json));
    }

    #[test]
    fn prereq_all_variant_round_trips() {
        let json = r#"{"all":["GLG-1000","GLG-1900"]}"#;
        let tree: PrereqTree = serde_json::from_str(json).expect("all");
        assert!(matches!(tree, PrereqTree::All { .. }));
        assert_eq!(serde_json::to_value(&tree).expect("ser"), as_value(json));
    }

    #[test]
    fn prereq_program_credits_variant_round_trips() {
        let json = r#"{"program_credits":{"program":"GEX","credits":60}}"#;
        let tree: PrereqTree =
            serde_json::from_str(json).expect("program_credits");
        assert_eq!(
            tree,
            PrereqTree::ProgramCredits {
                program_credits: ProgramCredits {
                    program: Some("GEX".to_string()),
                    credits: 60,
                }
            }
        );
        assert_eq!(serde_json::to_value(&tree).expect("ser"), as_value(json));
    }

    #[test]
    fn prereq_raw_variant_round_trips() {
        // an operand kept verbatim is an object with a single `raw` key,
        // which no other variant claims — a course is a bare string, and a
        // group is keyed `all`/`any`
        let json = r#"{"raw":"Examen Test espagnol avec résultat de 5 à 5"}"#;
        let tree: PrereqTree = serde_json::from_str(json).expect("raw");
        assert_eq!(
            tree,
            PrereqTree::Raw {
                raw: "Examen Test espagnol avec résultat de 5 à 5".to_string()
            }
        );
        assert_eq!(serde_json::to_value(&tree).expect("ser"), as_value(json));
    }

    #[test]
    fn prereq_nested_tree_round_trips() {
        let json = r#"{"any":[{"all":["A","B"]},"C"]}"#;
        let tree: PrereqTree = serde_json::from_str(json).expect("nested");
        assert_eq!(serde_json::to_value(&tree).expect("ser"), as_value(json));
    }

    #[test]
    fn prerequisites_deserialize_raw_and_tree() {
        let json = r#"{"raw":"GEX, Crédits exigés : 60","tree":{"program_credits":{"program":"GEX","credits":60}}}"#;
        let prereq: Prerequisites =
            serde_json::from_str(json).expect("prereq");
        match prereq {
            Prerequisites::Parsed { raw, tree } => {
                assert_eq!(raw, "GEX, Crédits exigés : 60");
                assert!(matches!(tree, PrereqTree::ProgramCredits { .. }));
            }
            Prerequisites::Raw { .. } => {
                panic!("expected Parsed variant, got Raw")
            }
        }
    }

    #[test]
    fn prerequisites_without_tree_is_raw() {
        let json = r#"{"raw":"Connaissance de base"}"#;
        let prereq: Prerequisites =
            serde_json::from_str(json).expect("raw prereq");
        assert_eq!(
            prereq,
            Prerequisites::Raw {
                raw: "Connaissance de base".to_string(),
            }
        );
        assert_eq!(
            serde_json::to_value(&prereq).expect("ser"),
            as_value(json)
        );
    }

    #[test]
    fn prerequisites_with_malformed_tree_falls_back_to_raw() {
        // Untagged variants are tried in declaration order: Parsed first,
        // then Raw. A "tree" key that doesn't match PrereqTree's shape
        // fails the Parsed attempt, and Raw ignores unknown fields — so a
        // corrupted tree degrades to raw-only instead of an error
        // (ADR 2026-07-prealables-hors-grammaire-en-enum).
        let json = r#"{"raw":"x","tree":{"bogus":true}}"#;
        let prereq: Prerequisites =
            serde_json::from_str(json).expect("degrades to raw");
        assert_eq!(
            prereq,
            Prerequisites::Raw {
                raw: "x".to_string(),
            }
        );
    }

    // --- Section: optional section identifier ---

    #[test]
    fn section_null_becomes_none() {
        let json =
            r#"{"nrc":"84664","section":null,"mode":"in-person","slots":[]}"#;
        let section: Section = serde_json::from_str(json).expect("section");
        assert_eq!(section.section, None);
        assert_eq!(section.mode, Mode::InPerson);
        assert!(section.slots.is_empty());
    }

    #[test]
    fn section_present_becomes_some() {
        let json =
            r#"{"nrc":"84665","section":"A","mode":"remote","slots":[]}"#;
        let section: Section = serde_json::from_str(json).expect("section");
        assert_eq!(section.section.as_deref(), Some("A"));
    }

    #[test]
    fn slot_holds_day_and_times() {
        let json = r#"{"day":"friday","start":"12:30","end":"15:20"}"#;
        let slot: Slot = serde_json::from_str(json).expect("slot");
        assert_eq!(slot.day, Day::Friday);
        assert_eq!(
            slot.start,
            Time {
                hour: 12,
                minute: 30
            }
        );
        assert_eq!(
            slot.end,
            Time {
                hour: 15,
                minute: 20
            }
        );
        assert!(slot.start < slot.end);
    }

    // --- Course: serde `default` behavior and the Season-keyed map ---

    #[test]
    fn course_missing_prerequisites_and_equivalents_default() {
        let json = r#"{"code":"ECN-4901","title":"x","credits":3,"cycle":1,"seasons":{}}"#;
        let course: Course = serde_json::from_str(json).expect("course");
        assert_eq!(course.prerequisites, None);
        assert!(course.equivalents.is_empty());
        assert!(course.seasons.is_empty());
    }

    #[test]
    fn course_null_prerequisites_is_none() {
        let json = r#"{"code":"ECN-4901","title":"x","credits":3,"cycle":1,"prerequisites":null,"equivalents":["ECN-6901"],"seasons":{}}"#;
        let course: Course = serde_json::from_str(json).expect("course");
        assert_eq!(course.prerequisites, None);
        assert_eq!(course.equivalents, vec!["ECN-6901".to_string()]);
    }

    #[test]
    fn course_deserializes_season_keyed_map() {
        let json = r#"{"code":"GEX-7002","title":"x","credits":3,"cycle":2,"prerequisites":null,"equivalents":[],"seasons":{"winter":{"groups":[[{"nrc":"14856","section":"A","mode":"in-person","slots":[{"day":"friday","start":"08:30","end":"11:20"}]}]]}}}"#;
        let course: Course = serde_json::from_str(json).expect("course");
        assert_eq!(course.cycle, Cycle::Second);
        let winter = course
            .seasons
            .get(&Season::Winter)
            .expect("winter offering");
        assert_eq!(winter.groups.len(), 1);
        assert_eq!(winter.groups[0][0].nrc, "14856");
    }
}
