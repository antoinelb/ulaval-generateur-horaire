// The floating status stack (ALR-3/ALR-4/ALR-6) and everything it decides:
// which note replaces which, and what the student has already waved away.
// Pure — the view only renders `alerts()` and calls back in (AP-5).

use std::collections::BTreeMap;

use crate::present::UiError;

// The persistent status region's content (ALR-4: persists until
// dismissed; ALR-6: a reserved region, never a modal).
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub key: u64,
    pub body: AlertBody,
    // what the alert reports on — a caused alert also retires by itself
    // when its cause disappears (ADR
    // `2026-08-peremption-des-toasts-par-cause`)
    pub cause: AlertCause,
    // what the alert is *about*, when a later note on the same subject
    // must replace the previous one instead of stacking beside it. The
    // cause decides when a note expires; the topic decides what replaces
    // it and what a dismissal silences — two different questions, hence
    // two fields (ADR `2026-08-toasts-un-par-sujet-et-rejet-memorise`).
    pub topic: Option<AlertTopic>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum AlertCause {
    // lives until dismissed (ALR-4)
    #[default]
    Sticky,
    // this code could not be seated — stale once it sits somewhere or
    // leaves the plan
    LeftOut(String),
    // nothing could be placed at all — stale once anything is
    EmptyGrid,
    // the worker refused a query — stale the moment a later one answers:
    // the refusal described an input that no longer stands (rapport
    // étudiante-gex 2026-08-20, « ANL-1010 is pinned outside 1..=4 »
    // survivait au retour à 8 sessions)
    SolverError,
    // an announcement about the current document (injection, étés forcés,
    // acquis présumés…) — it leaves with it at a swap: « GMC-3020 en
    // été » under B-GIN named a course of another program (contre-test
    // étudiante-cegep 2026-08-20)
    Document,
}

// The subjects a proposal answer speaks about. Each one holds at most one
// note on screen: `apply_proposal` republishes all of them at every
// answer, and body-only deduplication let a wording that shifted by a
// single code stack a second banner on the same subject (rapport
// étudiante 2026-08-27, G3 : « présumés acquis » en double).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertTopic {
    Completion,
    EmptyGrid,
    LeftOut(String),
    SetAside(String),
    SummersForced,
    Assumed,
    Injected,
    ProposalKept,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlertBody {
    Note(String),
    // a confirmation, styled apart from the warnings (INP-3: the glyph
    // carries the difference, not the colour alone)
    Success(String),
    Error(UiError),
    // a destructive act with no confirmation dialog, undoable from the
    // toast itself (pattern ACT, plan item 9) — not a `Success`: it must
    // outlive the 5 s auto-clear or the undo would vanish with it, so it
    // carries the removed program instead of already-worded text. Boxed:
    // a whole `Program` inline would make every alert in the stack as
    // large as the rarest of them.
    LocalProgramRemoved(Box<crate::import::LocalProgram>),
}

// The stack itself. `next_key` never goes backwards: the Toasts
// auto-dismiss timers hold keys past an alert's death, and a recycled key
// would let a stale timer kill an unrelated fresh message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AlertStack {
    alerts: Vec<Alert>,
    // subject → the exact wording the student waved away. A *new* wording
    // on the same subject speaks again; the same one stays silent (F1).
    dismissed: BTreeMap<AlertTopic, AlertBody>,
    next_key: u64,
}

impl AlertStack {
    // what the restore had to tolerate, said at boot — sticky, and none of
    // it a proposal's subject
    pub fn seeded(notes: &[String]) -> Self {
        let mut stack = AlertStack::default();
        for note in notes {
            stack.push(AlertBody::Note(note.clone()), AlertCause::Sticky);
        }
        stack
    }

    pub fn alerts(&self) -> &[Alert] {
        &self.alerts
    }

    // Never the same message twice (ALR-3) — but a repeat is refreshed to
    // the front instead of swallowed: relaunching a search that ends on
    // the same verdict must still visibly answer (rapport 2026-08-14).
    pub fn push(&mut self, body: AlertBody, cause: AlertCause) {
        self.alerts.retain(|alert| alert.body != body);
        self.seat(body, cause, None);
    }

    // One note per subject: the previous one on the same subject goes,
    // and a subject the student dismissed stays silent until its wording
    // actually changes.
    pub fn push_topic(
        &mut self,
        body: AlertBody,
        cause: AlertCause,
        topic: AlertTopic,
    ) {
        self.alerts.retain(|alert| {
            alert.topic.as_ref() != Some(&topic) && alert.body != body
        });
        if self.dismissed.get(&topic) == Some(&body) {
            return;
        }
        self.dismissed.remove(&topic);
        self.seat(body, cause, Some(topic));
    }

