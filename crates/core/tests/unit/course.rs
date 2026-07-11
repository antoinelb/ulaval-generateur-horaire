use ulaval_scheduler_core::{
    Component, ComponentKind, Course, Cycle, Day, Mode, PrereqTree,
    Prerequisites, ProgramCredits, Season, Section, Slot, Time,
};

// Parse a JSON literal into a comparable value, for asserting that an untagged
// enum serializes back to the exact shape it was read from.
fn as_value(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("test JSON should parse")
}

// --- Cycle: numeric level validated at the deserialization boundary ---

#[test]
fn cycle_deserializes_valid_levels() {
    let first: Cycle = serde_json::from_str("1").expect("level 1");
    let second: Cycle = serde_json::from_str("2").expect("level 2");
    assert_eq!(first, Cycle::First);
    assert_eq!(second, Cycle::Second);
}

#[test]
fn cycle_serializes_back_to_u8() {
    assert_eq!(serde_json::to_string(&Cycle::First).expect("ser"), "1");
    assert_eq!(serde_json::to_string(&Cycle::Second).expect("ser"), "2");
}

#[test]
fn cycle_rejects_out_of_range() {
    assert!(serde_json::from_str::<Cycle>("0").is_err());
    assert!(serde_json::from_str::<Cycle>("3").is_err());
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
    let midnight: Time = serde_json::from_str(r#""00:00""#).expect("00:00");
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
    let winter: Season = serde_json::from_str(r#""winter""#).expect("winter");
    let summer: Season = serde_json::from_str(r#""summer""#).expect("summer");
    assert_eq!(fall, Season::Fall);
    assert_eq!(winter, Season::Winter);
    assert_eq!(summer, Season::Summer);
}

#[test]
fn season_orders_by_academic_year() {
    assert!(Season::Fall < Season::Winter);
    assert!(Season::Winter < Season::Summer);
}

// --- Component / Mode / Day: serde rename conventions ---

#[test]
fn component_kind_round_trips_lowercase() {
    let lecture: ComponentKind =
        serde_json::from_str(r#""lecture""#).expect("lecture");
    assert_eq!(lecture, ComponentKind::Lecture);
    assert_eq!(
        serde_json::to_string(&ComponentKind::Laboratory).expect("ser"),
        r#""laboratory""#
    );
}

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
fn component_type_key_maps_to_kind() {
    let json = r#"{"type":"laboratory","sections":[]}"#;
    let component: Component = serde_json::from_str(json).expect("component");
    assert_eq!(component.kind, ComponentKind::Laboratory);
    assert!(component.sections.is_empty());
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
                program: "GEX".to_string(),
                credits: 60,
            }
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
    let prereq: Prerequisites = serde_json::from_str(json).expect("prereq");
    assert_eq!(prereq.raw, "GEX, Crédits exigés : 60");
    assert!(matches!(prereq.tree, PrereqTree::ProgramCredits { .. }));
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
    let json = r#"{"nrc":"84665","section":"A","mode":"remote","slots":[]}"#;
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
    let json = r#"{"code":"GEX-7002","title":"x","credits":3,"cycle":2,"prerequisites":null,"equivalents":[],"seasons":{"winter":{"components":[{"type":"lecture","sections":[{"nrc":"14856","section":"A","mode":"in-person","slots":[{"day":"friday","start":"08:30","end":"11:20"}]}]}]}}}"#;
    let course: Course = serde_json::from_str(json).expect("course");
    assert_eq!(course.cycle, Cycle::Second);
    let winter = course
        .seasons
        .get(&Season::Winter)
        .expect("winter offering");
    assert_eq!(winter.components.len(), 1);
    assert_eq!(winter.components[0].kind, ComponentKind::Lecture);
}
