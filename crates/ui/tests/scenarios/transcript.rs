// Scenarios 3 and 5 — a relevé Capsule pasted into an open plan.
//
// `capsule::load` and `capsule::apply_to_plan` are each proven alone. The
// sequence is what the student lives: the import anchors « Début » years
// back, freezes the graded sessions, grows the horizon — and then the
// solver has to build the rest of the path *on top of* that settled past.
// It is also one composite act, so it costs exactly one « Annuler »
// (AIR ACT-2).

use std::collections::BTreeSet;

use ulaval_scheduler_core::Semester;
use ulaval_scheduler_ui::{capsule, state};

use crate::harness::{semester, today, Session};

const TRANSCRIPT: &str = include_str!(
    "../../../../tests/fixtures/test_cases/transcripts/exemple.html"
);

fn loaded(session: &Session) -> capsule::CapsuleLoad {
    let known: BTreeSet<String> =
        session.snapshot.by_code.keys().cloned().collect();
    capsule::load(TRANSCRIPT, session.plan.study_sessions, &known)
        .unwrap_or_else(|error| panic!("{error}"))
}

// the same paste, for the scenarios of other modules
pub fn application_of(
    session: &Session,
) -> ulaval_scheduler_core::TranscriptApplication {
    loaded(session).application
}

#[test]
fn a_capsule_transcript_settles_the_past_and_the_solver_builds_on_it() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    let opened = session.plan.clone();
    assert_eq!(
        opened.start,
        today(),
        "a document nobody dated opens on the clock"
    );

    let load = loaded(&session);
    let application = load.application.clone();
    session.edit("Import Capsule", |plan| {
        capsule::apply_to_plan(plan, &application);
    });

    // --- what the relevé decided -----------------------------------------
    let a24: Semester = semester("A24");
    assert_eq!(session.plan.start, a24, "« Début » follows the relevé");
    assert!(
        state::semester_precedes(session.plan.start, today()),
        "and lands in the past, where no picker default ever puts it"
    );
    assert!(
        session.plan.study_sessions >= opened.study_sessions,
        "the horizon grows to hold the relevé, it never shrinks"
    );
    assert!(
        (1..=3).all(|graded| session.plan.frozen.contains(&graded)),
        "the graded sessions are settled: {:?}",
        session.plan.frozen
    );
    assert!(
        session.plan.summers_open,
        "the É25 of the relevé opens the étés"
    );
    assert!(
        session.plan.credit_cap >= 22,
        "a cap under a load the student really carried would make his own \
         past infeasible: {}",
        session.plan.credit_cap
    );
    assert!(
        session.plan.credited.contains("MAT-1910"),
        "RECONNAISSANCE DES ACQUIS credits without a session"
    );
    assert!(!session.plan.displayed_placement.contains_key("MAT-1910"));

    // --- the solver builds the rest on top of it -------------------------
    session.settle();
    assert_eq!(session.last_error, None);
    for (code, &index) in &application.pinned {
        assert_eq!(
            session.plan.displayed_placement.get(code),
            Some(&index),
            "{code} was lived in session {index}; the solver may not move it"
        );
    }
    assert!(
        !session.plan.displayed_placement.contains_key("MAT-1910"),
        "an acquired course never takes a seat"
    );
    // the acquis are assumed for what comes after: GCI-2012 lists GCI-2009,
    // and the relevé already holds GCI-2012 — everything still to place
    // sits strictly after the settled past
    let settled = *application
        .pinned
        .values()
        .max()
        .expect("the relevé pinned something");
    let still_to_place: Vec<&String> = session
        .plan
        .displayed_placement
        .iter()
        .filter(|(code, _)| !application.pinned.contains_key(*code))
        .filter(|(_, &index)| index <= 3)
        .map(|(code, _)| code)
        .collect();
    assert!(
        still_to_place.is_empty(),
        "the frozen sessions take no new course: {still_to_place:?}"
    );
    assert!(settled >= 3);
}

// Scenario 5 — ACT-2: an import is one act, whatever it touched.
#[test]
fn one_undo_reverses_a_whole_capsule_import() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    session.settle();
    let before = session.plan.clone();

    let application = loaded(&session).application;
    session.edit("Import Capsule", |plan| {
        capsule::apply_to_plan(plan, &application);
    });
    assert_ne!(
        session.plan, before,
        "the import really changed the document"
    );
    assert_eq!(session.history.undo_label(), Some("Import Capsule"));

    assert_eq!(session.undo().as_deref(), Some("Import Capsule"));
    assert_eq!(
        session.plan, before,
        "start, horizon, freezes, pins and credits all come back in one step"
    );
}

// The same act, followed by « Début » moved by hand: two acts, two undos —
// and the first of them must not have to be clicked twice either.
#[test]
fn moving_the_start_after_an_import_stays_one_undo_per_act() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    let application = loaded(&session).application;
    session.edit("Import Capsule", |plan| {
        capsule::apply_to_plan(plan, &application);
    });
    let imported = session.plan.clone();

    session.set_start("H25");
    assert_ne!(session.plan.start, imported.start);

    assert_eq!(session.undo().as_deref(), Some("Début déplacé à H25"));
    assert_eq!(session.plan, imported);
    assert_eq!(session.undo().as_deref(), Some("Import Capsule"));
    assert_eq!(session.plan.start, today());
}