    // The student's own ✕ (or click on the message): a topic waved away is
    // remembered with its exact wording, so the next answer that repeats it
    // word for word says nothing.
    pub fn dismiss(&mut self, key: u64) {
        let Some(index) = self.alerts.iter().position(|a| a.key == key) else {
            return;
        };
        let alert = self.alerts.remove(index);
        if let Some(topic) = alert.topic {
            self.dismissed.insert(topic, alert.body);
        }
    }

    // Expiry, not rejection: a cause that disappeared, a ✓ whose timer ran
    // out, an « Annuler » consumed. Never remembered — the student said
    // nothing, so the same note may speak again.
    pub fn retire(&mut self, retired: impl Fn(&Alert) -> bool) {
        self.alerts.retain(|alert| !retired(alert));
    }

    // A document swap: its announcements and its verdicts leave with it,
    // and so does the memory of what was dismissed about it — the next
    // document's notes are nobody's rejected ones. Only the Sticky alerts
    // (load warnings, confirmations) outlive it.
    pub fn purge_document(&mut self) {
        self.alerts
            .retain(|alert| alert.cause == AlertCause::Sticky);
        self.dismissed.clear();
    }

    fn seat(
        &mut self,
        body: AlertBody,
        cause: AlertCause,
        topic: Option<AlertTopic>,
    ) {
        let key = self.next_key;
        self.next_key = self.next_key.saturating_add(1);
        self.alerts.push(Alert {
            key,
            body,
            cause,
            topic,
        });
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn note(text: &str) -> AlertBody {
        AlertBody::Note(text.to_string())
    }

    fn bodies(stack: &AlertStack) -> Vec<AlertBody> {
        stack.alerts().iter().map(|a| a.body.clone()).collect()
    }

    #[test]
    fn a_topic_replaces_its_previous_note_instead_of_stacking() {
        let mut stack = AlertStack::default();
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        stack.push_topic(
            note("présume GEX-1000, GEX-2000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        assert_eq!(
            bodies(&stack),
            [note("présume GEX-1000, GEX-2000")],
            "one banner per subject, the latest wording"
        );
        // a different subject stands beside it
        stack.push_topic(
            note("étés forcés"),
            AlertCause::Document,
            AlertTopic::SummersForced,
        );
        assert_eq!(stack.alerts().len(), 2);
        // the codes are part of the subject: two left-out courses are two
        // subjects, not one replacing the other
        stack.push_topic(
            note("ANL-1010 sans place"),
            AlertCause::Document,
            AlertTopic::LeftOut("ANL-1010".to_string()),
        );
        stack.push_topic(
            note("GMN-2902 sans place"),
            AlertCause::Document,
            AlertTopic::LeftOut("GMN-2902".to_string()),
        );
        assert_eq!(stack.alerts().len(), 4);
    }

    #[test]
    fn the_same_body_is_never_shown_twice() {
        let mut stack = AlertStack::default();
        stack.push(note("même texte"), AlertCause::Sticky);
        stack.push(note("autre"), AlertCause::Sticky);
        stack.push(note("même texte"), AlertCause::Sticky);
        assert_eq!(
            bodies(&stack),
            [note("autre"), note("même texte")],
            "the repeat is refreshed to the front, never doubled"
        );
        // across the two doors too: a topic-tagged repeat of an untagged
        // note replaces it rather than doubling it
        stack.push_topic(
            note("autre"),
            AlertCause::Document,
            AlertTopic::Completion,
        );
        assert_eq!(bodies(&stack), [note("même texte"), note("autre")]);
    }

    #[test]
    fn a_dismissed_topic_stays_silent_until_its_body_changes() {
        let mut stack = AlertStack::default();
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        let key = stack.alerts()[0].key;
        stack.dismiss(key);
        assert!(stack.alerts().is_empty());
        // the very same wording, republished by the next answer: silent
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        assert!(stack.alerts().is_empty(), "the rejection holds");
        // a wording that actually changed is news again
        stack.push_topic(
            note("présume GEX-1000, GEX-2000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        assert_eq!(stack.alerts().len(), 1);
        // and the memory is cleared with it: the old wording speaks again
        let key = stack.alerts()[0].key;
        stack.retire(|alert| alert.key == key);
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        assert_eq!(stack.alerts().len(), 1);
        // dismissing a key nobody holds is a no-op, never a panic
        stack.dismiss(9_999);
        assert_eq!(stack.alerts().len(), 1);
        // an untagged alert dismissed leaves no memory to consult
        stack.push(note("sans sujet"), AlertCause::Sticky);
        let key = stack.alerts()[1].key;
        stack.dismiss(key);
        stack.push(note("sans sujet"), AlertCause::Sticky);
        assert_eq!(stack.alerts().len(), 2, "an untagged note always speaks");
    }

    #[test]
    fn retirement_is_not_a_dismissal() {
        let mut stack = AlertStack::default();
        stack.push_topic(
            note("ANL-1010 sans place"),
            AlertCause::LeftOut("ANL-1010".to_string()),
            AlertTopic::LeftOut("ANL-1010".to_string()),
        );
        // the course got a seat: its cause disappeared, nobody rejected it
        stack.retire(|alert| {
            alert.cause == AlertCause::LeftOut("ANL-1010".to_string())
        });
        assert!(stack.alerts().is_empty());
        stack.push_topic(
            note("ANL-1010 sans place"),
            AlertCause::LeftOut("ANL-1010".to_string()),
            AlertTopic::LeftOut("ANL-1010".to_string()),
        );
        assert_eq!(
            stack.alerts().len(),
            1,
            "it floats again — the warning must come back"
        );
    }

    #[test]
    fn dismissal_memory_dies_with_the_document() {
        let mut stack = AlertStack::default();
        stack.push(note("sauvegarde illisible"), AlertCause::Sticky);
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        stack.push_topic(
            note("GEX-2000 sans place"),
            AlertCause::LeftOut("GEX-2000".to_string()),
            AlertTopic::LeftOut("GEX-2000".to_string()),
        );
        let key = stack.alerts()[1].key;
        stack.dismiss(key);
        stack.purge_document();
        assert_eq!(
            bodies(&stack),
            [note("sauvegarde illisible")],
            "only the Sticky notes outlive the swap"
        );
        // the next document says its own thing, even word for word
        stack.push_topic(
            note("présume GEX-1000"),
            AlertCause::Document,
            AlertTopic::Assumed,
        );
        assert_eq!(stack.alerts().len(), 2);
    }

    #[test]
    fn keys_are_never_recycled() {
        let mut stack = AlertStack::default();
        stack.push(note("un"), AlertCause::Sticky);
        stack.push(note("deux"), AlertCause::Sticky);
        let first = stack.alerts()[0].key;
        let second = stack.alerts()[1].key;
        assert_ne!(first, second);
        stack.retire(|_| true);
        stack.push(note("trois"), AlertCause::Sticky);
        assert!(
            stack.alerts()[0].key > second,
            "a stale timer must never kill an unrelated fresh message"
        );
    }

    #[test]
    fn seeded_notes_are_sticky() {
        let stack = AlertStack::seeded(&[
            "Sauvegarde du plan illisible".to_string(),
            "Cours manuel masqué".to_string(),
        ]);
        assert_eq!(stack.alerts().len(), 2);
        assert!(stack
            .alerts()
            .iter()
            .all(|alert| alert.cause == AlertCause::Sticky
                && alert.topic.is_none()));
        // a swap leaves them exactly where they are
        let mut stack = stack;
        stack.purge_document();
        assert_eq!(stack.alerts().len(), 2);
    }

    // the derived impls the view leans on: every body and cause shape is
    // cloned, compared and printed at least once
    #[test]
    fn every_body_and_cause_clones_and_prints() {
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GEX","slug":"slug","semester":"A26","title":"T",
                "cycle":1,"credits_required":6,"mandatory":[],"rules":[],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let local = crate::import::LocalProgram {
            program,
            source_url: "https://exemple".to_string(),
            imported_at: "2026-08-27T12:00:00Z".to_string(),
            proxy: crate::import::PROXY_HOST.to_string(),
            anomalies: Vec::new(),
            origin: crate::import::ProgramOrigin::Url,
        };
        let bodies = [
            AlertBody::Note("n".to_string()),
            AlertBody::Success("s".to_string()),
            AlertBody::Error(UiError {
                what: "quoi".to_string(),
                reaction: "réaction".to_string(),
                affected: "quoi".to_string(),
                action: "quoi faire".to_string(),
                id: "ERR-1".to_string(),
                detail: "détail".to_string(),
            }),
            AlertBody::LocalProgramRemoved(Box::new(local)),
        ];
        let causes = [
            AlertCause::default(),
            AlertCause::LeftOut("GEX-1000".to_string()),
            AlertCause::EmptyGrid,
            AlertCause::SolverError,
            AlertCause::Document,
        ];
        let topics = [
            AlertTopic::Completion,
            AlertTopic::EmptyGrid,
            AlertTopic::LeftOut("GEX-1000".to_string()),
            AlertTopic::SetAside("GEX-1000".to_string()),
            AlertTopic::SummersForced,
            AlertTopic::Assumed,
            AlertTopic::Injected,
            AlertTopic::ProposalKept,
        ];
        let mut stack = AlertStack::default();
        for (body, cause) in bodies.iter().zip(causes.iter()) {
            stack.push(body.clone(), cause.clone());
        }
        for topic in topics.iter() {
            stack.push_topic(
                AlertBody::Note(format!("{topic:?}")),
                AlertCause::Document,
                topic.clone(),
            );
        }
        assert_eq!(AlertCause::default(), AlertCause::Sticky);
        assert!(format!("{:?}", stack.clone()).contains("AlertStack"));
        assert_eq!(stack.clone(), stack);
    }
}
