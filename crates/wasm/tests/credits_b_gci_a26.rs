use std::collections::BTreeSet;

use ulaval_scheduler_core::{
    apply_prereq_overrides, Course, CourseManual, Program,
};
use ulaval_scheduler_wasm::credits::credit_summary;

// ADR `2026-08-le-profil-napporte-jamais-de-credits-neufs` : Antoine's
// 2026-08-27 report — 129/120 cr filling B-GCI + a concentration + Profil
// développement durable — verified against the real, committed snapshot
// rather than a hand-built fixture, since the répertoire's own cross-
// listing turned out to matter (see the second test's comment).
fn catalogue() -> Vec<Course> {
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/cours.json"))
            .expect("the committed catalogue parses");
    let mut courses: Vec<Course> =
        serde_json::from_value(catalogue["courses"].clone())
            .expect("the catalogue carries courses");
    let manual: CourseManual =
        serde_json::from_str(include_str!("../../../data/cours.manuel.json"))
            .expect("the manual catalogue parses");
    courses.extend(manual.courses.clone());
    courses.sort_by(|left, right| left.code.cmp(&right.code));
    let notes =
        apply_prereq_overrides(&mut courses, &manual.overrides_for("A26"));
    assert!(
        notes.is_empty(),
        "all committed corrections apply: {notes:?}"
    );
    courses
}

fn b_gci() -> Program {
    serde_json::from_str(include_str!(
        "../../../data/programmes/B-GCI-A26.json"
    ))
    .expect("the B-GCI A26 snapshot parses")
}

fn codes(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|code| code.to_string()).collect()
}

// The trunk's own 99 cr (32 mandatory + « Règle 1 »'s GMN-2901, the
// cheapest of its three options) plus « Autres exigences – Règle 1 »'s
// language course — every scenario below builds on this fixed base.
fn trunk_base() -> Vec<&'static str> {
    let mut base = vec![
        "GCI-1000", "GCI-1001", "GCI-1010", "GCI-1011", "GLG-1000",
        "MAT-1900", "GCI-2000", "IFT-1903", "MAT-1910", "GCI-1003",
        "GCI-1007", "GCI-2001", "GCI-2002", "GCI-2003", "GCI-1004",
        "GCI-2004", "GCI-2006", "GCI-2007", "GCI-3008", "GMC-3009",
        "STT-1900", "GCI-2008", "GCI-2009", "GCI-3000", "ECN-4901",
        "GCI-2010", "GCI-2011", "GCI-2012", "GCI-3009", "GCI-3333",
        "PHI-2910", "PHI-3900", "GMN-2901", "ANL-2020",
    ];
    base.sort_unstable();
    base
}

// A conformant fill : « Eau et environnement » chosen, its Règle 1 filled
// with exactly the four courses the Profil développement durable also
// needs (the répertoire's own note on that rule spells out the overlap :
// « vous devez suivre GCI-4201 et 1 cours parmi GCI-3101 ou GCI-4301 »),
// its Règle 2 with the profile's own GBO-2040, and DDU-1000 landing in
// the trunk's one free "any" rule (« Autres exigences – Règle 2 », 3 cr).
// The profile adds not one new credit : counted lands on the program's
// own 120, profile_only stays at zero.
#[test]
fn a_conformant_fill_adds_no_credit_and_shelters_nothing() {
    let program = b_gci();
    let courses = catalogue();
    let mut selection = trunk_base();
    // concentration Règle 1 (12 cr) : the four DD-profile courses it lists
    selection.extend(["GAE-3006", "GCI-4201", "GCI-3101", "GCI-4301"]);
    // concentration Règle 2 (3 cr) : the profile's own fifth course
    selection.push("GBO-2040");
    // profile mandatory, absorbed by the trunk's 3-cr free rule
    selection.push("DDU-1000");
    let summary = credit_summary(
        Some(&program),
        Some("Eau et environnement"),
        Some("Profil développement durable"),
        &codes(&selection),
        &courses,
    );
    assert!(summary.unknown.is_empty(), "{:?}", summary.unknown);
    assert_eq!(summary.counted, 120, "the program's own total, no more");
    assert_eq!(
        summary.profile_only, 0,
        "every profile credit was sheltered"
    );
}

// The report's actual 129/120 reproduced : with no concentration chosen
// yet — a real, reachable state (the profile can be filled before the
// concentration is) — none of the profile's option courses are shielded
// by a concentration rule's own list, so three of the four selected land
// in `profile_only` (only DDU-1000 fits the trunk's 3-cr free rule) : 9 cr
// the report showed as counted and should not have been. 15 cr of
// unrelated complementary courses stand in for what a concentration would
// otherwise have supplied, so the *raw* selection totals 129 cr —
// Antoine's own number — while `counted` still lands on 120.
//
// This also means the bug is narrower on real data than the plan assumed:
// every one of B-GCI's three concentrations resolves its own Règle 2
// through a reference to « Cheminement sans concentration — Règle 1 »,
// whose 25-course list happens to carry all five of the DD profile's
// option courses — so choosing *any* concentration already shelters them,
// leaving only DDU-1000's fixed 3 cr ever needing the free-rule mechanism
// (see the test above). Only the concentration-less state below reaches
// the plan's predicted `profile_only = 9`.
#[test]
fn without_a_concentration_the_profiles_own_courses_are_not_sheltered() {
    let program = b_gci();
    let courses = catalogue();
    let mut selection = trunk_base();
    // the profile, filled with its own courses : mandatory + 2 of 3 for
    // Règle 1 (6 cr) + 1 of 2 for Règle 2 (3 cr) — 12 cr, none listed
    // anywhere else while no concentration is chosen
    selection.extend(["DDU-1000", "GAE-3006", "GCI-4201", "GCI-3101"]);
    // 15 cr of unrelated complementary courses, standing in for what a
    // chosen concentration would otherwise have supplied
    selection
        .extend(["FOR-2020", "GBO-4015", "GBO-4070", "GCI-2101", "GCI-3003"]);
    let summary = credit_summary(
        Some(&program),
        None,
        Some("Profil développement durable"),
        &codes(&selection),
        &courses,
    );
    assert!(summary.unknown.is_empty(), "{:?}", summary.unknown);
    let raw: u32 = codes(&selection)
        .iter()
        .map(|code| {
            courses
                .iter()
                .find(|course| &course.code == code)
                .expect("every selected code is in the catalogue")
                .credits
                .planning()
        })
        .sum();
    assert_eq!(raw, 129, "Antoine's own number, unfiltered");
    assert_eq!(summary.counted, 120, "the filter brings it back to the cap");
    assert_eq!(
        summary.profile_only, 9,
        "GAE-3006, GCI-3101, GCI-4201 : DDU-1000 alone fit the free rule"
    );
}
