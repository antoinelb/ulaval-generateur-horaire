// Scenario 2 — a shared link opened over an existing document, then
// edited. The codec round trip is proven in `persist`; what is proven here
// is the sequence `components::import_organigramme` builds around it: the
// recipient's own document goes to its shelf, the history restarts so one
// undo gives it back, the whole horizon arrives frozen — and once the
// recipient edits the imported grid, nothing keeps claiming to be what the
// sender sent.

use ulaval_scheduler_ui::{persist, present, state};

use crate::harness::{placed, Session};

#[test]
fn an_edited_shared_organigramme_stops_being_the_link_that_carried_it() {
    // --- the sender ------------------------------------------------------
    let mut sender = Session::empty();
    sender.open("B-GEX", "A26");
    sender.settle();
    let sent = sender.plan.clone();
    let payload = persist::encode_organigramme(&sent, &[]);

    // --- the recipient, who already has a document of his own ------------
    let mut session = Session::empty();
    session.open("B-GCI", "A26");
    session.settle();
    let own = session.plan.clone();

    let (shared, courses) = persist::decode_organigramme(&payload)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(courses.is_empty(), "no hand-entered course rode along");
    if let Some((key, encoded)) = persist::import_stash(&session.plan, &shared)
    {
        session.storage.insert(key, encoded);
    }
    session.history = state::History::default();
    session.edit("Organigramme partagé importé", |plan| {
        *plan = shared;
        plan.frozen = present::whole_horizon(plan);
    });

    assert_ne!(session.plan, own, "the link replaced his own document");
    assert_eq!(
        session.plan.displayed_placement, sent.displayed_placement,
        "the recipient sees the sender's grid, no adjustment"
    );
    assert_eq!(
        session.plan.frozen,
        present::whole_horizon(&session.plan),
        "it reopens frozen whole: the solver moves nothing in it"
    );
    assert_eq!(
        persist::encode_organigramme(&session.plan, &[]),
        payload,
        "re-sharing what was received hands on the very same link"
    );

    // --- the recipient makes it his own ----------------------------------
    session.edit("Tout dégeler", |plan| plan.frozen.clear());
    let moved = placed(&session.plan, "GCI-2008");
    session.pin("GCI-2008", moved + 1);

    let after = persist::encode_organigramme(&session.plan, &[]);
    assert_ne!(
        after, payload,
        "an edited organigramme no longer encodes as the one received"
    );
    let (reopened, _) = persist::decode_organigramme(&after)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        reopened.pinned_sessions.get("GCI-2008"),
        Some(&(moved + 1)),
        "the new link carries the recipient's own act"
    );
    // and the sender's link still says what the sender said
    let (original, _) = persist::decode_organigramme(&payload)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(original.displayed_placement, sent.displayed_placement);
    assert_eq!(
        original.pinned_sessions, sent.pinned_sessions,
        "editing a received organigramme never rewrites the link it came from"
    );
}

#[test]
fn one_undo_after_a_shared_import_gives_the_recipient_his_document_back() {
    let mut sender = Session::empty();
    sender.open("B-GEX", "A26");
    sender.settle();
    let payload = persist::encode_organigramme(&sender.plan, &[]);

    let mut session = Session::empty();
    session.open("B-GCI", "A26");
    session.settle();
    let own = session.plan.clone();

    let (shared, _) = persist::decode_organigramme(&payload)
        .unwrap_or_else(|error| panic!("{error}"));
    let stash = persist::import_stash(&session.plan, &shared);
    if let Some((key, encoded)) = stash.clone() {
        session.storage.insert(key, encoded);
    }
    session.history = state::History::default();
    session.edit("Organigramme partagé importé", |plan| {
        *plan = shared;
        plan.frozen = present::whole_horizon(plan);
    });

    // ACT-2: the import is one act, so it costs one undo
    assert_eq!(
        session.undo().as_deref(),
        Some("Organigramme partagé importé")
    );
    assert_eq!(session.plan, own, "his own document, whole");
    assert_eq!(session.undo(), None, "the history restarted at the import");

    // and the shelf holds it too, so « changer » then « Choisir » finds it
    let (key, _) = stash.expect("the link belongs to another program");
    assert_eq!(key, "gh.v1.plan/B-GCI-A26");
    session.leave();
    session.open("B-GCI", "A26");
    assert_eq!(session.plan, own, "the shelf answers as well as the undo");
}
