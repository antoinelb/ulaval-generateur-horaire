// Scenarios 6 and 8 — « Début », moved.
//
// Scenario 8 is the gesture the directeur du B-GCI could not make on
// 2026-08-30: without a start under the clock, no session of the horizon
// is in the past, and the only way the interface has to say « ce cours-là
// est fait » — placing it in a session already lived — is unreachable.
//
// Scenario 6 is the one he reported as a bug and which is not one: a
// « Début » moved away and back does not give the same cheminement. The
// solver minimises the distance to the *seed*, and the seed is the grid on
// screen when the ask is made, not the grid of an hour ago (ADR
// `2026-08-b-minimise-la-distance-au-seed`). This test documents that; it
// does not correct it. What it does hold to is that both grids are real
// solutions — the round trip may cost a different arrangement, never a
// false one.

use ulaval_scheduler_ui::{solve, state};

use crate::harness::{placed, semester, today, Session};

#[test]
fn a_start_in_the_past_lets_lived_sessions_feed_later_prerequisites() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");

    // the selector reaches eight years under the clock, which is what
    // makes the rest of this scenario reachable at all
    assert_eq!(
        state::start_year_window(session.plan.start, today()),
        (18, 31)
    );

    session.set_start("A22");
    assert_eq!(session.plan.start, semester("A22"));
    assert!(
        state::semester_precedes(
            solve::session_semester(&session.plan, 1)
                .expect("session 1 is on the horizon"),
            today(),
        ),
        "session 1 is a session the student has already lived"
    );

    session.settle();

    // --- the acquis of a lived session, stated by placing them there -----
    session.pin("GCI-1004", 1);
    assert_eq!(
        session.pin("GCI-2008", 1),
        Some(
            "GCI-2008 épinglé en A1-A22, mais préalable suivi la même \
             session sans concomitance permise : GCI-1004. Le placement est \
             conservé, mais le cheminement affiché ne tient plus : déplacez \
             GCI-2008 plus tard, placez le préalable plus tôt, ou cochez \
             « Permettre un préalable en concomitance »."
                .to_string()
        ),
        "the same session is not « before »"
    );
    assert_eq!(
        session.pin("GCI-2008", 2),
        None,
        "the session after it is: a course lived in A22 is acquired for H23"
    );

    session.settle();
    assert_eq!(session.last_error, None);
    assert_eq!(placed(&session.plan, "GCI-1004"), 1);
    assert_eq!(placed(&session.plan, "GCI-2008"), 2);
    assert!(
        placed(&session.plan, "GEX-1000") > 1,
        "GEX-1000 also lists GCI-1004: the solver reads the past pin as held"
    );
    assert!(
        session.left_out.is_empty(),
        "the whole bac still fits around the lived sessions: {:?}",
        session.left_out
    );

    // --- and a past start survives the reload ----------------------------
    session.save();
    assert_eq!(
        session.reloaded_plan().start,
        semester("A22"),
        "only the untouched factory default is re-dated onto the clock"
    );
}

// Deliberate, not a regression: the B-GCI grid after A26 → H27 → A26 is
// not the grid before it. Named so nobody « fixes » it by accident.
#[test]
fn a_start_round_trip_deliberately_returns_a_different_arrangement() {
    let mut session = Session::empty();
    session.open("B-GCI", "A26");
    session.settle();
    let reference = session.plan.displayed_placement.clone();
    let before = credits_per_session(&session);
    assert!(!reference.is_empty());

    session.set_start("H27");
    session.settle();
    session.set_start("A26");
    session.settle();

    assert_eq!(
        session.plan.start,
        semester("A26"),
        "the calendar came back"
    );
    assert_ne!(
        session.plan.displayed_placement, reference,
        "the seed of the second ask is the grid the détour left behind, not \
         the grid the first ask produced — ADR \
         `2026-08-b-minimise-la-distance-au-seed`"
    );
    let after = credits_per_session(&session);
    assert!(
        after
            .iter()
            .zip(&before)
            .any(|(now, then)| now + 5 <= *then),
        "and the student sees it as a session that empties — the directeur \
         du B-GCI read one hiver fall to 3 crédits. before {before:?}, \
         after {after:?}"
    );

    // what is *not* allowed to differ: the arrangement must still be one
    // the solver can prove
    session.verify();
    let verdict = session
        .verification
        .as_ref()
        .expect("the round-trip grid gets a verdict");
    assert_eq!(verdict.completion, "complete");
    assert_eq!(
        verdict.solutions.len(),
        1,
        "a different cheminement, never a false one"
    );
    assert!(session.left_out.is_empty());
}

fn credits_per_session(session: &Session) -> Vec<u32> {
    (1..=session.seasons().len())
        .map(|index| session.session_credits(index))
        .collect()
}
