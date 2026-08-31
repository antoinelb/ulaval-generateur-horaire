// Scenario 7 — a `verify` answer that judged a plan the student has since
// outrun. `solve::verdict_settles` decides it in one line; what no unit
// test can show is the *sequence* that makes the line matter: ask, edit,
// then answer. Before the 2026-08-30 fix the late verdict cleared the
// stale flag, freezing « Placement vérifié ✓ » over a grid no solver had
// ever looked at — and `auto_verify`'s idle guard then refused to ask
// again, so the lie stayed until the next edit (TRU-3).

use ulaval_scheduler_ui::solve::QueryKind;

use crate::harness::{placed, Session};

// `auto_verify`'s own idle guard, quoted: this is what decides whether the
// screen will ever ask for a fresh verdict.
fn would_ask_again(session: &Session) -> bool {
    session.verification.is_none() || session.verification_stale
}

#[test]
fn a_verify_answer_asked_under_an_older_plan_never_settles() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    session.settle();

    // --- the control: nothing moves between the ask and the answer -------
    let fresh = session.ask(QueryKind::Verify);
    session.deliver(fresh);
    assert!(session.verification.is_some(), "the grid gets its verdict");
    assert!(!session.verification_stale, "and the verdict is fresh");
    assert!(
        !would_ask_again(&session),
        "a settled verdict is exactly what stops the automatic re-ask"
    );

    // --- the real sequence: the student edits while the solver works -----
    let in_flight = session.ask(QueryKind::Verify);
    let seat = placed(&session.plan, "GCI-2008");
    session.pin("GCI-2008", seat + 1);
    session.deliver(in_flight);

    assert!(
        session.verification.is_some(),
        "the last verdict stays on screen: emptying the panel is what made \
         the rules jump under the pointer (LAT-7)"
    );
    assert!(
        session.verification_stale,
        "but it judged a grid that is no longer the one on screen"
    );
    assert!(
        would_ask_again(&session),
        "and because it stayed marked stale, a fresh verdict is still owed"
    );

    // --- which the next answer supplies ----------------------------------
    session.verify();
    assert!(!session.verification_stale);
}

// The proposal's own version of the same race, and the one that cost a
// student his seats on 2026-08-26: a `place` answer solved before a relevé
// Capsule landed, adopted after it. The answer knows nothing of the
// courses the relevé pinned, so writing its placement as-is would drop
// them off the grid — and the very next request would then die on
// « BIO-1904 is passed or pinned but has no Course in the request »,
// taking the whole automatic placement with it. `displayed ⊇ pinned`,
// always.
#[test]
fn a_proposal_solved_before_an_import_never_evicts_what_it_pinned() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    session.settle();

    let in_flight = session.ask(QueryKind::Propose);
    let application = crate::transcript::application_of(&session);
    session.edit("Import Capsule", |plan| {
        ulaval_scheduler_ui::capsule::apply_to_plan(plan, &application);
    });
    let pinned = session.plan.pinned_sessions.clone();
    assert!(
        pinned.contains_key("GEX-1580"),
        "the relevé pinned courses the answer in flight never saw"
    );

    session.deliver(in_flight);
    for code in pinned.keys() {
        assert!(
            session.plan.displayed_placement.contains_key(code),
            "{code} was pinned by the import and left the grid"
        );
    }
    assert_eq!(
        session.plan.displayed_placement.get("GEX-1580"),
        pinned.get("GEX-1580"),
        "a course the answer does not carry at all keeps the seat the \
         relevé gave it"
    );
    // and the next request still resolves, which is what the eviction broke
    session.propose();
    assert_eq!(session.last_error, None);
}

// The same guard from the other side: an answer that arrives after the
// plan moved *and moved back* is still stale — the count is a generation,
// not a hash of the plan. A verdict is fresh because a solver looked at
// this grid, never because the grid happens to match.
#[test]
fn a_verdict_stays_stale_even_when_the_plan_edits_cancel_out() {
    let mut session = Session::empty();
    session.open("B-GEX", "A26");
    session.settle();
    let grid = session.plan.clone();

    let in_flight = session.ask(QueryKind::Verify);
    session.edit("Plafond porté à 15 crédits", |plan| plan.credit_cap = 15);
    session.undo();
    assert_eq!(session.plan, grid, "the document is back where it started");

    session.deliver(in_flight);
    assert!(
        session.verification_stale,
        "freshness is a fact about what was asked, not about what is shown"
    );
}
