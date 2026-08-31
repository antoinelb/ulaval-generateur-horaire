// Scenario 4 — the shelf, walked both ways. `persist::leave_document` and
// `persist::enter_document` each decide their half alone; what this pins
// is that the pair is a round trip: a document put down and picked up
// again is the same document, placements, pins and settings included.
// The étudiante-cégep persona's whole session is this gesture, repeated.

use crate::harness::{placed, Session};

#[test]
fn switching_programs_and_back_restores_the_same_document() {
    let mut session = Session::empty();

    // --- the first document, worked on -----------------------------------
    session.open("B-GEX", "A26");
    session.settle();
    let seat = placed(&session.plan, "GCI-2008");
    session.pin("GCI-2008", seat + 1);
    session.edit("Plafond porté à 15 crédits", |plan| plan.credit_cap = 15);
    session.settle();
    let gex = session.plan.clone();
    assert_eq!(gex.credit_cap, 15);

    // --- « changer » : the picker, and nothing counted under no program ---
    session.leave();
    assert_eq!(session.plan.program, None);
    assert!(
        session.plan.displayed_placement.is_empty(),
        "the placements leave with the document"
    );
    assert!(
        session.storage.contains_key("gh.v1.plan/B-GEX-A26"),
        "the shelf write is synchronous, never behind the save debounce"
    );

    // --- a second document, also worked on -------------------------------
    session.open("B-GCI", "A26");
    assert_eq!(
        session.plan.credit_cap,
        ulaval_scheduler_ui::state::DEFAULT_CREDIT_CAP,
        "the cap is a fact of the document, not a setting carried across"
    );
    session.settle();
    session.edit("Étés ouverts", |plan| plan.summers_open = true);
    let gci = session.plan.clone();

    // --- back to the first -----------------------------------------------
    session.leave();
    session.open("B-GEX", "A26");
    assert_eq!(
        session.plan, gex,
        "the shelf hands back exactly the document that was put on it"
    );

    // --- and the second is still there too -------------------------------
    session.leave();
    session.open("B-GCI", "A26");
    assert_eq!(
        session.plan, gci,
        "one shelf slot per (programme, millésime)"
    );
}

// The document swap also drops every solver fact: a verdict computed on
// the document just left may not be read as describing the one entered.
#[test]
fn a_document_swap_leaves_no_verdict_behind() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    session.settle();
    session.verify();
    assert!(session.verification.is_some());
    assert!(!session.verification_stale);

    session.leave();
    assert_eq!(session.verification, None);
    assert!(session.left_out.is_empty());

    session.open("B-GCI", "A26");
    assert_eq!(
        session.verification, None,
        "the B-GCI document opens with no verdict of the B-GEX one"
    );
}
