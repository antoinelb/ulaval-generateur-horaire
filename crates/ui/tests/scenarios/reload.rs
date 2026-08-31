// Scenario 1 — the round trip a returning student makes every day:
// localStorage → restore → pin → solve → adopt → save → restore.
//
// Every one of those doors is proven alone. What no test held before is
// that the *chain* has a fixed point: the plan the app puts back on screen
// after a reload must be the plan it had before, byte for byte. Two of the
// three bugs the personas found on 2026-08-30 hid exactly there — a pin
// accepted without a word, and a grid that came back different from the
// one that was saved.

use ulaval_scheduler_ui::{persist, solve};

use crate::harness::{placed, Session};

// GCI-2008 lists GCI-1004 as a strict prerequisite; GEX-1000 lists it too.
const EARLY: &str = "GCI-1004";
const LATE: &str = "GCI-2008";

#[test]
fn a_pinned_and_solved_plan_comes_back_identical_after_a_reload() {
    // --- a first visit, settled and saved --------------------------------
    let mut first = Session::empty();
    first.open("B-GEX", "A26");
    first.settle();
    assert!(
        first.plan.displayed_placement.len() > 30,
        "the bac's own courses are on the grid: {}",
        first.plan.displayed_placement.len()
    );
    first.save();

    // --- the tab is reopened ---------------------------------------------
    let mut session = Session::boot(first.storage.clone());
    assert_eq!(
        session.plan, first.plan,
        "restoring a saved plan is a fixed point — nothing healed, nothing \
         evicted"
    );

    // --- the pin the student was never warned about ----------------------
    let early = placed(&session.plan, EARLY);
    let warning = session
        .pin(LATE, early)
        .expect("pinning a course in its own prerequisite's session warns");
    assert!(
        warning.contains(EARLY),
        "the warning names the préalable that is not held: {warning}"
    );
    // ACT-2: the pin is permitted and reversible, never silent
    assert_eq!(session.undo().as_deref(), Some("GCI-2008 épinglé"));
    assert_eq!(
        session.plan, first.plan,
        "the undo gives the grid back whole"
    );

    // --- the same pin, one session later, holds up -----------------------
    let late = placed(&session.plan, LATE);
    assert!(late > early, "the settled grid already orders the two");
    assert_eq!(
        session.pin(LATE, late),
        None,
        "a pin the prerequisites support says nothing"
    );

    // --- the solver answers, the pin survives it -------------------------
    session.settle();
    assert_eq!(
        session.plan.pinned_sessions.get(LATE),
        Some(&late),
        "an explicit act is sovereign"
    );
    assert_eq!(placed(&session.plan, LATE), late);
    assert!(
        session.left_out.is_empty(),
        "nothing floats: {:?}",
        session.left_out
    );

    // --- and the verdict settles on that very grid -----------------------
    session.verify();
    let verdict = session
        .verification
        .as_ref()
        .expect("a fully seated grid gets a verdict");
    assert_eq!(verdict.completion, "complete");
    assert_eq!(verdict.solutions.len(), 1, "the displayed grid is provable");
    assert!(!session.verification_stale, "nothing moved since the ask");

    // --- reload again: the plan is the same one --------------------------
    session.save();
    assert_eq!(
        session.reloaded_plan(),
        session.plan,
        "pin, proposal and save compose into a plan the restore reproduces"
    );
}

// A save written before the horizon rule existed: one seat past the last
// session of the grid. It made `verify` refuse the whole plan — « GEX-2001
// is pinned to session 11, outside 1..=9 » (2026-08-26) — so the student
// got no verdict at all, on every reload, with nothing on screen to
// explain it. The restore re-asserts the saved horizon, which evicts it.
#[test]
fn a_restored_plan_heals_a_seat_beyond_its_horizon() {
    let mut first = Session::empty();
    first.open("B-GEX", "A26");
    first.settle();

    // an automatic seat two years past the horizon — never a pin, which is
    // an explicit act and grows the horizon instead of being evicted
    let slots = first.seasons().len();
    let mut damaged = first.plan.clone();
    damaged
        .displayed_placement
        .insert("GEX-2001".to_string(), slots + 4);
    first.storage.insert(
        persist::PLAN_KEY.to_string(),
        persist::encode_plan(&damaged),
    );

    let mut session = Session::boot(first.storage.clone());
    assert!(
        session
            .plan
            .displayed_placement
            .values()
            .all(|&seat| seat <= slots),
        "the stray seat is gone: {:?}",
        session.plan.displayed_placement
    );
    assert_eq!(
        session.plan.study_sessions, damaged.study_sessions,
        "the horizon itself is the student's and does not move"
    );

    // the evicted seat falls back to « automatique », which is what arms
    // the automatic proposal — and the verdict the refusal used to swallow
    // comes back with it
    assert_eq!(
        solve::unplaced_codes(
            &session.snapshot,
            &session.plan,
            session.program().as_ref(),
        ),
        Ok(vec!["GEX-2001".to_string()]),
        "the panel says which course is floating, and the solver re-seats it"
    );
    session.settle();
    assert!(placed(&session.plan, "GEX-2001") <= slots);
    session.verify();
    assert_eq!(session.last_error, None, "the solver judges the plan again");
    assert!(session.verification.is_some());
}

// The same chain seen from the request: what the solver is asked must
// carry every course the grid draws. A save from before a fix could hold a
// placement with no elective entry, and the request builder is what heals
// it — a `PlacementError` here is the whole automatic placement going down.
#[test]
fn every_seat_of_a_restored_grid_rides_with_its_course_in_the_next_request() {
    let mut first = Session::empty();
    first.open("B-GEX", "A26");
    first.settle();
    first.save();

    let mut session = Session::boot(first.storage.clone());
    // a hand-added course, outside the program's own list
    session.edit("ANL-2020 ajouté", |plan| {
        plan.manual
            .entry(2)
            .or_default()
            .push("ANL-2020".to_string());
    });
    session.propose();
    assert_eq!(
        session.last_error, None,
        "the request resolves every code the plan carries"
    );
    assert_eq!(
        placed(&session.plan, "ANL-2020"),
        2,
        "a hand-added course is pinned where it was added"
    );
    assert!(solve::unplaced_codes(
        &session.snapshot,
        &session.plan,
        session.program().as_ref(),
    )
    .is_ok_and(|floating| floating.is_empty()));
}
